// SPDX-License-Identifier: Apache-2.0
//! Spaced-repetition scheduler (an SM-2 derivative with Anki-style learning steps).
//!
//! * New / lapsed cards go through short *learning steps* (1 min, 10 min) so a failed item comes
//!   back within the same play session.
//! * Graduated cards use SM-2 intervals measured in days with an ease factor.
//! * Reviewing a card *before* it is due ("extra practice", which battles do all the time) never
//!   grows the interval: only on-time reviews are rewarded, but failures always count.
//!
//! Time is passed in explicitly (unix seconds) so everything is deterministic and testable.

use crate::rng::Rng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const DAY: i64 = 86_400;
/// Learning steps in seconds.
pub const LEARNING_STEPS: [i64; 2] = [60, 600];
pub const MIN_EASE: f32 = 1.3;
pub const MAX_EASE: f32 = 3.0;
pub const START_EASE: f32 = 2.5;
pub const MAX_INTERVAL_DAYS: f32 = 365.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Grade {
    /// Wrong answer.
    Again,
    /// Right, but with a hint or an accent/typo slip.
    Hard,
    /// Right.
    Good,
    /// Right, effortlessly (typed exactly, streak).
    Easy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    New,
    /// In learning steps; the number is the index into [`LEARNING_STEPS`].
    Learning(u8),
    Review,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Card {
    pub phase: Phase,
    pub ease: f32,
    /// Current interval in days (Review phase).
    pub interval_days: f32,
    /// Unix seconds when the card is next due.
    pub due: i64,
    pub last_review: i64,
    pub reps: u32,
    pub lapses: u32,
    pub seen: u32,
    pub correct: u32,
}

impl Default for Card {
    fn default() -> Self {
        Card {
            phase: Phase::New,
            ease: START_EASE,
            interval_days: 0.0,
            due: 0,
            last_review: 0,
            reps: 0,
            lapses: 0,
            seen: 0,
            correct: 0,
        }
    }
}

impl Card {
    pub fn is_due(&self, now: i64) -> bool {
        self.phase != Phase::New && self.due <= now
    }

    /// Mastery in `0.0..=1.0`: 0 for new cards, saturating at a 30-day interval.
    pub fn mastery(&self) -> f32 {
        match self.phase {
            Phase::New => 0.0,
            Phase::Learning(_) => 0.1,
            Phase::Review => (0.2 + 0.8 * (self.interval_days / 30.0)).clamp(0.2, 1.0),
        }
    }

    /// Apply a review result at time `now`.
    pub fn review(&mut self, grade: Grade, now: i64) {
        let was_early = self.phase == Phase::Review && now < self.due;
        self.seen += 1;
        self.last_review = now;
        if grade != Grade::Again {
            self.correct += 1;
        }
        match (self.phase, grade) {
            // ---- failures
            (Phase::Review, Grade::Again) => {
                self.lapses += 1;
                self.ease = (self.ease - 0.2).max(MIN_EASE);
                // Remember a reduced interval to return to after relearning.
                self.interval_days = (self.interval_days * 0.25).max(1.0);
                self.phase = Phase::Learning(0);
                self.due = now + LEARNING_STEPS[0];
            }
            (_, Grade::Again) => {
                self.phase = Phase::Learning(0);
                self.due = now + LEARNING_STEPS[0];
            }
            // ---- learning
            (Phase::New, g) | (Phase::Learning(_), g) => {
                let step = match self.phase {
                    Phase::Learning(s) => s as usize,
                    _ => 0,
                };
                match g {
                    Grade::Easy => self.graduate(now, 4.0),
                    Grade::Hard => {
                        self.phase = Phase::Learning(step as u8);
                        self.due = now + LEARNING_STEPS[step.min(LEARNING_STEPS.len() - 1)];
                    }
                    _ => {
                        if step + 1 < LEARNING_STEPS.len() {
                            self.phase = Phase::Learning(step as u8 + 1);
                            self.due = now + LEARNING_STEPS[step + 1];
                        } else {
                            let base = if self.interval_days >= 1.0 {
                                self.interval_days
                            } else {
                                1.0
                            };
                            self.graduate(now, base);
                        }
                    }
                }
            }
            // ---- on-time or early review of a graduated card
            (Phase::Review, g) => {
                if was_early {
                    // Extra practice: no growth, but keep the due date.
                    return;
                }
                let prev = self.interval_days.max(1.0);
                let (factor, ease_delta) = match g {
                    Grade::Hard => (1.2, -0.15),
                    Grade::Good => (self.ease, 0.0),
                    Grade::Easy => (self.ease * 1.3, 0.15),
                    Grade::Again => unreachable!(),
                };
                self.ease = (self.ease + ease_delta).clamp(MIN_EASE, MAX_EASE);
                let next = (prev * factor).max(prev + 1.0).min(MAX_INTERVAL_DAYS);
                self.interval_days = next;
                self.reps += 1;
                self.due = now + (next * DAY as f32) as i64;
            }
        }
    }

    fn graduate(&mut self, now: i64, days: f32) {
        self.phase = Phase::Review;
        self.interval_days = days.clamp(1.0, MAX_INTERVAL_DAYS);
        self.reps += 1;
        self.due = now + (self.interval_days * DAY as f32) as i64;
    }
}

/// How many days ahead the review calendar looks.
pub const CALENDAR_DAYS: usize = 7;

/// A summary of the review workload: what is due now, within the next 24 hours ("today") and the
/// 24 hours after that ("tomorrow"), and a per-day histogram for the next week.
///
/// The windows are rolling 24-hour windows measured from `now`, not calendar days: the engine has
/// no time-zone information (only unix seconds are injected), and a rolling window is honest about
/// what it measures.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReviewCalendar {
    /// Cards already due (overdue included).
    pub due_now: usize,
    /// ...of which are in learning steps (short intervals; they come back within the session).
    pub due_now_learning: usize,
    /// Cards that become due within the next 24 hours (after `now`).
    pub later_today: usize,
    /// Cards that become due 24 to 48 hours from now.
    pub tomorrow: usize,
    /// Histogram of the next [`CALENDAR_DAYS`] windows of 24 h. Index 0 is "now to +24 h" and also
    /// counts everything already due.
    pub week: [usize; CALENDAR_DAYS],
    /// Cards due after the end of the week.
    pub later: usize,
    /// Cards started (not `Phase::New`).
    pub started: usize,
}

impl ReviewCalendar {
    /// Cards due now or within 24 hours.
    pub fn today(&self) -> usize {
        self.due_now + self.later_today
    }
}

/// All cards of a player, keyed by skill id (a vocab id or a grammar exercise id).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SrsStore {
    pub cards: BTreeMap<String, Card>,
}

impl SrsStore {
    pub fn card(&self, id: &str) -> Option<&Card> {
        self.cards.get(id)
    }

    pub fn review(&mut self, id: &str, grade: Grade, now: i64) -> &Card {
        let c = self.cards.entry(id.to_string()).or_default();
        c.review(grade, now);
        c
    }

    /// Mastery of one skill (0 if unseen).
    pub fn mastery(&self, id: &str) -> f32 {
        self.cards.get(id).map(Card::mastery).unwrap_or(0.0)
    }

    /// Number of cards due now.
    pub fn due_count(&self, now: i64) -> usize {
        self.cards.values().filter(|c| c.is_due(now)).count()
    }

    /// The review calendar at time `now` (see [`ReviewCalendar`]).
    pub fn calendar(&self, now: i64) -> ReviewCalendar {
        let mut cal = ReviewCalendar::default();
        for c in self.cards.values().filter(|c| c.phase != Phase::New) {
            cal.started += 1;
            if c.due <= now {
                cal.due_now += 1;
                if matches!(c.phase, Phase::Learning(_)) {
                    cal.due_now_learning += 1;
                }
                cal.week[0] += 1;
                continue;
            }
            // whole 24 h windows from now: 0 = (now, now+24h], 1 = (now+24h, now+48h], ...
            let window = ((c.due - now - 1) / DAY) as usize;
            match window {
                0 => cal.later_today += 1,
                1 => cal.tomorrow += 1,
                _ => {}
            }
            match cal.week.get_mut(window) {
                Some(slot) => *slot += 1,
                None => cal.later += 1,
            }
        }
        cal
    }

    /// Pick the next skill to show from `candidates`.
    ///
    /// Priority: (1) learning/lapsed cards that are due, most overdue first;
    /// (2) review cards that are due, most overdue first; (3) brand-new cards, but only
    /// `new_ratio` percent of the time once something has already been seen, so sessions mix new and old;
    /// (4) otherwise the card due soonest ("extra practice").
    /// `avoid` (the skill just asked) is skipped when there is any alternative.
    pub fn pick_next(
        &self,
        candidates: &[String],
        now: i64,
        new_ratio: u8,
        avoid: Option<&str>,
        rng: &mut Rng,
    ) -> Option<String> {
        let pool: Vec<&String> = {
            let filtered: Vec<&String> = candidates.iter().filter(|c| Some(c.as_str()) != avoid).collect();
            if filtered.is_empty() {
                candidates.iter().collect()
            } else {
                filtered
            }
        };
        if pool.is_empty() {
            return None;
        }
        let mut learning_due: Vec<(&String, i64)> = vec![];
        let mut review_due: Vec<(&String, i64)> = vec![];
        let mut news: Vec<&String> = vec![];
        let mut rest: Vec<(&String, i64)> = vec![];
        for id in &pool {
            match self.cards.get(*id) {
                None => news.push(id),
                Some(c) if c.phase == Phase::New => news.push(id),
                Some(c) if c.is_due(now) => match c.phase {
                    Phase::Learning(_) => learning_due.push((id, c.due)),
                    _ => review_due.push((id, c.due)),
                },
                Some(c) => rest.push((id, c.due)),
            }
        }
        learning_due.sort_by_key(|x| x.1);
        if let Some((id, _)) = learning_due.first() {
            return Some((*id).clone());
        }
        let have_seen = !review_due.is_empty() || !rest.is_empty();
        let want_new = !news.is_empty() && (!have_seen || rng.chance(new_ratio));
        if want_new {
            return rng.pick(&news).map(|s| (*s).clone());
        }
        review_due.sort_by_key(|x| x.1);
        if let Some((id, _)) = review_due.first() {
            return Some((*id).clone());
        }
        if let Some(id) = rng.pick(&news) {
            return Some((*id).clone());
        }
        rest.sort_by_key(|x| x.1);
        rest.first().map(|(id, _)| (*id).clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;
    use proptest::prelude::{Just, Strategy, prop_assert, prop_assert_eq, prop_oneof, proptest};

    #[test]
    fn new_card_goes_through_steps_then_graduates() {
        let mut c = Card::default();
        c.review(Grade::Good, 1000);
        assert_eq!(c.phase, Phase::Learning(1));
        assert_eq!(c.due, 1000 + 600);
        c.review(Grade::Good, 1600);
        assert_eq!(c.phase, Phase::Review);
        assert_eq!(c.interval_days, 1.0);
        assert_eq!(c.due, 1600 + DAY);
    }

    #[test]
    fn failed_item_returns_within_minutes() {
        let mut c = Card::default();
        c.review(Grade::Good, 0);
        c.review(Grade::Good, 600); // graduated
        c.review(Grade::Good, 600 + DAY); // 2.5 -> interval
        let before = c.interval_days;
        assert!(before > 1.0);
        c.review(Grade::Again, 10 * DAY);
        assert_eq!(c.phase, Phase::Learning(0));
        assert_eq!(c.due, 10 * DAY + 60);
        assert_eq!(c.lapses, 1);
        assert!(c.interval_days < before);
    }

    #[test]
    fn early_review_does_not_grow_interval() {
        let mut c = Card::default();
        c.review(Grade::Easy, 0);
        let (i, d) = (c.interval_days, c.due);
        c.review(Grade::Good, 100);
        assert_eq!(c.interval_days, i);
        assert_eq!(c.due, d);
    }

    #[test]
    fn mastery_monotone_with_interval() {
        let mut c = Card {
            phase: Phase::Review,
            interval_days: 1.0,
            ..Card::default()
        };
        let m1 = c.mastery();
        c.interval_days = 20.0;
        assert!(c.mastery() > m1);
        c.interval_days = 400.0;
        assert_eq!(c.mastery(), 1.0);
    }

    fn card_due_at(phase: Phase, due: i64) -> Card {
        Card {
            phase,
            due,
            ..Card::default()
        }
    }

    #[test]
    fn calendar_buckets_by_rolling_24h_windows() {
        let now = 1_000_000;
        let mut store = SrsStore::default();
        let mut add = |id: &str, c: Card| {
            store.cards.insert(id.to_string(), c);
        };
        add("overdue", card_due_at(Phase::Review, now - 5 * DAY));
        add("due_exactly_now", card_due_at(Phase::Review, now));
        add("learning_now", card_due_at(Phase::Learning(0), now - 30));
        add("in_10_min", card_due_at(Phase::Learning(1), now + 600));
        add("edge_24h", card_due_at(Phase::Review, now + DAY));
        add("just_after_24h", card_due_at(Phase::Review, now + DAY + 1));
        add("in_2_days", card_due_at(Phase::Review, now + 2 * DAY + 100));
        add("in_6_days", card_due_at(Phase::Review, now + 6 * DAY));
        add("in_30_days", card_due_at(Phase::Review, now + 30 * DAY));
        add("never_seen", card_due_at(Phase::New, 0)); // New cards are not part of the calendar
        let cal = store.calendar(now);
        assert_eq!(cal.started, 9);
        assert_eq!(cal.due_now, 3);
        assert_eq!(cal.due_now_learning, 1);
        assert_eq!(cal.later_today, 2, "10 minutes ahead and exactly 24 h ahead");
        assert_eq!(cal.tomorrow, 1, "one second past 24 h");
        assert_eq!(cal.today(), 5);
        assert_eq!(cal.week[0], 5);
        assert_eq!(cal.week[1], 1);
        assert_eq!(cal.week[2], 1);
        assert_eq!(cal.week[5], 1, "6 days ahead falls in the sixth window");
        assert_eq!(cal.later, 1);
        assert_eq!(store.due_count(now), cal.due_now, "agrees with due_count");
    }

    #[test]
    fn empty_store_has_an_empty_calendar() {
        assert_eq!(SrsStore::default().calendar(123), ReviewCalendar::default());
    }

    #[test]
    fn pick_prefers_due_learning_then_new() {
        let mut store = SrsStore::default();
        let mut rng = Rng::new(1);
        store.review("a", Grade::Again, 0); // learning, due at 60
        let cands: Vec<String> = vec!["a".into(), "b".into()];
        assert_eq!(
            store.pick_next(&cands, 100, 50, None, &mut rng).as_deref(),
            Some("a")
        );
        // before it is due, the new card "b" is chosen
        assert_eq!(
            store.pick_next(&cands, 10, 50, None, &mut rng).as_deref(),
            Some("b")
        );
    }

    #[test]
    fn pick_avoids_repeat_when_possible() {
        let store = SrsStore::default();
        let mut rng = Rng::new(2);
        let cands: Vec<String> = vec!["a".into(), "b".into()];
        for _ in 0..20 {
            assert_eq!(
                store.pick_next(&cands, 0, 50, Some("a"), &mut rng).as_deref(),
                Some("b")
            );
        }
        let single: Vec<String> = vec!["a".into()];
        assert_eq!(
            store.pick_next(&single, 0, 50, Some("a"), &mut rng).as_deref(),
            Some("a")
        );
        assert_eq!(store.pick_next(&[], 0, 50, None, &mut rng), None);
    }

    fn grade_strategy() -> impl Strategy<Value = Grade> {
        prop_oneof![
            Just(Grade::Again),
            Just(Grade::Hard),
            Just(Grade::Good),
            Just(Grade::Easy)
        ]
    }

    proptest! {
        #[test]
        fn invariants_hold_for_any_review_sequence(
            seq in proptest::collection::vec((grade_strategy(), 0i64..(30 * DAY)), 1..60)
        ) {
            let mut c = Card::default();
            let mut now = 1_000_000i64;
            for (g, gap) in seq {
                now += gap;
                c.review(g, now);
                prop_assert!(c.ease >= MIN_EASE && c.ease <= MAX_EASE);
                prop_assert!(c.interval_days >= 0.0 && c.interval_days <= MAX_INTERVAL_DAYS);
                prop_assert!(c.due >= now);
                prop_assert!(c.correct <= c.seen);
                prop_assert!((0.0..=1.0).contains(&c.mastery()));
                if g == Grade::Again {
                    prop_assert_eq!(c.phase, Phase::Learning(0));
                    prop_assert!(c.due - now <= 600);
                }
            }
        }

        #[test]
        fn calendar_accounts_for_every_started_card(
            offsets in proptest::collection::vec(-3_000_000i64..40_000_000, 0..40),
            learning in proptest::collection::vec(proptest::bool::ANY, 0..40),
        ) {
            let now = 5_000_000;
            let mut store = SrsStore::default();
            for (i, off) in offsets.iter().enumerate() {
                let phase = if learning.get(i).copied().unwrap_or(false) { Phase::Learning(0) } else { Phase::Review };
                store.cards.insert(format!("c{i}"), Card { phase, due: now + off, ..Card::default() });
            }
            let cal = store.calendar(now);
            prop_assert_eq!(cal.started, offsets.len());
            prop_assert_eq!(cal.week.iter().sum::<usize>() + cal.later, cal.started);
            prop_assert_eq!(cal.due_now, store.due_count(now));
            prop_assert!(cal.due_now_learning <= cal.due_now);
            prop_assert_eq!(cal.week[0], cal.due_now + cal.later_today);
            prop_assert_eq!(cal.week[1], cal.tomorrow);
        }

        #[test]
        fn on_time_successes_never_shrink_the_interval(n in 1usize..25) {
            let mut c = Card::default();
            let mut now = 0i64;
            c.review(Grade::Good, now); now += 600;
            c.review(Grade::Good, now); // graduated
            let mut last = c.interval_days;
            for _ in 0..n {
                now = c.due; // review exactly on time
                c.review(Grade::Good, now);
                prop_assert!(c.interval_days >= last);
                last = c.interval_days;
            }
        }

        #[test]
        fn due_dates_move_forward_for_easy_over_hard(start in 1.0f32..100.0) {
            let base = Card { phase: Phase::Review, interval_days: start, due: 0, ..Card::default() };
            let mut easy = base.clone();
            let mut hard = base.clone();
            easy.review(Grade::Easy, 0);
            hard.review(Grade::Hard, 0);
            prop_assert!(easy.interval_days >= hard.interval_days);
        }
    }
}
