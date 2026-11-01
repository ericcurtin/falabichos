// SPDX-License-Identifier: Apache-2.0
//! The content data model. Every type here is deserialised from RON files under `assets/content/`.
//!
//! Design rules (see `docs/CONTENT_FORMAT.md`):
//! * unknown fields are rejected, so typos in data files are caught by the linter;
//! * optional things have defaults, so a minimal entry stays short and diffs stay small;
//! * anything a native speaker should double-check carries `needs_native_review`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Grammatical gender of a noun.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Gender {
    Masc,
    Fem,
}

impl Gender {
    /// Definite article in pt-PT.
    pub fn definite(self) -> &'static str {
        match self {
            Gender::Masc => "o",
            Gender::Fem => "a",
        }
    }
    pub fn indefinite(self) -> &'static str {
        match self {
            Gender::Masc => "um",
            Gender::Fem => "uma",
        }
    }
}

/// Part of speech (coarse).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum WordKind {
    #[default]
    Noun,
    Verb,
    Adjective,
    Phrase,
    Other,
}

/// pt-PT pronunciation teaching points. Each maps to a tip in `pronunciation.ron`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PronTag {
    /// Nasal vowels: `ã`, `em`, `im`, `om`, `um`.
    NasalVowel,
    /// Nasal diphthongs: `ão`, `ões`, `ãe`, `ães`.
    NasalDiphthong,
    /// Unstressed `e` is reduced (`/ɨ/` or silent), unstressed `o` -> `/u/`, unstressed `a` -> `/ɐ/`.
    ReducedVowel,
    /// Word-final `-s`/`-z` and `s` before consonant sound like `/ʃ/` or `/ʒ/`.
    SibilantS,
    /// Stressed `a` before nasal consonants (`/ɐ/`), `ê`/`é`, `ô`/`ó` contrasts.
    OpenClosedVowel,
    /// `lh` = `/ʎ/`, `nh` = `/ɲ/`, `ch` = `/ʃ/`.
    Digraph,
    /// `r` (`/ʁ/` initial or `rr`) vs tap `/ɾ/`.
    RhoticR,
    /// `ç`, `c` before e/i.
    SoftC,
    /// Silent letters (`h`) and elided vowels.
    SilentLetter,
    /// Word-final `l` is a dark `/ɫ/`.
    DarkL,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Example {
    pub pt: String,
    pub en: String,
}

/// One vocabulary item (word or phrase).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VocabItem {
    /// Stable snake_case id. Never rename: save files refer to it.
    pub id: String,
    /// European Portuguese headword, without article (nouns). Post-1990 spelling.
    pub pt: String,
    /// English glosses. The first one is the primary translation.
    pub en: Vec<String>,
    /// Theme category, e.g. `cafe`, `transport`. Used for distractor selection.
    pub category: String,
    #[serde(default)]
    pub kind: WordKind,
    #[serde(default)]
    pub gender: Option<Gender>,
    /// Plural form if irregular or worth showing.
    #[serde(default)]
    pub plural: Option<String>,
    /// IPA for European Portuguese (broad transcription, no slashes).
    #[serde(default)]
    pub ipa: Option<String>,
    /// Clip name relative to `assets/audio/pt-PT/`, without extension (e.g. `cafe/bica`).
    #[serde(default)]
    pub audio: Option<String>,
    #[serde(default)]
    pub pron_tags: Vec<PronTag>,
    #[serde(default)]
    pub example: Option<Example>,
    /// Usage note (register, regional variation, pt-PT vs pt-BR).
    #[serde(default)]
    pub note: Option<String>,
    /// The Brazilian form learners should NOT use, if one commonly differs.
    #[serde(default)]
    pub not_pt_br: Option<String>,
    /// Extra surface forms (plurals, feminine, conjugations) used for word lookup in dialogue.
    #[serde(default)]
    pub forms: Vec<String>,
    /// 1 (beginner) to 5.
    #[serde(default = "one")]
    pub level: u8,
    #[serde(default)]
    pub needs_native_review: bool,
}

fn one() -> u8 {
    1
}

impl VocabItem {
    pub fn primary_en(&self) -> &str {
        self.en.first().map(String::as_str).unwrap_or("")
    }
    /// Headword with the definite article for nouns (`a casa`).
    pub fn with_article(&self) -> String {
        match (self.kind, self.gender) {
            (WordKind::Noun, Some(g)) => format!("{} {}", g.definite(), self.pt),
            _ => self.pt.clone(),
        }
    }
}

/// What kind of exercise a hand-written exercise is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExerciseKind {
    /// Pick one option for a prompt.
    Choice,
    /// Choose the word that fills the blank (`___`) in a sentence.
    FillBlank,
    /// Type the answer.
    Type,
    /// Put word tiles into the right order.
    Order,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Lang {
    #[default]
    Pt,
    En,
}

/// A hand-written exercise (used by grammar concepts and by Mestre tests).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExerciseDef {
    pub id: String,
    pub kind: ExerciseKind,
    /// Text shown to the player. For `FillBlank` use `___` for the gap.
    pub prompt: String,
    #[serde(default)]
    pub prompt_lang: Lang,
    /// Extra line shown under the prompt (usually the English meaning).
    #[serde(default)]
    pub context: Option<String>,
    /// Options for `Choice` / `FillBlank`.
    #[serde(default)]
    pub options: Vec<String>,
    /// Accepted answers. For choices these are the correct option texts; for `Order` the full
    /// correct sentence(s); for `Type` the accepted spellings. The first is shown as "the" answer.
    pub answer: Vec<String>,
    #[serde(default)]
    pub hint: Option<String>,
    /// Shown after answering (why it is right).
    #[serde(default)]
    pub explanation: Option<String>,
    #[serde(default)]
    pub needs_native_review: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleLine {
    pub pt: String,
    pub en: String,
}

/// A grammar concept: a notebook page plus its practice exercises.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrammarConcept {
    pub id: String,
    pub title: String,
    pub summary: String,
    /// Bullet points explaining the rule.
    #[serde(default)]
    pub rules: Vec<String>,
    #[serde(default)]
    pub examples: Vec<RuleLine>,
    /// pt-PT specific notes (vs Brazilian Portuguese).
    #[serde(default)]
    pub pt_pt_note: Option<String>,
    pub exercises: Vec<ExerciseDef>,
    #[serde(default)]
    pub needs_native_review: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SpeciesKind {
    /// Embodies vocabulary words.
    #[default]
    Vocab,
    /// Embodies a grammar concept.
    Grammar,
}

/// A Bicho species.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Species {
    pub id: String,
    pub name: String,
    /// Bichodex number.
    pub dex_no: u16,
    #[serde(default)]
    pub kind: SpeciesKind,
    /// Short English description of what the Bicho "means".
    pub theme: String,
    /// Portuguese flavour line (shown in the Bichodex with a gloss).
    pub flavor_pt: String,
    pub flavor_en: String,
    /// Vocab ids this Bicho embodies (for `Vocab` kind).
    #[serde(default)]
    pub vocab: Vec<String>,
    /// Grammar concept ids (for `Grammar` kind).
    #[serde(default)]
    pub grammar: Vec<String>,
    /// Art id in `assets/art/` (without extension).
    pub art: String,
    /// Correct answers needed to fill the trust meter.
    #[serde(default = "five")]
    pub trust_needed: u8,
    /// Sound pitch in Hz used for the cry (procedural SFX).
    #[serde(default = "default_cry")]
    pub cry_hz: u16,
    /// Primary and (optional) secondary palette letters for the Bichodex card background.
    #[serde(default)]
    pub needs_native_review: bool,
}

fn five() -> u8 {
    5
}
fn default_cry() -> u16 {
    440
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Dir {
    Up,
    #[default]
    Down,
    Left,
    Right,
}

impl Dir {
    pub fn delta(self) -> (i32, i32) {
        match self {
            Dir::Up => (0, -1),
            Dir::Down => (0, 1),
            Dir::Left => (-1, 0),
            Dir::Right => (1, 0),
        }
    }
    pub fn opposite(self) -> Dir {
        match self {
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
            Dir::Left => Dir::Right,
            Dir::Right => Dir::Left,
        }
    }
}

/// A condition on story progress. All conditions in a list must hold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cond {
    Flag(String),
    NotFlag(String),
    HasBicho(String),
    HasSeal(String),
    /// Party has at least this many Bichos.
    PartyAtLeast(u8),
}

/// Something that happens after a dialogue branch finishes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    SetFlag(String),
    ClearFlag(String),
    /// Restore full confidence (the player drinks a bica).
    Heal,
    /// Start a trainer/Mestre challenge by id.
    Challenge(String),
    /// Add a Bicho to the party directly (gifts).
    GiveBicho(String),
    Warp {
        map: String,
        x: i32,
        y: i32,
    },
    /// Save the game.
    Save,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Line {
    /// Speaker name; `None` for narration/sign text.
    #[serde(default)]
    pub who: Option<String>,
    pub pt: String,
    pub en: String,
    /// Hand-written word glosses for the lookup feature: lowercase word -> meaning.
    #[serde(default)]
    pub words: BTreeMap<String, String>,
    #[serde(default)]
    pub audio: Option<String>,
    /// `tu` / `formal` register marker for the learner (informational).
    #[serde(default)]
    pub register: Option<String>,
    #[serde(default)]
    pub needs_native_review: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Branch {
    #[serde(default)]
    pub when: Vec<Cond>,
    pub lines: Vec<Line>,
    #[serde(default)]
    pub then: Vec<Action>,
}

/// A dialogue: the first branch whose conditions hold is played.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dialogue {
    pub id: String,
    pub branches: Vec<Branch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ChallengeKind {
    #[default]
    Trainer,
    Mestre,
}

/// Where a challenge draws its exercises from.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pool {
    /// Vocab categories.
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub vocab: Vec<String>,
    /// Grammar concept ids (all their exercises are used).
    #[serde(default)]
    pub grammar: Vec<String>,
}

/// A trainer or Mestre test.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Challenge {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: ChallengeKind,
    /// What the test covers (shown on the intro card).
    pub theme: String,
    pub pool: Pool,
    /// Number of questions.
    pub questions: u8,
    /// Correct answers required to win.
    pub pass_correct: u8,
    /// Mistakes allowed before failing.
    pub confidence: u8,
    /// Art id of the opponent portrait sprite.
    pub art: String,
    /// Flag set when beaten.
    pub reward_flag: String,
    /// Seal awarded when beaten.
    #[serde(default)]
    pub seal: Option<String>,
    #[serde(default)]
    pub win_dialogue: Option<String>,
    #[serde(default)]
    pub lose_dialogue: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Seal {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Art id of the seal icon.
    pub art: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Warp {
    pub x: i32,
    pub y: i32,
    pub map: String,
    pub to_x: i32,
    pub to_y: i32,
    #[serde(default)]
    pub facing: Option<Dir>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcTrainer {
    /// Tiles of line-of-sight.
    pub sight: u8,
    pub challenge: String,
    /// Dialogue played when the trainer spots the player.
    pub spot_dialogue: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Npc {
    pub id: String,
    pub x: i32,
    pub y: i32,
    /// Art id of the sprite sheet.
    pub art: String,
    #[serde(default)]
    pub facing: Dir,
    #[serde(default)]
    pub dialogue: Option<String>,
    /// Only present while all conditions hold.
    #[serde(default)]
    pub show_when: Vec<Cond>,
    #[serde(default)]
    pub trainer: Option<NpcTrainer>,
    /// Wanders randomly within 2 tiles.
    #[serde(default)]
    pub wander: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sign {
    pub x: i32,
    pub y: i32,
    pub dialogue: String,
}

/// A trigger tile that plays a dialogue/actions once when stepped on (story beats, gates).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trigger {
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub when: Vec<Cond>,
    pub dialogue: String,
    /// If true the player is pushed one tile back in the opposite direction afterwards (a "gate").
    #[serde(default)]
    pub push_back: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterEntry {
    pub species: String,
    /// Relative weight.
    pub weight: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapDef {
    pub id: String,
    pub name: String,
    /// Music track id (see `assets/music`). Optional: silence.
    #[serde(default)]
    pub music: Option<String>,
    #[serde(default)]
    pub indoor: bool,
    /// Rows of tile characters (see `tiles.ron`). All rows must have equal length.
    pub rows: Vec<String>,
    #[serde(default)]
    pub warps: Vec<Warp>,
    #[serde(default)]
    pub npcs: Vec<Npc>,
    #[serde(default)]
    pub signs: Vec<Sign>,
    #[serde(default)]
    pub triggers: Vec<Trigger>,
    /// Wild Bichos found in tall grass on this map.
    #[serde(default)]
    pub encounters: Vec<EncounterEntry>,
    /// Chance (percent) of an encounter per step on a grass tile.
    #[serde(default = "default_rate")]
    pub encounter_rate: u8,
    /// Town this map belongs to (for the town-name banner).
    #[serde(default)]
    pub town: Option<String>,
}

fn default_rate() -> u8 {
    12
}

impl MapDef {
    pub fn width(&self) -> usize {
        self.rows.first().map(|r| r.chars().count()).unwrap_or(0)
    }
    pub fn height(&self) -> usize {
        self.rows.len()
    }
    pub fn tile_char(&self, x: i32, y: i32) -> Option<char> {
        if x < 0 || y < 0 {
            return None;
        }
        self.rows.get(y as usize)?.chars().nth(x as usize)
    }
}

/// How a tile behaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TileKind {
    #[default]
    Floor,
    Solid,
    /// Wild encounters can trigger here.
    TallGrass,
    /// Blocks walking (impassable), drawn animated.
    Water,
    /// Counters: solid, but can be talked across.
    Counter,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TileDef {
    /// The character used in map rows.
    pub ch: char,
    pub name: String,
    /// Art ids of the animation frames (`assets/art/tiles/<id>.px`). One entry = static.
    pub art: Vec<String>,
    #[serde(default)]
    pub kind: TileKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Town {
    pub id: String,
    pub name: String,
    pub blurb: String,
    /// Inspired by (real region), for documentation.
    pub inspired_by: String,
    /// Map the player arrives on.
    pub start_map: String,
    #[serde(default)]
    pub needs_native_review: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PronTip {
    pub tag: PronTag,
    pub title: String,
    pub tip: String,
    /// Example words (pt) to try.
    #[serde(default)]
    pub examples: Vec<String>,
}

/// Story start position and metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Story {
    pub start_map: String,
    pub start_x: i32,
    pub start_y: i32,
    pub intro_dialogue: String,
    /// Flag that marks the end of the vertical slice.
    pub finale_flag: String,
    pub finale_dialogue: String,
}
