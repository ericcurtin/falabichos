// SPDX-License-Identifier: Apache-2.0
//! UI string tables. A language file is plain text:
//!
//! ```text
//! # comment
//! menu.save = Save
//! battle.trust = Trust {value}/{max}
//! ```
//!
//! `{name}` placeholders are replaced by [`Strings::fmt`]. Missing keys fall back to English,
//! then to the key itself (so a missing translation is visible but never crashes).

use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct Strings {
    map: HashMap<String, String>,
    fallback: HashMap<String, String>,
}

pub fn parse_table(text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            map.insert(k.trim().to_string(), v.trim().replace("\\n", "\n"));
        }
    }
    map
}

impl Strings {
    pub fn new(lang_text: &str, fallback_text: &str) -> Strings {
        Strings { map: parse_table(lang_text), fallback: parse_table(fallback_text) }
    }

    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.map.get(key).or_else(|| self.fallback.get(key)).map(String::as_str).unwrap_or(key)
    }

    pub fn fmt(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut s = self.get(key).to_string();
        for (k, v) in args {
            s = s.replace(&format!("{{{k}}}"), v);
        }
        s
    }

    pub fn has(&self, key: &str) -> bool {
        self.map.contains_key(key) || self.fallback.contains_key(key)
    }

    /// Keys of the primary language (for linting translations against English).
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.map.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_fallback_and_format() {
        let s = Strings::new("a = Olá {name}\n# c\n", "a = Hello {name}\nb = Bye\nc = x\\ny");
        assert_eq!(s.fmt("a", &[("name", "Rui")]), "Olá Rui");
        assert_eq!(s.get("b"), "Bye");
        assert_eq!(s.get("zzz"), "zzz");
        assert_eq!(s.get("c"), "x\ny");
        assert!(s.has("b") && !s.has("zzz"));
    }
}
