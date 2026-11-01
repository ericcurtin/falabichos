// SPDX-License-Identifier: Apache-2.0
//! Story progress, party and Bichodex, badge ("Selo") rules.

use crate::content::Content;
use crate::model::{Action, Cond, Dialogue};
use crate::srs::SrsStore;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyBicho {
    pub species: String,
    /// Unix seconds when the Bicho joined.
    pub caught_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Progress {
    pub flags: BTreeSet<String>,
    pub seals: BTreeSet<String>,
    /// Species the player has met (Bichodex "seen").
    pub dex_seen: BTreeSet<String>,
    pub party: Vec<PartyBicho>,
}

/// Something the game layer must do after actions are applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Heal,
    Challenge(String),
    Warp { map: String, x: i32, y: i32 },
    Save,
}

impl Progress {
    pub fn has_flag(&self, f: &str) -> bool {
        self.flags.contains(f)
    }
    pub fn has_bicho(&self, species: &str) -> bool {
        self.party.iter().any(|b| b.species == species)
    }
    pub fn eval(&self, conds: &[Cond]) -> bool {
        conds.iter().all(|c| match c {
            Cond::Flag(f) => self.flags.contains(f),
            Cond::NotFlag(f) => !self.flags.contains(f),
            Cond::HasBicho(s) => self.has_bicho(s),
            Cond::HasSeal(s) => self.seals.contains(s),
            Cond::PartyAtLeast(n) => self.party.len() >= *n as usize,
        })
    }

    pub fn see(&mut self, species: &str) {
        self.dex_seen.insert(species.to_string());
    }

    /// Add a Bicho to the party (once per species). Returns true if it was new.
    pub fn add_bicho(&mut self, species: &str, now: i64) -> bool {
        self.dex_seen.insert(species.to_string());
        if self.has_bicho(species) {
            return false;
        }
        self.party.push(PartyBicho { species: species.to_string(), caught_at: now });
        true
    }

    pub fn award_seal(&mut self, seal: &str) -> bool {
        self.seals.insert(seal.to_string())
    }

    /// Apply dialogue actions; world-affecting ones come back as [`Effect`]s.
    pub fn apply(&mut self, actions: &[Action], now: i64) -> Vec<Effect> {
        let mut fx = vec![];
        for a in actions {
            match a {
                Action::SetFlag(f) => {
                    self.flags.insert(f.clone());
                }
                Action::ClearFlag(f) => {
                    self.flags.remove(f);
                }
                Action::GiveBicho(s) => {
                    self.add_bicho(s, now);
                }
                Action::Heal => fx.push(Effect::Heal),
                Action::Challenge(c) => fx.push(Effect::Challenge(c.clone())),
                Action::Warp { map, x, y } => fx.push(Effect::Warp { map: map.clone(), x: *x, y: *y }),
                Action::Save => fx.push(Effect::Save),
            }
        }
        fx
    }

    /// Dex counts: (seen, caught).
    pub fn dex_counts(&self) -> (usize, usize) {
        (self.dex_seen.len(), self.party.len())
    }
}

impl Dialogue {
    /// The first branch whose conditions hold.
    pub fn resolve(&self, progress: &Progress) -> Option<&crate::model::Branch> {
        self.branches.iter().find(|b| progress.eval(&b.when))
    }
}

/// Mastery of a species in `0.0..=1.0`: the mean mastery of its skills.
pub fn species_mastery(content: &Content, srs: &SrsStore, species: &str) -> f32 {
    let skills = content.species_skills(species);
    if skills.is_empty() {
        return 0.0;
    }
    skills.iter().map(|s| srs.mastery(s)).sum::<f32>() / skills.len() as f32
}

/// Bicho level 1..=10, driven entirely by SRS mastery of its words.
pub fn bicho_level(content: &Content, srs: &SrsStore, species: &str) -> u8 {
    let m = species_mastery(content, srs, species);
    (1.0 + (m * 9.0).floor()).clamp(1.0, 10.0) as u8
}

/// (words started, total words) for a species.
pub fn species_learned(content: &Content, srs: &SrsStore, species: &str) -> (usize, usize) {
    let skills = content.species_skills(species);
    let learned = skills
        .iter()
        .filter(|s| srs.card(s).map(|c| c.phase != crate::srs::Phase::New).unwrap_or(false))
        .count();
    (learned, skills.len())
}

/// Battle bonuses from the party: (extra confidence, hints).
pub fn party_bonus(content: &Content, srs: &SrsStore, progress: &Progress) -> (u8, u8) {
    let n = progress.party.len().min(3) as u8;
    let best = progress.party.iter().map(|b| bicho_level(content, srs, &b.species)).max().unwrap_or(0);
    (n + best / 4, 1 + n.min(2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_and_conditions() {
        let mut p = Progress::default();
        assert!(p.eval(&[]));
        assert!(p.eval(&[Cond::NotFlag("x".into())]));
        p.apply(&[Action::SetFlag("x".into())], 0);
        assert!(p.eval(&[Cond::Flag("x".into()), Cond::PartyAtLeast(0)]));
        assert!(!p.eval(&[Cond::PartyAtLeast(1)]));
        p.add_bicho("galo", 5);
        assert!(p.eval(&[Cond::HasBicho("galo".into()), Cond::PartyAtLeast(1)]));
        assert!(!p.add_bicho("galo", 6), "no duplicates");
        assert_eq!(p.dex_counts(), (1, 1));
    }

    #[test]
    fn effects_are_returned() {
        let mut p = Progress::default();
        let fx = p.apply(&[Action::Heal, Action::Challenge("c".into()), Action::Save], 0);
        assert_eq!(fx, vec![Effect::Heal, Effect::Challenge("c".into()), Effect::Save]);
    }

    #[test]
    fn seals_award_once() {
        let mut p = Progress::default();
        assert!(p.award_seal("a"));
        assert!(!p.award_seal("a"));
    }
}
