# Architecture

## Goals

1. **Contributor-friendly**: all learning content is plain data; Portuguese speakers and teachers never touch Rust.
2. **Testable without a display**: the whole game runs headlessly, including a scripted playthrough.
3. **Five platforms, one code path**: Linux, macOS, Windows, Android, iOS. Native only.
4. **Engine as a thin shell**: [Bevy](https://bevy.org) (pinned `=0.19.1`) provides windowing, GPU, audio and
   input; the game does not depend on it.

## Crate graph

```
                       assets/  (RON, .px, .song, .lang)
                          |
                  falabichos-assets    embeds assets/ with include_bytes! (build.rs); reads a directory for mods
                   /       |       \
 falabichos-content   falabichos-media      falabichos-core
 (load + validate)    (art, font, canvas,   (model, SRS, exercises, battle,
        \              synth)                progress, saves, i18n)  <- no engine, no I/O
         \               |               /
          +---------- falabichos-game ---+     scenes, world, dialogue, battle UI, menus, sim
                         |
                    falabichos (Bevy)           window, input, audio, mobile entry points
```

`falabichos-tools` (lint, mapcheck, preview, new) and `xtask` sit beside this graph.

Why is there a `falabichos-game` crate in between? The original brief asks for an engine-agnostic core and
a Bevy client. Putting *scene logic and drawing* in an engine-agnostic crate as well means the headless
simulator and the screenshots exercise the real game, and the Bevy crate stays small and easy to port to a
newer Bevy. The trade-off: we use Bevy's ECS very little (it owns the window and the audio, not the game).

## The frame loop

```
Bevy FixedUpdate @60 Hz:  Input (keyboard/gamepad/touch)  ->  Game::tick(&Input)
Bevy Update:              Game::draw(&mut Canvas 240x160) ->  upload RGBA to one Image  ->  Sprite
                          Game::take_events() -> Sfx / Music / Clip / Cry / Fullscreen / Quit / Saved
```

- The sprite is scaled by an **integer** number of physical pixels (`display::frame_scale`) with nearest
  sampling and black letterboxing; the camera never changes.
- Input is *held-state*; `PadTracker` turns it into edges and d-pad auto-repeat inside the game.
  Between-tick taps are latched in the shell so fast presses are not lost.
- `Game` receives time from an injected clock and randomness from a seeded `Rng`.

## Game state

`Game { scene, world, save: SaveGame, rng, settings, ... }`

- `SaveGame` = player position, `Progress` (flags, seals, Bichodex seen, party), `SrsStore` (all SRS cards by skill id),
  RNG state, steps, play time, carried confidence. It is serialised as pretty RON with a `version` field.
- `Scene` is an enum: `Title`, `World` (with an optional dialogue overlay), `Menu`, `Battle`, `Dex`
  (Bichodex and party), `Notebook`, `Seals`, `Calendar` (review calendar), `Lesson` (mini-lesson before a grammar Bicho's first battle), `Settings`. Each tick takes the scene out of `Game`, updates
  it, and puts the next scene back, which keeps the borrow checker happy and transitions explicit.

## Learning engine (`falabichos-core`)

### Skills and exercises

A *skill* is the unit that the SRS schedules: a vocabulary id (`cafe_bica`) or a grammar exercise id (`se_3`).
`exercise::generate` turns a skill into an `Exercise`:

- **Vocabulary** skills pick a *flavour* from a difficulty ladder driven by the card (how often seen, graduated or
  not): new cards get multiple choice (pt→en, en→pt); known cards get the article/gender question, an IPA
  reading ("which word has this pronunciation?"), word ordering from the example sentence, and finally **typed
  answers**. Distractors come from the same category first.
- **Grammar** skills are hand-written exercises (`Choice`, `FillBlank`, `Type`, `Order`) from `grammar/*.ron`.
- Listening exercises use a real clip when `audio` exists, otherwise degrade to the IPA reading. **Dictation** (play the
  clip, type the word) is only generated when a real clip exists; without one it would just be copying the IPA.
- **Translation** exercises (type a whole sentence from its English meaning) come from hand-written `Translate`
  exercises or, at the advanced stage, from vocabulary example sentences. Per-word hints (`english=português`) are
  derived from the vocabulary (`exercise::word_hints_for`) or written by hand (`word_hints`).

### Answer checking (`normalize`)

Answers are lower-cased, punctuation-stripped and compared. By default (`AnswerPolicy`) a missing accent is
accepted with a note ("watch the accents: pão") and small typos are accepted for words of 5+ letters
(1 slip for 5-8 letters, 2 for longer; **never** for short words or for grammar exercises, so `falas` is not accepted for
`falam`). Both policies are settings. Mastery-wise, a typo or accent slip grades as *Hard*.

### Spaced repetition (`srs`)

An SM-2 derivative with Anki-style learning steps:

- New/lapsed cards go through **learning steps** of 1 and 10 minutes, so a failed item returns within the session.
- Graduated cards use days with an **ease factor** (1.3-3.0): Hard ×1.2, Good ×ease, Easy ×ease×1.3, max 365 days.
- A lapse sends the card back to learning with a quartered interval and lower ease.
- **Early reviews** (battles constantly show cards before they are due) never grow the interval, so players cannot
  "farm" intervals; failures always count.
- `pick_next` prefers due learning cards, then due reviews, mixes in new cards (40%), and avoids repeating the
  previous skill.
- Invariants are covered by property tests: ease stays in range, intervals bounded, due dates never in the past after
  a review, failures return within ten minutes, on-time successes never shrink an interval.

A Bicho's **level (1-10) is derived** from the mean SRS mastery of its skills; nothing is stored, so it can never
disagree with the scheduler. Party size and level give battles more confidence and hints.

### Battles (`battle`)

A pure state machine. *Wild*: correct answers fill trust (a streak of three gives a bonus), wrong answers cost
confidence, full trust catches the Bicho, empty confidence makes it flee (and, if you are out of confidence, sends you
home to Grandma). *Challenge* (trainer / Mestre): N questions, a pass mark and a mistake budget, no hints; a win sets a flag and
awards a Selo. *Practice*: reviews your party's started words with no penalty.

### Saves (`save`)

RON with a leading `version`. `load` probes the version, migrates old formats (the v0 prototype format is kept in a
`legacy` module with a test) and rejects *newer* versions rather than guessing. The client writes atomically and keeps
`save.ron.bak`. Settings are a separate file and tolerate missing fields (`#[serde(default)]`).

## Content pipeline

`assets/` is embedded at compile time by `falabichos-assets/build.rs`, so mobile builds need no file access;
`FALABICHOS_ASSETS=/path` or `--assets` overrides it for modding. `falabichos-content::load` parses every RON file
by path, then `validate` checks cross references (every id, warp target, dialogue, species, seal), tile legends,
map rectangles, walkability of NPCs and arrivals, and BFS reachability of every warp and NPC. `falabichos-tools`
adds art, font-coverage, music and UI-string checks.

## Rendering and art (`falabichos-media`)

- `Palette` (31 colours), `.px` text sprites (frames, `outline:`, `repeat:` patterns, `bob:`), recolourable
  character templates (`art/chars.txt`), optional PNG overrides.
- `Canvas`: RGBA software canvas with sprite blits at integer scale, alpha, fades, PNG export.
- `Font`: 5x7 bitmap glyphs in `art/font.txt`, accents composed in code (á à â ã é ê í ó ô õ ú ç ü, combining marks),
  IPA glyphs (ɐ ɛ ɔ ə ɨ ʃ ʒ ʁ ɾ ɲ ʎ ŋ ɫ ˈ ː and nasal vowels via combining tilde).
- `synth`: text-defined chiptune (square / pulse / triangle / saw / noise channels, pitch slides) rendered to 16-bit
  PCM and wrapped as WAV; used at runtime for music, SFX and Bicho cries.

## Mobile

`crates/falabichos` is a `rlib` + `cdylib` + `staticlib`. Android: `#[bevy_main]` generates `android_main` (GameActivity),
cargo-ndk puts `libfalabichos.so` into the Gradle project. iOS: `main_rs` is called from the Xcode project's
`main.m`; a Run Script phase builds the static library. Lifecycle: `AppLifecycle::WillSuspend/Suspended`
→ `Game::on_suspend()` (autosave when safe). Touch controls are sprites laid out by `TouchLayout` (landscape-first,
safe-area aware); a tap elsewhere on the screen acts as "A" (tap to advance dialogue).

## Decisions worth knowing

- **Bevy 0.19.1, pinned**: the newest release that builds on the pinned Rust 1.95 (0.20 needs a newer compiler).
- **Software rendering of the logical frame**: gives pixel-perfect output on every platform, deterministic
  screenshots, and a testable game. A GPU renderer is not needed for a 240x160 frame.
- **Own bitmap font**: no third-party font licence, perfect diacritics and IPA, crisp at any DPI.
- **Embedded assets**: one self-contained executable on desktop; no file permissions needed on mobile.
