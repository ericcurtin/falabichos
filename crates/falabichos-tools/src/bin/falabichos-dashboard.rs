// SPDX-License-Identifier: Apache-2.0
//! `falabichos-dashboard`: render the content linter's findings and coverage tables as one static HTML file.
//!
//! Usage: `cargo run -p falabichos-tools --bin falabichos-dashboard -- [--assets DIR] [-o out/dashboard.html]`
//!
//! The page has no scripts and makes no requests; open it from disk or publish it anywhere. It is a
//! contributor tool and is not part of the game.

use falabichos_assets::Assets;
use falabichos_tools::dashboard;
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |f: &str| args.iter().position(|a| a == f).and_then(|i| args.get(i + 1));
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "falabichos-dashboard [--assets DIR] [-o FILE]\n  --assets DIR   content directory (default: ./assets if it exists, else the embedded copy)\n  -o FILE        output file (default: dashboard.html)"
        );
        return;
    }
    let assets = match value("--assets") {
        Some(dir) => Assets::from_dir(Path::new(dir)).unwrap_or_else(|e| {
            eprintln!("cannot read {dir}: {e}");
            std::process::exit(2);
        }),
        None => {
            let local = Path::new("assets");
            if local.is_dir() {
                Assets::from_dir(local).unwrap_or_else(|_| Assets::embedded())
            } else {
                Assets::embedded()
            }
        }
    };
    let out: PathBuf = value("-o")
        .map(PathBuf::from)
        .unwrap_or_else(|| "dashboard.html".into());
    if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty())
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        eprintln!("cannot create {}: {e}", parent.display());
        std::process::exit(2);
    }
    let html = dashboard::render(&assets);
    if let Err(e) = std::fs::write(&out, html) {
        eprintln!("cannot write {}: {e}", out.display());
        std::process::exit(2);
    }
    println!("wrote {}", out.display());
}
