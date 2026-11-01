// SPDX-License-Identifier: Apache-2.0
//! Embeds everything under `<repo>/assets/` into the binary, so the game needs no file access to
//! start (important on Android/iOS where assets live inside the app bundle).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.filter_map(Result::ok).collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if p.is_dir() {
            walk(&p, root, out);
        } else {
            let rel = p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            out.push((rel, p));
        }
    }
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest.join("../../assets");
    let root = root.canonicalize().unwrap_or(root);
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = vec![];
    walk(&root, &root, &mut files);
    let mut src = String::from("pub static EMBEDDED: &[(&str, &[u8])] = &[\n");
    for (rel, abs) in files {
        let _ = writeln!(
            src,
            "    ({:?}, include_bytes!({:?})),",
            rel,
            abs.to_string_lossy()
        );
    }
    src.push_str("];\n");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("embedded.rs");
    std::fs::write(out, src).unwrap();
}
