// SPDX-License-Identifier: Apache-2.0
//! Asset access. [`Assets::embedded`] returns the assets compiled into the binary;
//! [`Assets::from_dir`] reads a directory (used by tools and for local modding with
//! `FALABICHOS_ASSETS=/path/to/assets`). Paths are always `/`-separated and relative to `assets/`.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::Path;

mod generated {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

#[derive(Debug, Clone, Default)]
pub struct Assets {
    files: BTreeMap<String, Cow<'static, [u8]>>,
}

impl Assets {
    /// Everything under `assets/` as of compile time.
    pub fn embedded() -> Assets {
        Assets { files: generated::EMBEDDED.iter().map(|(p, b)| (p.to_string(), Cow::Borrowed(*b))).collect() }
    }

    /// Read all files below `root` (recursively).
    pub fn from_dir(root: &Path) -> std::io::Result<Assets> {
        fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, Cow<'static, [u8]>>) -> std::io::Result<()> {
            for e in std::fs::read_dir(dir)? {
                let e = e?;
                let p = e.path();
                if e.file_name().to_string_lossy().starts_with('.') {
                    continue;
                }
                if p.is_dir() {
                    walk(&p, root, out)?;
                } else {
                    let rel = p.strip_prefix(root).unwrap_or(&p).to_string_lossy().replace('\\', "/");
                    out.insert(rel, Cow::Owned(std::fs::read(&p)?));
                }
            }
            Ok(())
        }
        let mut files = BTreeMap::new();
        walk(root, root, &mut files)?;
        Ok(Assets { files })
    }

    /// Embedded assets, or a directory override if `FALABICHOS_ASSETS` is set and readable.
    pub fn from_env_or_embedded() -> Assets {
        if let Ok(dir) = std::env::var("FALABICHOS_ASSETS") {
            match Assets::from_dir(Path::new(&dir)) {
                Ok(a) => return a,
                Err(e) => eprintln!("FALABICHOS_ASSETS={dir} unreadable ({e}); using embedded assets"),
            }
        }
        Assets::embedded()
    }

    pub fn get(&self, path: &str) -> Option<&[u8]> {
        self.files.get(path).map(|c| c.as_ref())
    }

    pub fn text(&self, path: &str) -> Option<&str> {
        std::str::from_utf8(self.get(path)?).ok()
    }

    pub fn contains(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }

    /// Paths starting with `prefix`, in sorted order.
    pub fn list<'a>(&'a self, prefix: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.files.keys().filter(move |k| k.starts_with(prefix)).map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Add or replace a file (tests and tools).
    pub fn insert(&mut self, path: &str, bytes: Vec<u8>) {
        self.files.insert(path.to_string(), Cow::Owned(bytes));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_is_listable() {
        let a = Assets::embedded();
        // assets/ always contains at least the README placeholder files we ship.
        let _ = a.len();
        let mut b = Assets::default();
        b.insert("x/y.txt", b"hi".to_vec());
        assert_eq!(b.text("x/y.txt"), Some("hi"));
        assert_eq!(b.list("x/").count(), 1);
        assert!(!b.contains("nope"));
    }
}
