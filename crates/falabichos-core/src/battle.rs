// SPDX-License-Identifier: Apache-2.0
//! Conversation battles: wild Bicho encounters (win trust to catch), trainer / Mestre challenges,
//! and practice sessions. Pure state machine: the UI feeds answers in and reads state out.

use crate::content::Content;
use crate::exercise::{Answer, Exercise, GenOptions, Hint, Judgement, generate};
use crate::normalize::{AnswerPolicy, Verdict};
use crate::progress::{Progress, party_bonus};
use crate::rng::Rng;
use crate::srs::{Grade, SrsStore};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BattleError {
    #[error("unknown species `{0}`")]
    UnknownSpecies(String),
    #[error("unknown challenge `{0}`")]
    UnknownChallenge(String),
    #[error("nothing to ask: the exercise pool is empty")]
    EmptyPool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattleKind {
    Wild { species: String },
    Challenge { id: String },
    Practice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Wild: trust full, the Bicho joins.
    Caught,
    /// Challenge passed.
    Won,
    /// Out of confidence (wild Bicho ran away / challenge failed).
    Lost,
    /// Practice session finished.
    Completed,
    /// Player left.
    Fled,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnswerReport {
    pub judgement: Judgement,
    pub grade: Grade,
    pub trust_gain: u8,
    /// Set when this answer ended the battle.
    pub outcome: Option<Outcome>,
}

#[derive(Debug, Clone)]
pub struct Battle {
    pub kind: BattleKind,
    pub name: String,
    pub confidence: u8,
    pub max_confidence: u8,
    pub trust: u8,
    pub trust_needed: u8,
    pub streak: u8,
    pub asked: u32,
    pub correct: u32,
    pub wrong: u32,
    pub hints_left: u8,
    /// Total questions for challenge / practice (None = until caught or beaten).
    pub total: Option<u8>,
    pub pass_correct: u8,
    pub current: Option<Exercise>,
    pub hint_used: bool,
    pub outcome: Option<Outcome>,
    pub policy: AnswerPolicy,
    pool: Vec<String>,
    last_skill: Option<String>,
    answered: bool,
    /// Skills answered wrongly this battle (for the post-battle review).
    pub missed: Vec<String>,
}

impl Battle {
    fn base(kind: BattleKind, name: String, pool: Vec<String>) -> Battle {
        Battle {
            kind,
            name,
            confidence: 5,
            max_confidence: 5,
            trust: 0,
            trust_needed: 5,
            streak: 0,
            asked: 0,
            correct: 0,
            wrong: 0,
            hints_left: 1,
            total: None,
            pass_correct: 0,
            current: None,
            hint_used: false,
            outcome: None,
            policy: AnswerPolicy::default(),
            pool,
            last_skill: None,
            answered: false,
            missed: vec![],
        }
    }

    /// Start a wild encounter.
    pub fn wild(
        content: &Content,
        species: &str,
        progress: &mut Progress,
        srs: &SrsStore,
        policy: AnswerPolicy,
        start_confidence: Option<u8>,
        rng: &mut Rng,
        now: i64,
    ) -> Result<Battle, BattleError> {
        let sp = content.species.get(species).ok_or_else(|| BattleError::UnknownSpecies(species.to_string()))?;
        progress.see(species);
        let pool = content.species_skills(species);
        if pool.is_empty() {
            return Err(BattleError::EmptyPool);
        }
        let mut b = Battle::base(BattleKind::Wild { species: species.to_string() }, sp.name.clone(), pool);
        let (conf, hints) = party_bonus(content, srs, progress);
        b.max_confidence = 4 + conf;
        b.confidence = start_confidence.map(|c| c.clamp(1, b.max_confidence)).unwrap_or(b.max_confidence);
        b.hints_left = hints;
        b.trust_needed = sp.trust_needed.max(1);
        b.policy = policy;
        b.next_question(content, srs, rng, now);
        Ok(b)
    }

    /// Start a trainer or Mestre challenge.
    pub fn challenge(
        content: &Content,
        id: &str,
        srs: &SrsStore,
        policy: AnswerPolicy,
        rng: &mut Rng,
        now: i64,
    ) -> Result<Battle, BattleError> {
        let ch = content.challenges.get(id).ok_or_else(|| BattleError::UnknownChallenge(id.to_string()))?;
        let pool = content.pool_skills(&ch.pool);
        if pool.is_empty() {
            return Err(BattleError::EmptyPool);
        }
        let mut b = Battle::base(BattleKind::Challenge { id: id.to_string() }, ch.name.clone(), pool);
        b.max_confidence = ch.confidence.max(1);
        b.confidence = b.max_confidence;
        b.total = Some(ch.questions.max(1));
        b.pass_correct = ch.pass_correct;
        b.hints_left = 0; // no hints in tests
        b.policy = policy;
        b.next_question(content, srs, rng, now);
        Ok(b)
    }

    /// Practice (Treino): review the words of the party's Bichos. No penalty.
    pub fn practice(
        content: &Content,
        progress: &Progress,
        srs: &SrsStore,
        policy: AnswerPolicy,
        questions: u8,
        rng: &mut Rng,
        now: i64,
    ) -> Result<Battle, BattleError> {
        let mut pool: Vec<String> = vec![];
        for b in &progress.party {
            pool.extend(content.species_skills(&b.species));
        }
        // Only skills already started are "review"; if nothing started yet, use everything.
        let started: Vec<String> = pool.iter().filter(|s| srs.card(s).is_some()).cloned().collect();
        let pool = if started.is_empty() { pool } else { started };
        if pool.is_empty() {
            return Err(BattleError::EmptyPool);
        }
        let mut b = Battle::base(BattleKind::Practice, "Practice".to_string(), pool);
        b.confidence = 99;
        b.max_confidence = 99;
        b.total = Some(questions.max(1));
        b.hints_left = 99;
        b.policy = policy;
        b.next_question(content, srs, rng, now);
        Ok(b)
    }

    pub fn is_over(&self) -> bool {
        self.outcome.is_some()
    }

    /// Whether the current question was answered and the UI should show feedback/advance.
    pub fn awaiting_advance(&self) -> bool {
        self.answered
    }

    fn next_question(&mut self, content: &Content, srs: &SrsStore, rng: &mut Rng, now: i64) {
        self.answered = false;
        self.hint_used = false;
        let mut tries = 0;
        loop {
            let Some(skill) = srs.pick_next(&self.pool, now, 40, self.last_skill.as_deref(), rng) else {
                self.current = None;
                return;
            };
            self.last_skill = Some(skill.clone());
            let ex = generate(content, &skill, srs.card(&skill), rng, |_| GenOptions::default());
            if ex.is_some() || tries > 20 {
                self.current = ex;
                return;
            }
            tries += 1;
        }
    }

    /// Spend a hint on the current question. Returns the hint, or `None` if none are left.
    pub fn use_hint(&mut self, rng: &mut Rng) -> Option<Hint> {
        if self.hints_left == 0 || self.answered || self.hint_used {
            return None;
        }
        let h = self.current.as_ref()?.hint(rng)?;
        self.hints_left = self.hints_left.saturating_sub(1);
        self.hint_used = true;
        Some(h)
    }

    /// Submit an answer to the current question. Updates trust/confidence and the SRS.
    pub fn answer(&mut self, answer: &Answer, srs: &mut SrsStore, now: i64) -> Option<AnswerReport> {
        if self.outcome.is_some() || self.answered {
            return None;
        }
        let ex = self.current.as_ref()?;
        let judgement = ex.check(answer, self.policy);
        let skill = ex.skill.clone();
        self.asked += 1;
        self.answered = true;
        let mut trust_gain = 0u8;
        let grade;
        if judgement.correct {
            self.correct += 1;
            self.streak = self.streak.saturating_add(1);
            grade = match &judgement.verdict {
                _ if self.hint_used => Grade::Hard,
                Some(Verdict::AccentSlip { .. }) | Some(Verdict::Typo { .. }) => Grade::Hard,
                Some(Verdict::Exact) if self.streak >= 3 => Grade::Easy,
                _ => Grade::Good,
            };
            if matches!(self.kind, BattleKind::Wild { .. }) {
                trust_gain = 1 + u8::from(self.streak >= 3 && self.streak % 3 == 0);
                self.trust = (self.trust + trust_gain).min(self.trust_needed);
            }
        } else {
            self.wrong += 1;
            self.streak = 0;
            grade = Grade::Again;
            self.confidence = self.confidence.saturating_sub(1);
            self.missed.push(skill.clone());
        }
        srs.review(&skill, grade, now);
        self.outcome = self.evaluate_outcome();
        Some(AnswerReport { judgement, grade, trust_gain, outcome: self.outcome })
    }

    fn evaluate_outcome(&self) -> Option<Outcome> {
        match &self.kind {
            BattleKind::Wild { .. } => {
                if self.trust >= self.trust_needed {
                    Some(Outcome::Caught)
                } else if self.confidence == 0 {
                    Some(Outcome::Lost)
                } else {
                    None
                }
            }
            BattleKind::Challenge { .. } => {
                let total = self.total.unwrap_or(1) as u32;
                if self.confidence == 0 {
                    Some(Outcome::Lost)
                } else if self.asked >= total {
                    Some(if self.correct >= self.pass_correct as u32 { Outcome::Won } else { Outcome::Lost })
                } else {
                    None
                }
            }
            BattleKind::Practice => {
                if self.asked >= self.total.unwrap_or(1) as u32 {
                    Some(Outcome::Completed)
                } else {
                    None
                }
            }
        }
    }

    /// Move on after the feedback has been read. No-op if the battle is over.
    pub fn advance(&mut self, content: &Content, srs: &SrsStore, rng: &mut Rng, now: i64) {
        if self.outcome.is_none() && self.answered {
            self.next_question(content, srs, rng, now);
        }
    }

    pub fn flee(&mut self) {
        if self.outcome.is_none() {
            self.outcome = Some(Outcome::Fled);
        }
    }

    /// Remaining questions for challenge/practice.
    pub fn remaining(&self) -> Option<u32> {
        self.total.map(|t| (t as u32).saturating_sub(self.asked))
    }
}

/// Apply a finished battle to the player's progress. Returns a human-readable list of events.
pub fn settle(
    battle: &Battle,
    content: &Content,
    progress: &mut Progress,
    now: i64,
) -> Vec<SettleEvent> {
    let mut ev = vec![];
    match (&battle.kind, battle.outcome) {
        (BattleKind::Wild { species }, Some(Outcome::Caught)) => {
            if progress.add_bicho(species, now) {
                ev.push(SettleEvent::Caught(species.clone()));
            }
        }
        (BattleKind::Challenge { id }, Some(Outcome::Won)) => {
            if let Some(ch) = content.challenges.get(id) {
                progress.flags.insert(ch.reward_flag.clone());
                if let Some(seal) = &ch.seal {
                    if progress.award_seal(seal) {
                        ev.push(SettleEvent::Seal(seal.clone()));
                    }
                }
            }
        }
        _ => {}
    }
    ev
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettleEvent {
    Caught(String),
    Seal(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exercise::ExKind;
    use crate::model::*;

    fn content() -> Content {
        let mut c = Content::default();
        for (i, (pt, en)) in [("galo", "rooster"), ("vaca", "cow"), ("porco", "pig"), ("ovelha", "sheep"), ("cavalo", "horse"), ("ovo", "egg")].iter().enumerate() {
            let id = format!("v{i}");
            c.vocab.insert(
                id.clone(),
                VocabItem { id, pt: pt.to_string(), en: vec![en.to_string()], category: "farm".into(), gender: Some(Gender::Masc), ..VocabItem::default() },
            );
        }
        c.species.insert(
            "galo".into(),
            Species {
                id: "galo".into(),
                name: "Galo".into(),
                dex_no: 1,
                kind: SpeciesKind::Vocab,
                theme: "farm".into(),
                flavor_pt: "x".into(),
                flavor_en: "x".into(),
                vocab: (0..6).map(|i| format!("v{i}")).collect(),
                grammar: vec![],
                art: "galo".into(),
                trust_needed: 5,
                cry_hz: 440,
                needs_native_review: false,
            },
        );
        c.challenges.insert(
            "mestre".into(),
            Challenge {
                id: "mestre".into(),
                name: "Mestre".into(),
                kind: ChallengeKind::Mestre,
                theme: "t".into(),
                pool: Pool { categories: vec!["farm".into()], ..Pool::default() },
                questions: 4,
                pass_correct: 3,
                confidence: 2,
                art: "a".into(),
                reward_flag: "beat_mestre".into(),
                seal: Some("selo1".into()),
                win_dialogue: None,
                lose_dialogue: None,
            },
        );
        c.finalize();
        c
    }

    fn solve(b: &Battle, content: &Content) -> Answer {
        let ex = b.current.as_ref().unwrap();
        match ex.kind {
            ExKind::Choice => Answer::Choice(ex.correct[0]),
            ExKind::Type => Answer::Text(ex.accepted[0].clone()),
            ExKind::Order => {
                let _ = content;
                let canon: Vec<&str> = ex.accepted[0].split_whitespace().collect();
                let mut used = vec![false; ex.options.len()];
                let mut idx = vec![];
                for w in canon {
                    let i = (0..ex.options.len()).find(|&i| !used[i] && ex.options[i] == w).unwrap();
                    used[i] = true;
                    idx.push(i);
                }
                Answer::Order(idx)
            }
        }
    }

    fn wrong(b: &Battle) -> Answer {
        let ex = b.current.as_ref().unwrap();
        match ex.kind {
            ExKind::Choice => Answer::Choice((0..ex.options.len()).find(|i| !ex.correct.contains(i)).unwrap()),
            _ => Answer::Text("zzzzz".into()),
        }
    }

    #[test]
    fn wild_catch_flow() {
        let c = content();
        let mut p = Progress::default();
        let mut srs = SrsStore::default();
        let mut rng = Rng::new(11);
        let mut b = Battle::wild(&c, "galo", &mut p, &srs, AnswerPolicy::default(), None, &mut rng, 1000).unwrap();
        assert!(p.dex_seen.contains("galo"));
        let mut now = 1000;
        let mut guard = 0;
        while !b.is_over() {
            let a = solve(&b, &c);
            let r = b.answer(&a, &mut srs, now).unwrap();
            assert!(r.judgement.correct);
            b.advance(&c, &srs, &mut rng, now);
            now += 5;
            guard += 1;
            assert!(guard < 50);
        }
        assert_eq!(b.outcome, Some(Outcome::Caught));
        let ev = settle(&b, &c, &mut p, now);
        assert_eq!(ev, vec![SettleEvent::Caught("galo".into())]);
        assert!(p.has_bicho("galo"));
        assert!(!srs.cards.is_empty(), "answers feed the SRS");
        assert!(srs.cards.values().all(|c| c.seen >= 1));
    }

    #[test]
    fn wild_loss_when_confidence_runs_out() {
        let c = content();
        let mut p = Progress::default();
        let mut srs = SrsStore::default();
        let mut rng = Rng::new(3);
        let mut b = Battle::wild(&c, "galo", &mut p, &srs, AnswerPolicy::default(), None, &mut rng, 0).unwrap();
        assert_eq!(b.max_confidence, 4);
        for _ in 0..4 {
            assert!(!b.is_over());
            let a = wrong(&b);
            b.answer(&a, &mut srs, 0).unwrap();
            b.advance(&c, &srs, &mut rng, 0);
        }
        assert_eq!(b.outcome, Some(Outcome::Lost));
        assert!(settle(&b, &c, &mut p, 0).is_empty());
        assert!(!p.has_bicho("galo"));
        // failed skills come back soon
        assert!(srs.cards.values().all(|c| c.due <= 60));
        assert!(b.answer(&Answer::Choice(0), &mut srs, 0).is_none(), "no answers after the end");
    }

    #[test]
    fn challenge_win_awards_seal() {
        let c = content();
        let mut p = Progress::default();
        let mut srs = SrsStore::default();
        let mut rng = Rng::new(21);
        let mut b = Battle::challenge(&c, "mestre", &srs, AnswerPolicy::default(), &mut rng, 0).unwrap();
        assert_eq!(b.hints_left, 0);
        assert!(b.use_hint(&mut rng).is_none());
        while !b.is_over() {
            let a = solve(&b, &c);
            b.answer(&a, &mut srs, 0).unwrap();
            b.advance(&c, &srs, &mut rng, 0);
        }
        assert_eq!(b.outcome, Some(Outcome::Won));
        let ev = settle(&b, &c, &mut p, 0);
        assert_eq!(ev, vec![SettleEvent::Seal("selo1".into())]);
        assert!(p.has_flag("beat_mestre"));
        assert!(p.seals.contains("selo1"));
    }

    #[test]
    fn challenge_fail_by_score() {
        let c = content();
        let mut srs = SrsStore::default();
        let mut rng = Rng::new(5);
        let mut b = Battle::challenge(&c, "mestre", &srs, AnswerPolicy::default(), &mut rng, 0).unwrap();
        // 1 wrong (allowed: confidence 2), 3 right => passes (3 >= 3)
        let a = wrong(&b);
        b.answer(&a, &mut srs, 0);
        b.advance(&c, &srs, &mut rng, 0);
        for _ in 0..3 {
            let a = solve(&b, &c);
            b.answer(&a, &mut srs, 0);
            b.advance(&c, &srs, &mut rng, 0);
        }
        assert_eq!(b.outcome, Some(Outcome::Won));
    }

    #[test]
    fn hints_are_limited_and_downgrade_grade() {
        let c = content();
        let mut p = Progress::default();
        let mut srs = SrsStore::default();
        let mut rng = Rng::new(8);
        let mut b = Battle::wild(&c, "galo", &mut p, &srs, AnswerPolicy::default(), None, &mut rng, 0).unwrap();
        assert_eq!(b.hints_left, 1);
        assert!(b.use_hint(&mut rng).is_some());
        assert!(b.use_hint(&mut rng).is_none());
        let a = solve(&b, &c);
        let r = b.answer(&a, &mut srs, 0).unwrap();
        assert_eq!(r.grade, Grade::Hard);
    }

    #[test]
    fn practice_completes_without_penalty() {
        let c = content();
        let mut p = Progress::default();
        p.add_bicho("galo", 0);
        let mut srs = SrsStore::default();
        let mut rng = Rng::new(2);
        let mut b = Battle::practice(&c, &p, &srs, AnswerPolicy::default(), 3, &mut rng, 0).unwrap();
        for _ in 0..3 {
            let a = wrong(&b);
            b.answer(&a, &mut srs, 0);
            b.advance(&c, &srs, &mut rng, 0);
        }
        assert_eq!(b.outcome, Some(Outcome::Completed));
        assert!(b.confidence > 90);
    }

    #[test]
    fn unknown_ids_error() {
        let c = content();
        let mut p = Progress::default();
        let srs = SrsStore::default();
        let mut rng = Rng::new(1);
        assert_eq!(
            Battle::wild(&c, "nope", &mut p, &srs, AnswerPolicy::default(), None, &mut rng, 0).unwrap_err(),
            BattleError::UnknownSpecies("nope".into())
        );
        assert!(Battle::challenge(&c, "nope", &srs, AnswerPolicy::default(), &mut rng, 0).is_err());
    }
}
