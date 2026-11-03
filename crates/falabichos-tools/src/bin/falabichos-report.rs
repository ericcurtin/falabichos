// SPDX-License-Identifier: Apache-2.0
//! `falabichos-report`: a teacher's progress report from a learner's save file (text, CSV or JSON).
//!
//! Usage: `falabichos-report [SAVE_FILE] [--format text|csv|json] [-o FILE] [--assets DIR] [--now UNIX_SECONDS]`
//!
//! Without `SAVE_FILE` it reads `save.ron` from the game's data directory (`FALABICHOS_DATA_DIR`
//! overrides it). Everything is computed locally.

use falabichos_assets::Assets;
use falabichos_tools::report::{Format, SECONDS_PER_DAY, build, render};
use std::path::{Path, PathBuf};

fn fail(msg: String) -> ! {
    eprintln!("{msg}");
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |f: &str| args.iter().position(|a| a == f).and_then(|i| args.get(i + 1));
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "falabichos-report [SAVE_FILE] [--format text|csv|json] [-o FILE] [--assets DIR] [--now UNIX_SECONDS]\n  SAVE_FILE   a save.ron (default: the game's data directory)\n  --format    text (default), csv (one row per word/exercise) or json\n  -o FILE     write to a file instead of standard output\n  --now       report as of this unix time (default: the time of the save); one day is {SECONDS_PER_DAY} s"
        );
        return;
    }
    let format = match value("--format") {
        Some(f) => {
            Format::parse(f).unwrap_or_else(|| fail(format!("unknown format `{f}` (text, csv or json)")))
        }
        None => Format::Text,
    };
    // the save file is the first argument that is not a flag or a flag's value
    let flags_with_value = ["--format", "-o", "--assets", "--now"];
    let mut positional: Option<&String> = None;
    let mut skip = false;
    for a in &args {
        if skip {
            skip = false;
        } else if flags_with_value.contains(&a.as_str()) {
            skip = true;
        } else if !a.starts_with('-') {
            positional = Some(a);
            break;
        }
    }
    let save_path: PathBuf = positional
        .map(PathBuf::from)
        .unwrap_or_else(|| falabichos_game::default_data_dir().join("save.ron"));
    let text = std::fs::read_to_string(&save_path)
        .unwrap_or_else(|e| fail(format!("cannot read {}: {e}", save_path.display())));
    let save = falabichos_core::save::load(&text)
        .unwrap_or_else(|e| fail(format!("{} is not a usable save: {e}", save_path.display())));
    let assets = match value("--assets") {
        Some(dir) => {
            Assets::from_dir(Path::new(dir)).unwrap_or_else(|e| fail(format!("cannot read {dir}: {e}")))
        }
        None => Assets::embedded(),
    };
    let content = falabichos_content::load_strict(&assets).unwrap_or_else(|errs| {
        fail(format!(
            "content is invalid:\n{}",
            errs.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n")
        ))
    });
    let now = match value("--now") {
        Some(n) => n
            .parse::<i64>()
            .unwrap_or_else(|_| fail(format!("--now needs unix seconds, got `{n}`"))),
        None => save.saved_at,
    };
    let out = render(&build(&content, &save, now), format);
    match value("-o") {
        Some(path) => {
            std::fs::write(path, out).unwrap_or_else(|e| fail(format!("cannot write {path}: {e}")));
            eprintln!("wrote {path}");
        }
        None => print!("{out}"),
    }
}
