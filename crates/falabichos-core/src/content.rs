// SPDX-License-Identifier: Apache-2.0
//! The in-memory content database.

use crate::model::*;
use crate::normalize::{fold, normalize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// A word-lookup result for the dialogue "tap a word" feature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordGloss {
    pub pt: String,
    pub en: String,
    pub vocab_id: Option<String>,
}

/// All game content, loaded and validated by `falabichos-content`.
#[derive(Debug, Clone, Default)]
pub struct Content {
    pub tiles: Vec<TileDef>,
    pub vocab: BTreeMap<String, VocabItem>,
    pub grammar: BTreeMap<String, GrammarConcept>,
    pub verbs: BTreeMap<String, Verb>,
    pub species: BTreeMap<String, Species>,
    pub towns: BTreeMap<String, Town>,
    pub maps: BTreeMap<String, MapDef>,
    pub dialogues: BTreeMap<String, Dialogue>,
    pub challenges: BTreeMap<String, Challenge>,
    pub seals: BTreeMap<String, Seal>,
    pub pron_tips: Vec<PronTip>,
    pub story: Option<Story>,
    /// Recorded pronunciation clips present in the assets (slot names, e.g. `cafe/bica`).
    pub audio_clips: BTreeSet<String>,
    tile_index: HashMap<char, usize>,
    word_index: HashMap<String, String>,
    exercise_index: HashMap<String, String>,
}

impl Content {
    /// Build lookup indexes. Call after all collections are filled.
    pub fn finalize(&mut self) {
        self.tile_index = self.tiles.iter().enumerate().map(|(i, t)| (t.ch, i)).collect();
        self.exercise_index.clear();
        for g in self.grammar.values() {
            for e in &g.exercises {
                self.exercise_index.insert(e.id.clone(), g.id.clone());
            }
        }
        // word index: exact normalised and folded forms -> vocab id. Longer headwords never shadow
        // single words because we index each individually and keep the first writer.
        let mut idx: HashMap<String, String> = HashMap::new();
        for v in self.vocab.values() {
            let mut forms: Vec<String> = vec![v.pt.clone()];
            forms.extend(v.forms.iter().cloned());
            if let Some(p) = &v.plural {
                forms.push(p.clone());
            }
            for f in forms {
                idx.entry(normalize(&f)).or_insert_with(|| v.id.clone());
                idx.entry(fold(&f)).or_insert_with(|| v.id.clone());
            }
        }
        self.word_index = idx;
    }

    /// Is there a recorded clip for this slot?
    pub fn has_clip(&self, slot: &str) -> bool {
        self.audio_clips.contains(slot)
    }

    pub fn tile(&self, ch: char) -> Option<&TileDef> {
        self.tile_index.get(&ch).and_then(|&i| self.tiles.get(i))
    }

    /// Look up a word (or the whole phrase) from dialogue text. `hand` is the line's own glossary.
    pub fn lookup_word(&self, word: &str, hand: &BTreeMap<String, String>) -> Option<WordGloss> {
        let n = normalize(word);
        if n.is_empty() {
            return None;
        }
        if let Some(en) = hand.get(&n).or_else(|| hand.get(word)) {
            return Some(WordGloss {
                pt: word.to_string(),
                en: en.clone(),
                vocab_id: None,
            });
        }
        let id = self
            .word_index
            .get(&n)
            .or_else(|| self.word_index.get(&fold(word)))?;
        let v = self.vocab.get(id)?;
        Some(WordGloss {
            pt: v.pt.clone(),
            en: v.en.join(" / "),
            vocab_id: Some(v.id.clone()),
        })
    }

    /// Skill ids (SRS keys) a species embodies.
    pub fn species_skills(&self, species: &str) -> Vec<String> {
        let Some(s) = self.species.get(species) else {
            return vec![];
        };
        let mut out: Vec<String> = s.vocab.clone();
        for g in &s.grammar {
            if let Some(g) = self.grammar.get(g) {
                out.extend(g.exercises.iter().map(|e| e.id.clone()));
            }
        }
        out
    }

    /// Skill ids for a challenge pool.
    pub fn pool_skills(&self, pool: &Pool) -> Vec<String> {
        let mut out: Vec<String> = vec![];
        for v in self.vocab.values() {
            if pool.categories.contains(&v.category) || pool.vocab.contains(&v.id) {
                out.push(v.id.clone());
            }
        }
        for g in &pool.grammar {
            if let Some(g) = self.grammar.get(g) {
                out.extend(g.exercises.iter().map(|e| e.id.clone()));
            }
        }
        out.sort();
        out.dedup();
        out
    }

    pub fn grammar_exercise(&self, skill: &str) -> Option<&ExerciseDef> {
        let g = self.grammar.get(self.exercise_index.get(skill)?)?;
        g.exercises.iter().find(|e| e.id == skill)
    }

    /// Skill id of one conjugation form: `conj_<verb>_<pp|im|fu>_<person index 0..5>`.
    pub fn conjugation_skill(verb: &str, tense: Tense, person: usize) -> String {
        format!("conj_{verb}_{}_{person}", tense.code())
    }

    /// Split a conjugation skill id back into (verb id, tense, person index). The verb must exist.
    pub fn parse_conjugation_skill(&self, skill: &str) -> Option<(&Verb, Tense, usize)> {
        let rest = skill.strip_prefix("conj_")?;
        let (rest, person) = rest.rsplit_once('_')?;
        let (verb, tense) = rest.rsplit_once('_')?;
        let person: usize = person.parse().ok()?;
        let verb = self.verbs.get(verb)?;
        let tense = Tense::from_code(tense)?;
        (person < PERSONS.len() && verb.forms(tense).len() == PERSONS.len()).then_some((verb, tense, person))
    }

    /// Every conjugation skill (verbs x 3 tenses x 5 persons), optionally only some verbs.
    pub fn conjugation_skills(&self, only: Option<&str>) -> Vec<String> {
        let mut out = vec![];
        for v in self.verbs.values().filter(|v| only.is_none_or(|o| o == v.id)) {
            for t in Tense::ALL {
                for p in 0..PERSONS.len() {
                    out.push(Self::conjugation_skill(&v.id, t, p));
                }
            }
        }
        out
    }

    /// The exercise for a conjugation skill: type the form. Both `falaste` and `tu falaste` are accepted.
    pub fn conjugation_exercise(&self, skill: &str) -> Option<ExerciseDef> {
        let (v, t, p) = self.parse_conjugation_skill(skill)?;
        let form = v.forms(t)[p].clone();
        let pronouns: &[&str] = match p {
            2 => &["ele", "ela"],
            4 => &["eles", "elas"],
            _ => &[PERSONS[p]],
        };
        let mut answer = vec![form.clone()];
        answer.extend(pronouns.iter().map(|pr| format!("{pr} {form}")));
        let row: Vec<String> = v.forms(t).to_vec();
        Some(ExerciseDef {
            id: skill.to_string(),
            kind: ExerciseKind::Type,
            prompt: format!("{} ({}): {} ___", v.inf, t.pt(), PERSONS[p]),
            prompt_lang: Lang::Pt,
            context: Some(format!("{}: {}", v.en, t.en())),
            options: vec![],
            answer,
            hint: None,
            word_hints: vec![],
            explanation: Some(format!("{}, {}: {}", v.inf, t.pt(), row.join(", "))),
            needs_native_review: v.needs_native_review,
        })
    }

    pub fn is_grammar_skill(&self, skill: &str) -> bool {
        self.exercise_index.contains_key(skill)
    }

    pub fn pron_tip(&self, tag: PronTag) -> Option<&PronTip> {
        self.pron_tips.iter().find(|t| t.tag == tag)
    }
}
