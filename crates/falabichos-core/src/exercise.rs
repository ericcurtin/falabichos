// SPDX-License-Identifier: Apache-2.0
//! Exercises: generation from vocab / grammar content, and answer checking.

use crate::content::Content;
use crate::model::*;
use crate::normalize::{AnswerPolicy, Verdict, check_text, fold};
use crate::rng::Rng;
use crate::srs::{Card, Phase};

/// How the player answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExKind {
    Choice,
    Type,
    Order,
}

/// The pedagogical flavour; the UI uses it to pick the instruction text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    PtToEn,
    EnToPt,
    Article,
    /// Play the clip, choose the word.
    Listen,
    /// No clip yet: read the IPA, choose the word.
    ListenIpa,
    Grammar,
    Sentence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Exercise {
    /// SRS key: vocab id or grammar exercise id.
    pub skill: String,
    pub kind: ExKind,
    pub flavor: Flavor,
    pub prompt: String,
    pub prompt_lang: Lang,
    pub context: Option<String>,
    /// `Choice`: the options. `Order`: the shuffled word tiles.
    pub options: Vec<String>,
    /// `Choice`: indices of correct options.
    pub correct: Vec<usize>,
    /// `Type` / `Order`: accepted answers (first is canonical).
    pub accepted: Vec<String>,
    pub hint: Option<String>,
    pub explanation: Option<String>,
    pub audio: Option<String>,
    pub ipa: Option<String>,
    pub typo_ok: bool,
    pub needs_native_review: bool,
    /// Pronunciation tags of the underlying vocab (shown as tips after answering).
    pub pron_tags: Vec<PronTag>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    Choice(usize),
    Text(String),
    /// Indices into `options` (tiles) in the order chosen.
    Order(Vec<usize>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Judgement {
    pub correct: bool,
    pub verdict: Option<Verdict>,
    /// The canonical answer to display.
    pub expected: String,
    pub explanation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hint {
    /// Remove these option indices.
    Eliminate(Vec<usize>),
    Text(String),
}

impl Exercise {
    /// Check an answer. Does not mutate the exercise.
    pub fn check(&self, answer: &Answer, policy: AnswerPolicy) -> Judgement {
        let expected = match self.kind {
            ExKind::Choice => self
                .correct
                .first()
                .and_then(|&i| self.options.get(i))
                .cloned()
                .unwrap_or_default(),
            _ => self.accepted.first().cloned().unwrap_or_default(),
        };
        let (correct, verdict) = match (self.kind, answer) {
            (ExKind::Choice, Answer::Choice(i)) => (self.correct.contains(i), None),
            (ExKind::Type, Answer::Text(t)) => {
                let v = check_text(t, &self.accepted, policy, self.typo_ok);
                (v.is_correct(), Some(v))
            }
            (ExKind::Order, Answer::Order(idx)) => {
                let text: Vec<&str> = idx
                    .iter()
                    .filter_map(|&i| self.options.get(i))
                    .map(String::as_str)
                    .collect();
                let v = check_text(&text.join(" "), &self.accepted, policy, false);
                (v.is_correct(), Some(v))
            }
            // A mismatched answer type is simply wrong.
            _ => (false, None),
        };
        Judgement {
            correct,
            verdict,
            expected,
            explanation: self.explanation.clone(),
        }
    }

    /// Produce a hint. `level` 0 gives a mild hint.
    pub fn hint(&self, rng: &mut Rng) -> Option<Hint> {
        match self.kind {
            ExKind::Choice => {
                let mut wrong: Vec<usize> = (0..self.options.len())
                    .filter(|i| !self.correct.contains(i))
                    .collect();
                if wrong.len() <= 1 {
                    return self.hint.clone().map(Hint::Text);
                }
                rng.shuffle(&mut wrong);
                let n = (wrong.len() / 2).max(1);
                wrong.truncate(n);
                wrong.sort_unstable();
                Some(Hint::Eliminate(wrong))
            }
            ExKind::Type => {
                if let Some(h) = &self.hint {
                    return Some(Hint::Text(h.clone()));
                }
                let first = self.accepted.first()?;
                let letter: String = first.chars().take(1).collect();
                Some(Hint::Text(format!("{letter}…")))
            }
            ExKind::Order => {
                if let Some(h) = &self.hint {
                    return Some(Hint::Text(h.clone()));
                }
                let first = self.accepted.first()?.split_whitespace().next()?.to_string();
                Some(Hint::Text(format!("{first} …")))
            }
        }
    }
}

fn words_of(s: &str) -> Vec<String> {
    s.split_whitespace().map(|w| w.to_string()).collect()
}

fn strip_end_punct(s: &str) -> String {
    s.trim().trim_end_matches(['.', '!', '?']).to_string()
}

/// Build an exercise from a hand-written definition.
pub fn from_def(def: &ExerciseDef, rng: &mut Rng) -> Exercise {
    let mut ex = Exercise {
        skill: def.id.clone(),
        kind: ExKind::Choice,
        flavor: Flavor::Grammar,
        prompt: def.prompt.clone(),
        prompt_lang: def.prompt_lang,
        context: def.context.clone(),
        options: vec![],
        correct: vec![],
        accepted: def.answer.clone(),
        hint: def.hint.clone(),
        explanation: def.explanation.clone(),
        audio: None,
        ipa: None,
        typo_ok: false,
        needs_native_review: def.needs_native_review,
        pron_tags: vec![],
    };
    match def.kind {
        ExerciseKind::Choice | ExerciseKind::FillBlank => {
            let mut opts = def.options.clone();
            rng.shuffle(&mut opts);
            ex.correct = opts
                .iter()
                .enumerate()
                .filter(|(_, o)| def.answer.iter().any(|a| fold(a) == fold(o)))
                .map(|(i, _)| i)
                .collect();
            ex.options = opts;
            ex.accepted.clear();
        }
        ExerciseKind::Type => {
            ex.kind = ExKind::Type;
        }
        ExerciseKind::Order => {
            ex.kind = ExKind::Order;
            let mut tiles = words_of(&strip_end_punct(
                def.answer.first().map(String::as_str).unwrap_or(""),
            ));
            rng.shuffle(&mut tiles);
            ex.options = tiles;
            ex.accepted = def.answer.iter().map(|a| strip_end_punct(a)).collect();
        }
    }
    ex
}

fn distractor_pool<'a>(content: &'a Content, target: &VocabItem) -> Vec<&'a VocabItem> {
    let mut same: Vec<&VocabItem> = content
        .vocab
        .values()
        .filter(|v| v.id != target.id && v.category == target.category && v.kind == target.kind)
        .collect();
    let mut other: Vec<&VocabItem> = content
        .vocab
        .values()
        .filter(|v| v.id != target.id && v.category != target.category)
        .collect();
    same.append(&mut other);
    same
}

fn pick_distractors<'a>(
    pool: &[&'a VocabItem],
    n: usize,
    rng: &mut Rng,
    key: impl Fn(&VocabItem) -> String,
    avoid: &str,
) -> Vec<&'a VocabItem> {
    // Take distractors preferring the front of the pool (same category), with shuffling inside tiers.
    let mut head: Vec<&VocabItem> = pool.iter().take(n * 3).copied().collect();
    rng.shuffle(&mut head);
    let mut out: Vec<&VocabItem> = vec![];
    let mut seen: Vec<String> = vec![fold(avoid)];
    for v in head.into_iter().chain(pool.iter().skip(n * 3).copied()) {
        let k = fold(&key(v));
        if seen.contains(&k) {
            continue;
        }
        seen.push(k);
        out.push(v);
        if out.len() == n {
            break;
        }
    }
    out
}

fn place_correct(options: &mut Vec<String>, correct: String, rng: &mut Rng) -> usize {
    let at = rng.below(options.len() + 1);
    options.insert(at, correct);
    at
}

/// Controls which exercise flavours are possible.
#[derive(Debug, Clone, Copy, Default)]
pub struct GenOptions {
    /// True when the clip for the vocab item exists (listening exercises use real audio).
    pub audio_available: bool,
}

/// Generate an exercise for a skill (vocab id or grammar exercise id).
pub fn generate(
    content: &Content,
    skill: &str,
    card: Option<&Card>,
    rng: &mut Rng,
    opts: impl Fn(&str) -> GenOptions,
) -> Option<Exercise> {
    if let Some(def) = content.grammar_exercise(skill) {
        return Some(from_def(def, rng));
    }
    let item = content.vocab.get(skill)?;
    Some(generate_vocab(content, item, card, rng, opts(skill)))
}

/// Pick a flavour appropriate to how well the card is known, then build it.
pub fn generate_vocab(
    content: &Content,
    item: &VocabItem,
    card: Option<&Card>,
    rng: &mut Rng,
    gen_opts: GenOptions,
) -> Exercise {
    #[derive(Clone, Copy)]
    enum Pick {
        PtToEn,
        EnToPt,
        Article,
        Listen,
        TypeEnPt,
        Order,
    }
    let seen = card.map(|c| c.seen).unwrap_or(0);
    let is_review = matches!(card.map(|c| c.phase), Some(Phase::Review));
    let has_gender = item.kind == WordKind::Noun && item.gender.is_some();
    let order_ok = item
        .example
        .as_ref()
        .map(|e| (3..=8).contains(&e.pt.split_whitespace().count()))
        .unwrap_or(false);
    let ipa_ok = item.ipa.is_some();
    let mut picks: Vec<Pick> = vec![];
    if seen < 2 && !is_review {
        picks.push(Pick::PtToEn);
        picks.push(Pick::EnToPt);
    } else if seen < 6 {
        picks.push(Pick::EnToPt);
        picks.push(Pick::PtToEn);
        if has_gender {
            picks.push(Pick::Article);
        }
        if ipa_ok {
            picks.push(Pick::Listen);
        }
        if order_ok {
            picks.push(Pick::Order);
        }
    } else {
        picks.push(Pick::TypeEnPt);
        picks.push(Pick::EnToPt);
        if has_gender {
            picks.push(Pick::Article);
        }
        if order_ok {
            picks.push(Pick::Order);
        }
        if ipa_ok {
            picks.push(Pick::Listen);
        }
    }
    let pick = *rng.pick(&picks).unwrap_or(&Pick::PtToEn);
    let pool = distractor_pool(content, item);
    let mut base = Exercise {
        skill: item.id.clone(),
        kind: ExKind::Choice,
        flavor: Flavor::PtToEn,
        prompt: String::new(),
        prompt_lang: Lang::Pt,
        context: None,
        options: vec![],
        correct: vec![],
        accepted: vec![],
        hint: item.note.clone().filter(|_| false),
        explanation: None,
        audio: None,
        ipa: item.ipa.clone(),
        typo_ok: true,
        needs_native_review: item.needs_native_review,
        pron_tags: item.pron_tags.clone(),
    };
    let pt_shown = item.with_article();
    let explain = |item: &VocabItem| -> Option<String> {
        let mut s = format!("{} = {}", item.with_article(), item.en.join(" / "));
        if let Some(n) = &item.not_pt_br {
            s.push_str(&format!("  (not \"{n}\": that is Brazilian)"));
        }
        Some(s)
    };
    match pick {
        Pick::PtToEn => {
            base.flavor = Flavor::PtToEn;
            base.prompt = pt_shown;
            base.prompt_lang = Lang::Pt;
            let d = pick_distractors(&pool, 3, rng, |v| v.primary_en().to_string(), item.primary_en());
            let mut options: Vec<String> = d.iter().map(|v| v.primary_en().to_string()).collect();
            let at = place_correct(&mut options, item.primary_en().to_string(), rng);
            base.options = options;
            base.correct = vec![at];
            base.hint = item.example.as_ref().map(|e| format!("{} / {}", e.pt, e.en));
        }
        Pick::EnToPt => {
            base.flavor = Flavor::EnToPt;
            base.prompt = item.primary_en().to_string();
            base.prompt_lang = Lang::En;
            let d = pick_distractors(&pool, 3, rng, |v| v.with_article(), &item.with_article());
            let mut options: Vec<String> = d.iter().map(|v| v.with_article()).collect();
            let at = place_correct(&mut options, item.with_article(), rng);
            base.options = options;
            base.correct = vec![at];
        }
        Pick::Article => {
            base.flavor = Flavor::Article;
            let g = item.gender.unwrap_or(Gender::Masc);
            let indef = rng.chance(40);
            base.prompt = format!("___ {}", item.pt);
            base.prompt_lang = Lang::Pt;
            base.context = Some(if indef {
                format!("a / an {}", item.primary_en())
            } else {
                format!("the {}", item.primary_en())
            });
            let (a, b) = if indef { ("um", "uma") } else { ("o", "a") };
            base.options = vec![a.to_string(), b.to_string()];
            let right = if indef { g.indefinite() } else { g.definite() };
            base.correct = vec![base.options.iter().position(|o| o == right).unwrap_or(0)];
            base.hint = Some(match g {
                Gender::Masc => "masculine noun".to_string(),
                Gender::Fem => "feminine noun".to_string(),
            });
        }
        Pick::Listen => {
            let real = gen_opts.audio_available && item.audio.is_some();
            base.flavor = if real { Flavor::Listen } else { Flavor::ListenIpa };
            base.audio = item.audio.clone().filter(|_| real);
            base.prompt = if real {
                String::new()
            } else {
                format!("[{}]", item.ipa.clone().unwrap_or_default())
            };
            base.prompt_lang = Lang::Pt;
            let d = pick_distractors(&pool, 3, rng, |v| v.with_article(), &item.with_article());
            let mut options: Vec<String> = d.iter().map(|v| v.with_article()).collect();
            let at = place_correct(&mut options, item.with_article(), rng);
            base.options = options;
            base.correct = vec![at];
        }
        Pick::TypeEnPt => {
            base.kind = ExKind::Type;
            base.flavor = Flavor::EnToPt;
            base.prompt = item.primary_en().to_string();
            base.prompt_lang = Lang::En;
            let mut acc = vec![item.pt.clone()];
            if has_gender {
                acc.push(item.with_article());
            }
            base.accepted = acc;
            if item.pt.chars().count() < 5 {
                base.typo_ok = false;
            }
            base.hint = None;
        }
        Pick::Order => {
            let ex = item.example.clone().unwrap_or_default();
            base.kind = ExKind::Order;
            base.flavor = Flavor::Sentence;
            base.prompt = ex.en.clone();
            base.prompt_lang = Lang::En;
            let mut tiles = words_of(&strip_end_punct(&ex.pt));
            rng.shuffle(&mut tiles);
            base.options = tiles;
            base.accepted = vec![strip_end_punct(&ex.pt)];
            base.typo_ok = false;
        }
    }
    base.explanation = explain(item);
    base
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn sample_content() -> Content {
        let mut c = Content::default();
        let mk = |id: &str, pt: &str, en: &str, cat: &str, g: Option<Gender>| VocabItem {
            id: id.into(),
            pt: pt.into(),
            en: vec![en.into()],
            category: cat.into(),
            gender: g,
            ipa: Some("x".into()),
            example: Some(Example {
                pt: format!("Eu quero {pt} agora."),
                en: format!("I want {en} now."),
            }),
            ..VocabItem::default()
        };
        for (id, pt, en, g) in [
            ("casa", "casa", "house", Gender::Fem),
            ("livro", "livro", "book", Gender::Masc),
            ("mesa", "mesa", "table", Gender::Fem),
            ("carro", "carro", "car", Gender::Masc),
            ("gato", "gato", "cat", Gender::Masc),
        ] {
            c.vocab.insert(id.into(), mk(id, pt, en, "stuff", Some(g)));
        }
        c.grammar.insert(
            "g".into(),
            GrammarConcept {
                id: "g".into(),
                title: "t".into(),
                summary: "s".into(),
                rules: vec![],
                examples: vec![],
                pt_pt_note: None,
                exercises: vec![
                    ExerciseDef {
                        id: "g1".into(),
                        kind: ExerciseKind::FillBlank,
                        prompt: "Eu ___ português.".into(),
                        prompt_lang: Lang::Pt,
                        context: None,
                        options: vec!["sou".into(), "estou".into(), "são".into()],
                        answer: vec!["sou".into()],
                        hint: None,
                        explanation: Some("ser for identity".into()),
                        needs_native_review: false,
                    },
                    ExerciseDef {
                        id: "g2".into(),
                        kind: ExerciseKind::Order,
                        prompt: "I am eating.".into(),
                        prompt_lang: Lang::En,
                        context: None,
                        options: vec![],
                        answer: vec!["Estou a comer.".into()],
                        hint: None,
                        explanation: None,
                        needs_native_review: false,
                    },
                ],
                needs_native_review: false,
            },
        );
        c.finalize();
        c
    }

    #[test]
    fn grammar_choice_checks() {
        let c = sample_content();
        let mut rng = Rng::new(1);
        let ex = generate(&c, "g1", None, &mut rng, |_| GenOptions::default()).unwrap();
        assert_eq!(ex.kind, ExKind::Choice);
        let right = ex.correct[0];
        assert!(ex.check(&Answer::Choice(right), AnswerPolicy::default()).correct);
        let wrong = (0..ex.options.len()).find(|i| *i != right).unwrap();
        assert!(!ex.check(&Answer::Choice(wrong), AnswerPolicy::default()).correct);
    }

    #[test]
    fn order_exercise_accepts_right_order_only() {
        let c = sample_content();
        let mut rng = Rng::new(5);
        let ex = generate(&c, "g2", None, &mut rng, |_| GenOptions::default()).unwrap();
        assert_eq!(ex.kind, ExKind::Order);
        let mut idx: Vec<usize> = vec![];
        for w in ["Estou", "a", "comer"] {
            idx.push(ex.options.iter().position(|o| o == w).unwrap());
        }
        assert!(
            ex.check(&Answer::Order(idx.clone()), AnswerPolicy::default())
                .correct
        );
        idx.reverse();
        assert!(!ex.check(&Answer::Order(idx), AnswerPolicy::default()).correct);
    }

    #[test]
    fn vocab_exercises_are_always_answerable_and_well_formed() {
        let c = sample_content();
        for seed in 0..60 {
            let mut rng = Rng::new(seed);
            for seen in [0u32, 3, 9] {
                let card = Card {
                    seen,
                    phase: Phase::Review,
                    ..Card::default()
                };
                for id in c.vocab.keys() {
                    let ex = generate(&c, id, Some(&card), &mut rng, |_| GenOptions {
                        audio_available: false,
                    })
                    .unwrap();
                    match ex.kind {
                        ExKind::Choice => {
                            assert!(!ex.correct.is_empty());
                            assert!(ex.correct.iter().all(|&i| i < ex.options.len()));
                            let mut f: Vec<String> = ex.options.iter().map(|o| fold(o)).collect();
                            f.sort();
                            f.dedup();
                            assert_eq!(f.len(), ex.options.len(), "duplicate options in {ex:?}");
                            assert!(
                                ex.check(&Answer::Choice(ex.correct[0]), AnswerPolicy::default())
                                    .correct
                            );
                        }
                        ExKind::Type => {
                            let a = ex.accepted[0].clone();
                            assert!(ex.check(&Answer::Text(a), AnswerPolicy::default()).correct);
                        }
                        ExKind::Order => {
                            let canon: Vec<&str> = ex.accepted[0].split_whitespace().collect();
                            let idx: Vec<usize> = canon
                                .iter()
                                .scan(vec![false; ex.options.len()], |used, w| {
                                    let i =
                                        (0..ex.options.len()).find(|&i| !used[i] && ex.options[i] == *w)?;
                                    used[i] = true;
                                    Some(i)
                                })
                                .collect();
                            assert_eq!(idx.len(), canon.len());
                            assert!(ex.check(&Answer::Order(idx), AnswerPolicy::default()).correct);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn article_exercise_matches_gender() {
        let c = sample_content();
        let item = &c.vocab["casa"];
        for seed in 0..40 {
            let mut rng = Rng::new(seed);
            let card = Card {
                seen: 3,
                phase: Phase::Review,
                ..Card::default()
            };
            let ex = generate_vocab(&c, item, Some(&card), &mut rng, GenOptions::default());
            if ex.flavor == Flavor::Article {
                let right = &ex.options[ex.correct[0]];
                assert!(right == "a" || right == "uma", "got {right}");
            }
        }
    }

    #[test]
    fn typed_vocab_accepts_missing_accent_and_article() {
        let mut c = sample_content();
        c.vocab.get_mut("casa").unwrap().pt = "pão".into();
        c.finalize();
        let mut rng = Rng::new(1);
        let card = Card {
            seen: 20,
            phase: Phase::Review,
            ..Card::default()
        };
        let mut found = false;
        for _ in 0..200 {
            let ex = generate_vocab(&c, &c.vocab["casa"], Some(&card), &mut rng, GenOptions::default());
            if ex.kind == ExKind::Type {
                found = true;
                assert!(
                    ex.check(&Answer::Text("pao".into()), AnswerPolicy::default())
                        .correct
                );
                assert!(
                    !ex.check(&Answer::Text("pao".into()), AnswerPolicy::STRICT)
                        .correct
                );
                assert!(
                    ex.check(&Answer::Text("a pão".into()), AnswerPolicy::default())
                        .correct
                );
                assert!(
                    !ex.check(&Answer::Text("gato".into()), AnswerPolicy::default())
                        .correct
                );
                break;
            }
        }
        assert!(found);
    }

    #[test]
    fn hints() {
        let c = sample_content();
        let mut rng = Rng::new(9);
        let ex = generate(&c, "g1", None, &mut rng, |_| GenOptions::default()).unwrap();
        match ex.hint(&mut rng) {
            Some(Hint::Eliminate(v)) => {
                assert!(!v.is_empty());
                assert!(
                    v.iter().all(|i| !ex.correct.contains(i)),
                    "never eliminate the right answer"
                );
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}
