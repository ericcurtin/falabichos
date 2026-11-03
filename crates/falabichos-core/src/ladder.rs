// SPDX-License-Identifier: Apache-2.0
//! The adaptive strictness ladder: how forgiving typed answers are, per learner.
//!
//! The two answer-forgiveness settings (missing accents, small typos) form a three-rung ladder:
//!
//! | rung | accents forgiven | small typos forgiven |
//! |------|------------------|----------------------|
//! | 0    | yes              | yes                  |
//! | 1    | yes              | no                   |
//! | 2    | no               | no                   |
//!
//! With *adaptive strictness* switched on, the learner climbs a rung after a clean run (almost every
//! recent typed answer exactly right) and slides back after a poor one. The ladder can only make the
//! rules **stricter than the player's own settings**, never more forgiving: a player who turned
//! forgiveness off keeps it off. Only typed answers count.

use crate::normalize::{AnswerPolicy, Verdict};
use serde::{Deserialize, Serialize};

/// Typed answers looked at before the rung can change.
pub const WINDOW: usize = 12;
/// Exactly-right answers (no accent or typo slip) in a full window needed to climb a rung.
pub const CLIMB_EXACT: usize = 11;
/// Fewer right answers than this in a full window (under the current rules) slides back a rung.
pub const SLIDE_BELOW: usize = 6;
/// The strictest rung.
pub const TOP_RUNG: u8 = 2;

/// How one typed answer went.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    /// Right with no slip.
    Exact,
    /// Right, but only because a missing accent or a small typo was forgiven.
    Lenient,
    Wrong,
}

impl Outcome {
    pub fn from_verdict(v: &Verdict) -> Outcome {
        match v {
            Verdict::Exact => Outcome::Exact,
            Verdict::AccentSlip { .. } | Verdict::Typo { .. } => Outcome::Lenient,
            Verdict::Wrong => Outcome::Wrong,
        }
    }
}

/// A rung change, so the game can tell the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// Moved up to this rung.
    Up(u8),
    /// Moved down to this rung.
    Down(u8),
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Ladder {
    /// 0 (most forgiving) to [`TOP_RUNG`].
    pub rung: u8,
    /// The most recent typed answers (at most [`WINDOW`]).
    pub recent: Vec<Outcome>,
}

impl Ladder {
    /// Record a typed answer; returns the rung change it caused, if any.
    pub fn record(&mut self, outcome: Outcome) -> Option<Change> {
        self.recent.push(outcome);
        if self.recent.len() > WINDOW {
            self.recent.remove(0);
        }
        if self.recent.len() < WINDOW {
            return None;
        }
        let exact = self.recent.iter().filter(|o| **o == Outcome::Exact).count();
        let right = self.recent.iter().filter(|o| **o != Outcome::Wrong).count();
        if self.rung < TOP_RUNG && exact >= CLIMB_EXACT {
            self.rung += 1;
            self.recent.clear();
            Some(Change::Up(self.rung))
        } else if self.rung > 0 && right < SLIDE_BELOW {
            self.rung -= 1;
            self.recent.clear();
            Some(Change::Down(self.rung))
        } else {
            None
        }
    }

    /// The answer policy this rung allows, never more forgiving than `base` (the player's settings).
    pub fn apply(&self, base: AnswerPolicy) -> AnswerPolicy {
        AnswerPolicy {
            accent_insensitive: base.accent_insensitive && self.rung <= 1,
            typo_tolerance: base.typo_tolerance && self.rung == 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LENIENT: AnswerPolicy = AnswerPolicy {
        accent_insensitive: true,
        typo_tolerance: true,
    };

    fn feed(l: &mut Ladder, o: Outcome, n: usize) -> Vec<Change> {
        (0..n).filter_map(|_| l.record(o)).collect()
    }

    #[test]
    fn starts_forgiving() {
        let l = Ladder::default();
        assert_eq!(l.apply(LENIENT), LENIENT);
    }

    #[test]
    fn nothing_changes_before_a_full_window() {
        let mut l = Ladder::default();
        assert!(feed(&mut l, Outcome::Exact, WINDOW - 1).is_empty());
        assert_eq!(l.rung, 0);
        assert_eq!(l.record(Outcome::Exact), Some(Change::Up(1)));
    }

    #[test]
    fn a_clean_run_climbs_and_the_window_restarts() {
        let mut l = Ladder::default();
        assert_eq!(feed(&mut l, Outcome::Exact, WINDOW), vec![Change::Up(1)]);
        assert!(l.recent.is_empty(), "a new rung starts with a fresh window");
        assert_eq!(
            l.apply(LENIENT),
            AnswerPolicy {
                accent_insensitive: true,
                typo_tolerance: false
            }
        );
        assert_eq!(feed(&mut l, Outcome::Exact, WINDOW), vec![Change::Up(2)]);
        assert_eq!(
            l.apply(LENIENT),
            AnswerPolicy {
                accent_insensitive: false,
                typo_tolerance: false
            }
        );
        // the top rung is the end of the ladder
        assert!(feed(&mut l, Outcome::Exact, 3 * WINDOW).is_empty());
        assert_eq!(l.rung, TOP_RUNG);
    }

    #[test]
    fn leniency_is_not_a_clean_run() {
        let mut l = Ladder::default();
        // 10 exact + 2 lenient is below the climb mark (11 exact)
        for i in 0..WINDOW {
            let o = if i < 2 { Outcome::Lenient } else { Outcome::Exact };
            assert_eq!(l.record(o), None);
        }
        assert_eq!(l.rung, 0);
    }

    #[test]
    fn a_poor_run_slides_back_but_never_below_zero() {
        let mut l = Ladder {
            rung: 2,
            recent: vec![],
        };
        assert_eq!(feed(&mut l, Outcome::Wrong, WINDOW), vec![Change::Down(1)]);
        assert_eq!(feed(&mut l, Outcome::Wrong, WINDOW), vec![Change::Down(0)]);
        assert!(feed(&mut l, Outcome::Wrong, 3 * WINDOW).is_empty());
        assert_eq!(l.rung, 0);
    }

    #[test]
    fn half_right_is_enough_to_stay_put() {
        let mut l = Ladder {
            rung: 1,
            recent: vec![],
        };
        for i in 0..WINDOW {
            let o = if i % 2 == 0 {
                Outcome::Lenient
            } else {
                Outcome::Wrong
            };
            assert_eq!(l.record(o), None, "answer {i}");
        }
        assert_eq!(l.rung, 1);
    }

    #[test]
    fn the_window_slides_over_old_answers() {
        let mut l = Ladder::default();
        feed(&mut l, Outcome::Wrong, WINDOW - 1);
        // a long clean streak pushes the old mistakes out of the window
        let changes = feed(&mut l, Outcome::Exact, WINDOW + 5);
        assert_eq!(changes, vec![Change::Up(1)]);
        assert!(l.recent.len() <= WINDOW);
    }

    #[test]
    fn it_never_forgives_more_than_the_player_chose() {
        let strict = AnswerPolicy {
            accent_insensitive: false,
            typo_tolerance: false,
        };
        let no_typos = AnswerPolicy {
            accent_insensitive: true,
            typo_tolerance: false,
        };
        for rung in 0..=TOP_RUNG {
            let l = Ladder { rung, recent: vec![] };
            assert_eq!(l.apply(strict), strict);
            let p = l.apply(no_typos);
            assert!(!p.typo_tolerance);
            assert_eq!(p.accent_insensitive, rung <= 1);
        }
    }

    #[test]
    fn verdicts_map_to_outcomes() {
        assert_eq!(Outcome::from_verdict(&Verdict::Exact), Outcome::Exact);
        assert_eq!(
            Outcome::from_verdict(&Verdict::AccentSlip {
                expected: "pão".into()
            }),
            Outcome::Lenient
        );
        assert_eq!(
            Outcome::from_verdict(&Verdict::Typo {
                expected: "falam".into()
            }),
            Outcome::Lenient
        );
        assert_eq!(Outcome::from_verdict(&Verdict::Wrong), Outcome::Wrong);
    }

    #[test]
    fn serialises_and_reads_back() {
        let l = Ladder {
            rung: 1,
            recent: vec![Outcome::Exact, Outcome::Lenient, Outcome::Wrong],
        };
        let text = ron::ser::to_string(&l).unwrap();
        assert_eq!(ron::de::from_str::<Ladder>(&text).unwrap(), l);
    }
}
