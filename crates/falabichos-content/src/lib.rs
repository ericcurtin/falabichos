// SPDX-License-Identifier: Apache-2.0
//! Loads the RON content files from [`Assets`], builds a [`Content`], and validates it.
//!
//! File layout (all paths relative to `assets/content/`):
//!
//! | path | contains |
//! |------|----------|
//! | `tiles.ron` | list of `TileDef` |
//! | `story.ron` | one `Story` |
//! | `pronunciation.ron` | list of `PronTip` |
//! | `seals.ron` | list of `Seal` |
//! | `vocab/*.ron` | list of `VocabItem` |
//! | `grammar/*.ron` | one `GrammarConcept` per file |
//! | `bichos/*.ron` | one `Species` per file |
//! | `towns/<town>/town.ron` | one `Town` |
//! | `towns/<town>/maps/*.ron` | one `MapDef` per file |
//! | `towns/<town>/dialogues/*.ron` | list of `Dialogue` |
//! | `towns/<town>/challenges/*.ron` | list of `Challenge` |

mod validate;

use falabichos_assets::Assets;
use falabichos_core::Content;
use falabichos_core::model::*;
use ron::extensions::Extensions;
use serde::de::DeserializeOwned;
use std::fmt;

pub use validate::validate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    /// File path or `kind:id` the message is about.
    pub location: String,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tag = match self.severity {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        };
        write!(f, "{tag}: {}: {}", self.location, self.message)
    }
}

pub(crate) fn diag(
    severity: Severity,
    location: impl Into<String>,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        severity,
        location: location.into(),
        message: message.into(),
    }
}

/// Result of loading: a (possibly partial) content database plus everything that went wrong.
pub struct Loaded {
    pub content: Content,
    pub diagnostics: Vec<Diagnostic>,
}

impl Loaded {
    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter().filter(|d| d.severity == Severity::Error)
    }
    pub fn has_errors(&self) -> bool {
        self.errors().next().is_some()
    }
}

fn ron_options() -> ron::Options {
    ron::Options::default().with_default_extension(Extensions::IMPLICIT_SOME)
}

fn parse<T: DeserializeOwned>(path: &str, text: &str, diags: &mut Vec<Diagnostic>) -> Option<T> {
    match ron_options().from_str::<T>(text) {
        Ok(v) => Some(v),
        Err(e) => {
            diags.push(diag(
                Severity::Error,
                path,
                format!("parse error at {}: {}", e.span.start, e.code),
            ));
            None
        }
    }
}

fn insert_unique<T>(
    map: &mut std::collections::BTreeMap<String, T>,
    id: String,
    value: T,
    kind: &str,
    path: &str,
    diags: &mut Vec<Diagnostic>,
) {
    use std::collections::btree_map::Entry;
    match map.entry(id) {
        Entry::Occupied(e) => diags.push(diag(
            Severity::Error,
            path,
            format!("duplicate {kind} id `{}`", e.key()),
        )),
        Entry::Vacant(e) => {
            e.insert(value);
        }
    }
}

/// Load and validate all content.
pub fn load(assets: &Assets) -> Loaded {
    let mut content = Content::default();
    let mut d: Vec<Diagnostic> = vec![];
    let prefix = "content/";
    let paths: Vec<String> = assets
        .list(prefix)
        .filter(|p| p.ends_with(".ron"))
        .map(String::from)
        .collect();
    for path in &paths {
        let rel = &path[prefix.len()..];
        let Some(text) = assets.text(path) else {
            d.push(diag(Severity::Error, path, "not valid UTF-8"));
            continue;
        };
        let parts: Vec<&str> = rel.split('/').collect();
        match parts.as_slice() {
            ["tiles.ron"] => {
                if let Some(v) = parse::<Vec<TileDef>>(path, text, &mut d) {
                    content.tiles = v;
                }
            }
            ["story.ron"] => content.story = parse::<Story>(path, text, &mut d),
            ["pronunciation.ron"] => {
                if let Some(v) = parse::<Vec<PronTip>>(path, text, &mut d) {
                    content.pron_tips = v;
                }
            }
            ["seals.ron"] => {
                for s in parse::<Vec<Seal>>(path, text, &mut d).unwrap_or_default() {
                    insert_unique(&mut content.seals, s.id.clone(), s, "seal", path, &mut d);
                }
            }
            ["vocab", _] => {
                for v in parse::<Vec<VocabItem>>(path, text, &mut d).unwrap_or_default() {
                    insert_unique(&mut content.vocab, v.id.clone(), v, "vocab", path, &mut d);
                }
            }
            ["grammar", _] => {
                if let Some(g) = parse::<GrammarConcept>(path, text, &mut d) {
                    insert_unique(&mut content.grammar, g.id.clone(), g, "grammar", path, &mut d);
                }
            }
            ["bichos", _] => {
                if let Some(s) = parse::<Species>(path, text, &mut d) {
                    insert_unique(&mut content.species, s.id.clone(), s, "species", path, &mut d);
                }
            }
            ["towns", _, "town.ron"] => {
                if let Some(t) = parse::<Town>(path, text, &mut d) {
                    insert_unique(&mut content.towns, t.id.clone(), t, "town", path, &mut d);
                }
            }
            ["towns", _, "maps", _] => {
                if let Some(m) = parse::<MapDef>(path, text, &mut d) {
                    insert_unique(&mut content.maps, m.id.clone(), m, "map", path, &mut d);
                }
            }
            ["towns", _, "dialogues", _] => {
                for x in parse::<Vec<Dialogue>>(path, text, &mut d).unwrap_or_default() {
                    insert_unique(&mut content.dialogues, x.id.clone(), x, "dialogue", path, &mut d);
                }
            }
            ["towns", _, "challenges", _] => {
                for x in parse::<Vec<Challenge>>(path, text, &mut d).unwrap_or_default() {
                    insert_unique(
                        &mut content.challenges,
                        x.id.clone(),
                        x,
                        "challenge",
                        path,
                        &mut d,
                    );
                }
            }
            _ => d.push(diag(
                Severity::Warning,
                path,
                "unrecognised content file location (ignored)",
            )),
        }
    }
    content.audio_clips = assets
        .list("audio/pt-PT/")
        .filter_map(|p| p.strip_prefix("audio/pt-PT/")?.strip_suffix(".ogg"))
        .map(String::from)
        .collect();
    content.finalize();
    d.extend(validate(&content));
    Loaded {
        content,
        diagnostics: d,
    }
}

/// Load, failing with the list of errors if anything is wrong. Used by the game at startup.
pub fn load_strict(assets: &Assets) -> Result<Content, Vec<Diagnostic>> {
    let l = load(assets);
    if l.has_errors() {
        Err(l
            .diagnostics
            .into_iter()
            .filter(|d| d.severity == Severity::Error)
            .collect())
    } else {
        Ok(l.content)
    }
}

/// Every content entry flagged `needs_native_review`, as `(kind, id)`.
pub fn review_items(c: &Content) -> Vec<(&'static str, String)> {
    let mut v = vec![];
    v.extend(
        c.vocab
            .values()
            .filter(|x| x.needs_native_review)
            .map(|x| ("vocab", x.id.clone())),
    );
    v.extend(
        c.grammar
            .values()
            .filter(|x| x.needs_native_review)
            .map(|x| ("grammar", x.id.clone())),
    );
    for g in c.grammar.values() {
        v.extend(
            g.exercises
                .iter()
                .filter(|e| e.needs_native_review)
                .map(|e| ("exercise", e.id.clone())),
        );
    }
    v.extend(
        c.species
            .values()
            .filter(|x| x.needs_native_review)
            .map(|x| ("species", x.id.clone())),
    );
    v.extend(
        c.towns
            .values()
            .filter(|x| x.needs_native_review)
            .map(|x| ("town", x.id.clone())),
    );
    for dlg in c.dialogues.values() {
        for (bi, b) in dlg.branches.iter().enumerate() {
            for (li, l) in b.lines.iter().enumerate() {
                if l.needs_native_review {
                    v.push(("line", format!("{}#{}.{}", dlg.id, bi, li)));
                }
            }
        }
    }
    v
}

/// Embedded content, validated; panics with the error list if the shipped data is broken
/// (the lint and CI prevent that).
pub fn embedded_content() -> Content {
    match load_strict(&Assets::embedded()) {
        Ok(c) => c,
        Err(errs) => {
            let msg: Vec<String> = errs.iter().map(|e| e.to_string()).collect();
            panic!("embedded content is invalid:\n{}", msg.join("\n"))
        }
    }
}
