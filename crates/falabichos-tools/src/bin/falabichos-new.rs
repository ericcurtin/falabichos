// SPDX-License-Identifier: Apache-2.0
//! `falabichos-new <vocab|grammar|bicho|town> <id>`: scaffold a new content file with comments.
//! Refuses to overwrite existing files.

use falabichos_tools::scaffold::files_for;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        eprintln!(
            "usage: falabichos-new <vocab|grammar|bicho|town> <id> [--assets DIR]\n  vocab <category>   new vocabulary file\n  grammar <id>       new grammar concept\n  bicho <id>         new Bicho species + 16x16 art stub\n  town <id>          new town with a starter map and dialogue"
        );
        std::process::exit(2);
    }
    let root = args
        .iter()
        .position(|a| a == "--assets")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| "assets".into());
    let files = match files_for(&args[0], &args[1]) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    };
    for (rel, _) in &files {
        if Path::new(&root).join(rel).exists() {
            eprintln!("error: {rel} already exists; not overwriting");
            std::process::exit(1);
        }
    }
    for (rel, body) in files {
        let p = Path::new(&root).join(&rel);
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir).expect("create directory");
        }
        std::fs::write(&p, body).expect("write file");
        println!("created {}", p.display());
    }
    println!("next: edit the files, then run `cargo run -p falabichos-tools --bin falabichos-lint`");
}
