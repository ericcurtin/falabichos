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
//! | `verbs/*.ron` | list of `Verb` (conjugation tables) |
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
            ["verbs", _] => {
                for v in parse::<Vec<Verb>>(path, text, &mut d).unwrap_or_default() {
                    insert_unique(&mut content.verbs, v.id.clone(), v, "verb", path, &mut d);
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
        c.verbs
            .values()
            .filter(|x| x.needs_native_review)
            .map(|x| ("verb", x.id.clone())),
    );
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

#[cfg(test)]
mod tests {
    use super::*;
    use falabichos_core::Rng;
    use std::collections::BTreeSet;

    fn shipped() -> Content {
        embedded_content()
    }

    /// Every species can actually be met: it appears in some map's encounter table or is gifted.
    #[test]
    fn every_species_can_be_met_in_the_world() {
        let c = shipped();
        let mut met: BTreeSet<String> = BTreeSet::new();
        for m in c.maps.values() {
            met.extend(m.encounters.iter().map(|e| e.species.clone()));
        }
        for d in c.dialogues.values() {
            for b in &d.branches {
                for a in &b.then {
                    if let Action::GiveBicho(s) = a {
                        met.insert(s.clone());
                    }
                }
            }
        }
        for s in c.species.keys() {
            assert!(met.contains(s), "species `{s}` is never encountered or gifted");
        }
    }

    /// Every grammar concept is attached to a grammar Bicho, so players meet it in battles.
    #[test]
    fn every_grammar_concept_belongs_to_a_bicho() {
        let c = shipped();
        let used: BTreeSet<&String> = c.species.values().flat_map(|s| s.grammar.iter()).collect();
        for g in c.grammar.keys() {
            assert!(
                used.contains(g),
                "grammar concept `{g}` is not taught by any Bicho"
            );
        }
    }

    /// Every Mestre awards a Selo that exists, and every Selo is awarded by exactly one Mestre.
    #[test]
    fn mestres_and_seals_match() {
        let c = shipped();
        let mut awarded = BTreeSet::new();
        for ch in c.challenges.values().filter(|c| c.kind == ChallengeKind::Mestre) {
            let seal = ch
                .seal
                .as_ref()
                .unwrap_or_else(|| panic!("{} has no seal", ch.id));
            assert!(c.seals.contains_key(seal), "{} awards unknown seal {seal}", ch.id);
            assert!(awarded.insert(seal.clone()), "seal {seal} is awarded twice");
        }
        for s in c.seals.keys() {
            assert!(awarded.contains(s), "seal `{s}` is never awarded");
        }
    }

    /// The conjugation tables are right for European Portuguese (spot checks) and drive a working drill.
    #[test]
    fn conjugation_tables_and_drill_work() {
        use falabichos_core::battle::{Battle, BattleKind};
        use falabichos_core::exercise::{Answer, ExKind, generate};
        use falabichos_core::normalize::AnswerPolicy;
        use falabichos_core::srs::SrsStore;
        let c = shipped();
        assert!(c.verbs.len() >= 10);
        let forms = |v: &str, t: Tense| c.verbs[v].forms(t).join(" ");
        // pt-PT: the nós form of the pretérito perfeito of -ar verbs has an accent
        assert_eq!(
            forms("falar", Tense::Perfeito),
            "falei falaste falou falámos falaram"
        );
        assert_eq!(
            forms("comer", Tense::Perfeito),
            "comi comeste comeu comemos comeram"
        );
        assert_eq!(forms("ser", Tense::Perfeito), forms("ir", Tense::Perfeito));
        assert_eq!(forms("ser", Tense::Imperfeito), "era eras era éramos eram");
        assert_eq!(forms("ir", Tense::Imperfeito), "ia ias ia íamos iam");
        assert_eq!(
            forms("ter", Tense::Imperfeito),
            "tinha tinhas tinha tínhamos tinham"
        );
        assert_eq!(forms("fazer", Tense::Futuro), "farei farás fará faremos farão");
        assert_eq!(
            forms("partir", Tense::Futuro),
            "partirei partirás partirá partiremos partirão"
        );
        // regular verbs follow their group's endings
        let endings: [(&str, Tense, [&str; 5]); 6] = [
            ("ar", Tense::Perfeito, ["ei", "aste", "ou", "ámos", "aram"]),
            ("er", Tense::Perfeito, ["i", "este", "eu", "emos", "eram"]),
            ("ir", Tense::Perfeito, ["i", "iste", "iu", "imos", "iram"]),
            ("ar", Tense::Imperfeito, ["ava", "avas", "ava", "ávamos", "avam"]),
            ("er", Tense::Imperfeito, ["ia", "ias", "ia", "íamos", "iam"]),
            ("ar", Tense::Futuro, ["arei", "arás", "ará", "aremos", "arão"]),
        ];
        for v in c.verbs.values().filter(|v| v.group != "irregular") {
            for (g, t, ends) in &endings {
                if v.group == *g {
                    let stem = v.inf.trim_end_matches(g);
                    for (form, end) in v.forms(*t).iter().zip(ends) {
                        assert!(form.ends_with(end), "{}: {form} should end in -{end}", v.id);
                        if *t != Tense::Futuro {
                            assert!(form.starts_with(stem), "{}: {form}", v.id);
                        }
                    }
                }
            }
        }
        // skill ids round trip and every one generates a typed exercise whose answer checks out
        let skills = c.conjugation_skills(None);
        assert_eq!(skills.len(), c.verbs.len() * 15);
        let mut rng = Rng::new(7);
        for s in &skills {
            let (v, t, p) = c.parse_conjugation_skill(s).expect("parses");
            assert_eq!(&Content::conjugation_skill(&v.id, t, p), s);
            let ex = generate(&c, s, None, &mut rng).expect("generates");
            assert_eq!(ex.kind, ExKind::Type);
            let form = &v.forms(t)[p];
            for ok in [
                form.clone(),
                format!("{} {form}", PERSONS[p].split('/').next().unwrap()),
            ] {
                assert!(
                    ex.check(&Answer::Text(ok), AnswerPolicy::default()).correct,
                    "{s}"
                );
            }
            assert!(
                !ex.check(&Answer::Text("zzz".into()), AnswerPolicy::default())
                    .correct
            );
            assert!(ex.explanation.as_deref().unwrap().contains(form));
        }
        assert!(c.parse_conjugation_skill("conj_nope_pp_0").is_none());
        assert!(c.parse_conjugation_skill("conj_falar_xx_0").is_none());
        assert!(c.parse_conjugation_skill("conj_falar_pp_9").is_none());
        // a whole drill can be played to the end by answering correctly
        let srs = SrsStore::default();
        let mut b =
            Battle::conjugation(&c, &srs, AnswerPolicy::default(), Some("falar"), 6, &mut rng, 0).unwrap();
        assert!(matches!(b.kind, BattleKind::Practice));
        let mut store = SrsStore::default();
        let mut asked = 0;
        while !b.is_over() && asked < 20 {
            let ex = b.current.clone().expect("a question");
            assert!(
                ex.skill.starts_with("conj_falar_"),
                "limited to one verb: {}",
                ex.skill
            );
            let right = ex.accepted[0].clone();
            b.answer(&Answer::Text(right), &mut store, 100 + asked).unwrap();
            b.advance(&c, &store, &mut rng, 100 + asked);
            asked += 1;
        }
        assert!(b.is_over());
        assert_eq!(store.cards.len(), 6, "each answered form gets an SRS card");
    }

    /// Part 2, town 3 (Ribeira Dourada): the content promised by the roadmap is really there.
    #[test]
    fn ribeira_dourada_is_complete() {
        let c = shipped();
        assert!(c.towns.contains_key("ribeira_dourada"));
        let maps: Vec<&MapDef> = c
            .maps
            .values()
            .filter(|m| m.town.as_deref() == Some("ribeira_dourada"))
            .collect();
        assert!(
            maps.len() >= 5,
            "town 3 needs an outdoor map, a route and interiors"
        );
        let mestre = &c.challenges["mestre_duarte"];
        assert_eq!(mestre.kind, ChallengeKind::Mestre);
        assert_eq!(mestre.seal.as_deref(), Some("selo_ponte"));
        let trainers = c
            .challenges
            .values()
            .filter(|ch| ch.kind == ChallengeKind::Trainer && ch.id != "treino_hugo")
            .filter(|ch| {
                maps.iter()
                    .flat_map(|m| m.npcs.iter())
                    .any(|n| n.trainer.as_ref().is_some_and(|t| t.challenge == ch.id))
            })
            .count();
        assert!(
            trainers >= 3,
            "town 3 needs at least three trainers, found {trainers}"
        );
        // New Bichos, with unique Bichodex numbers.
        for (id, dex) in [("rabelo", 10), ("bussola", 11), ("pombo", 12), ("coruja", 13)] {
            assert_eq!(c.species[id].dex_no, dex);
        }
        // Vocabulary: directions and the city, each word fully annotated.
        for cat in ["directions", "city"] {
            let items: Vec<&VocabItem> = c.vocab.values().filter(|v| v.category == cat).collect();
            assert!(items.len() >= 12, "category {cat} has only {} words", items.len());
            for v in items {
                assert!(v.ipa.is_some(), "{} lacks ipa", v.id);
                assert!(v.example.is_some(), "{} lacks an example", v.id);
                assert!(
                    v.needs_native_review,
                    "{} must stay flagged for native review",
                    v.id
                );
            }
        }
        // Grammar: ter/haver and the directions concepts, with real exercise sets.
        for g in ["ter_haver", "directions_prep", "imperative_directions"] {
            let concept = &c.grammar[g];
            assert!(concept.exercises.len() >= 8, "{g} needs 8+ exercises");
            assert!(concept.exercises.iter().all(|e| e.explanation.is_some()));
            assert!(concept.needs_native_review);
        }
    }
}
