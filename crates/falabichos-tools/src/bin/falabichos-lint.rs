// SPDX-License-Identifier: Apache-2.0
//! `falabichos-lint`: validate all game content, art, music and UI strings.
//!
//! Usage: `cargo run -p falabichos-tools --bin falabichos-lint -- [--assets DIR] [--strict] [--review] [--missing-audio]`

use falabichos_assets::Assets;
use falabichos_content::Severity;
use falabichos_tools::lint;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |f: &str| args.iter().any(|a| a == f);
    if flag("--help") || flag("-h") {
        println!(
            "falabichos-lint [--assets DIR] [--strict] [--review] [--missing-audio] [--quiet]\n  --strict         warnings fail the run\n  --review         list items flagged needs_native_review\n  --missing-audio  list vocab ids without a recorded clip\n  --quiet          hide info messages"
        );
        return;
    }
    let assets = match args
        .iter()
        .position(|a| a == "--assets")
        .and_then(|i| args.get(i + 1))
    {
        Some(dir) => match Assets::from_dir(Path::new(dir)) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("cannot read {dir}: {e}");
                std::process::exit(2);
            }
        },
        None => {
            // Prefer the working tree so contributors lint what they edited, not what was compiled in.
            let local = Path::new("assets");
            if local.is_dir() {
                Assets::from_dir(local).unwrap_or_else(|_| Assets::embedded())
            } else {
                Assets::embedded()
            }
        }
    };
    let rep = lint(&assets);
    for d in &rep.diagnostics {
        if d.severity == Severity::Info && flag("--quiet") {
            continue;
        }
        println!("{d}");
    }
    if flag("--review") {
        println!("\nneeds native review ({}):", rep.review_list.len());
        for (k, id) in &rep.review_list {
            println!("  {k}: {id}");
        }
    }
    if flag("--missing-audio") {
        println!("\nvocab without audio ({}):", rep.missing_audio.len());
        for id in &rep.missing_audio {
            println!("  {id}");
        }
    }
    println!("\n{}", rep.summary);
    let (e, w, i) = (
        rep.count(Severity::Error),
        rep.count(Severity::Warning),
        rep.count(Severity::Info),
    );
    println!("{e} error(s), {w} warning(s), {i} info");
    if e > 0 || (flag("--strict") && w > 0) {
        std::process::exit(1);
    }
}
