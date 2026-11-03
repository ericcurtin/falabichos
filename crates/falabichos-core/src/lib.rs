// SPDX-License-Identifier: Apache-2.0
//! Engine-agnostic game logic for Falabichos. No rendering, no I/O, no Bevy.
//!
//! * [`model`]: the content data model (deserialised from RON by `falabichos-content`)
//! * [`content`]: the in-memory content database and word lookup
//! * [`normalize`]: answer matching (accent-insensitive / typo-tolerant)
//! * [`srs`]: spaced-repetition scheduler
//! * [`exercise`]: exercise generation and checking
//! * [`ladder`]: the adaptive strictness ladder for typed answers
//! * [`battle`]: conversation-battle state machine (catch / challenge / practice)
//! * [`progress`]: story flags, party, Bichodex, seals
//! * [`save`]: versioned saves and settings with migrations
//! * [`i18n`]: UI string tables

pub mod battle;
pub mod content;
pub mod exercise;
pub mod i18n;
pub mod ladder;
pub mod model;
pub mod normalize;
pub mod progress;
pub mod rng;
pub mod save;
pub mod srs;

pub use content::Content;
pub use rng::Rng;
