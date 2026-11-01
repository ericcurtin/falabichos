// SPDX-License-Identifier: Apache-2.0
//! Versioned save games and settings, with migrations from older formats.
//!
//! Format: RON text with a leading `version` field. To change the format: bump
//! [`SAVE_VERSION`], keep the old struct under `legacy`, and add a step to [`load`].

use crate::model::Dir;
use crate::progress::Progress;
use crate::srs::SrsStore;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const SAVE_VERSION: u32 = 1;
pub const SETTINGS_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("could not parse save data: {0}")]
    Parse(String),
    #[error("save was written by a newer game version (format {found}, this build understands up to {supported})")]
    TooNew { found: u32, supported: u32 },
    #[error("could not serialise: {0}")]
    Serialise(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerState {
    pub map: String,
    pub x: i32,
    pub y: i32,
    pub facing: Dir,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveGame {
    pub version: u32,
    /// Unix seconds of the save.
    pub saved_at: i64,
    pub play_secs: u64,
    pub player: PlayerState,
    pub progress: Progress,
    pub srs: SrsStore,
    pub rng_state: u64,
    pub steps: u64,
}

impl SaveGame {
    pub fn new(player: PlayerState, now: i64) -> SaveGame {
        SaveGame {
            version: SAVE_VERSION,
            saved_at: now,
            play_secs: 0,
            player,
            progress: Progress::default(),
            srs: SrsStore::default(),
            rng_state: now as u64 ^ 0xFA1A_B1C0,
            steps: 0,
        }
    }

    pub fn to_ron(&self) -> Result<String, SaveError> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default().indentor("  ".to_string()))
            .map_err(|e| SaveError::Serialise(e.to_string()))
    }
}

/// Old on-disk formats, kept only so they can be migrated.
mod legacy {
    use serde::Deserialize;

    /// Version 0: the pre-release prototype format (no SRS, no seals, flat fields).
    #[derive(Deserialize)]
    pub struct V0 {
        pub map: String,
        pub x: i32,
        pub y: i32,
        #[serde(default)]
        pub caught: Vec<String>,
        #[serde(default)]
        pub flags: Vec<String>,
    }
}

#[derive(Deserialize)]
struct VersionProbe {
    #[serde(default)]
    version: u32,
}

/// Parse a save, migrating older versions to the current one.
pub fn load(text: &str) -> Result<SaveGame, SaveError> {
    let probe: VersionProbe = ron::de::from_str(text).map_err(|e| SaveError::Parse(e.to_string()))?;
    match probe.version {
        v if v > SAVE_VERSION => Err(SaveError::TooNew { found: v, supported: SAVE_VERSION }),
        0 => {
            let old: legacy::V0 = ron::de::from_str(text).map_err(|e| SaveError::Parse(e.to_string()))?;
            Ok(migrate_v0(old))
        }
        _ => ron::de::from_str(text).map_err(|e| SaveError::Parse(e.to_string())),
    }
}

fn migrate_v0(old: legacy::V0) -> SaveGame {
    let mut s = SaveGame::new(PlayerState { map: old.map, x: old.x, y: old.y, facing: Dir::Down }, 0);
    for sp in old.caught {
        s.progress.add_bicho(&sp, 0);
    }
    s.progress.flags = old.flags.into_iter().collect();
    s
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextSize {
    Normal,
    Large,
}

/// Player settings, saved separately from the game so they survive "New game".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub text_size: TextSize,
    pub high_contrast: bool,
    pub reduce_motion: bool,
    /// 0..=10
    pub music_volume: u8,
    pub sfx_volume: u8,
    /// Show English gloss under every dialogue line by default.
    pub gloss_default: bool,
    pub accent_insensitive: bool,
    pub typo_tolerance: bool,
    pub fullscreen: bool,
    /// UI language code (folder in `assets/i18n`).
    pub ui_lang: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            version: SETTINGS_VERSION,
            text_size: TextSize::Normal,
            high_contrast: false,
            reduce_motion: false,
            music_volume: 6,
            sfx_volume: 8,
            gloss_default: false,
            accent_insensitive: true,
            typo_tolerance: true,
            fullscreen: false,
            ui_lang: "en".to_string(),
        }
    }
}

impl Settings {
    pub fn to_ron(&self) -> Result<String, SaveError> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default().indentor("  ".to_string()))
            .map_err(|e| SaveError::Serialise(e.to_string()))
    }

    /// Parse settings; fields missing from older files fall back to defaults.
    pub fn load(text: &str) -> Result<Settings, SaveError> {
        let mut s: Settings = ron::de::from_str(text).map_err(|e| SaveError::Parse(e.to_string()))?;
        if s.version > SETTINGS_VERSION {
            return Err(SaveError::TooNew { found: s.version, supported: SETTINGS_VERSION });
        }
        s.version = SETTINGS_VERSION;
        s.music_volume = s.music_volume.min(10);
        s.sfx_volume = s.sfx_volume.min(10);
        Ok(s)
    }

    pub fn policy(&self) -> crate::normalize::AnswerPolicy {
        crate::normalize::AnswerPolicy {
            accent_insensitive: self.accent_insensitive,
            typo_tolerance: self.typo_tolerance,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::srs::Grade;

    fn sample() -> SaveGame {
        let mut s = SaveGame::new(PlayerState { map: "vila".into(), x: 3, y: 4, facing: Dir::Left }, 1234);
        s.progress.add_bicho("galo", 99);
        s.progress.flags.insert("met_avo".into());
        s.progress.award_seal("selo");
        s.srs.review("galo_1", Grade::Good, 100);
        s.srs.review("galo_2", Grade::Again, 100);
        s.steps = 42;
        s
    }

    #[test]
    fn roundtrip() {
        let s = sample();
        let text = s.to_ron().unwrap();
        let back = load(&text).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn migrates_v0() {
        let text = r#"(map: "vila", x: 2, y: 5, caught: ["galo", "sardinha"], flags: ["a"])"#;
        let s = load(text).unwrap();
        assert_eq!(s.version, SAVE_VERSION);
        assert_eq!(s.player.map, "vila");
        assert_eq!((s.player.x, s.player.y), (2, 5));
        assert!(s.progress.has_bicho("sardinha"));
        assert!(s.progress.has_flag("a"));
        // And a migrated save can be written and read back as current.
        assert_eq!(load(&s.to_ron().unwrap()).unwrap(), s);
    }

    #[test]
    fn rejects_future_versions_and_garbage() {
        assert!(matches!(load("(version: 99)"), Err(SaveError::TooNew { found: 99, .. })));
        assert!(matches!(load("not ron at all {{{"), Err(SaveError::Parse(_))));
    }

    #[test]
    fn settings_defaults_fill_missing_fields() {
        let s = Settings::load("(high_contrast: true)").unwrap();
        assert!(s.high_contrast);
        assert_eq!(s.music_volume, 6);
        let rt = Settings::load(&s.to_ron().unwrap()).unwrap();
        assert_eq!(rt, s);
        let loud = Settings::load("(music_volume: 99)").unwrap();
        assert_eq!(loud.music_volume, 10);
    }
}
