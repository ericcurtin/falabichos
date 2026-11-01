# Contributing to Falabichos

Thank you for helping people learn European Portuguese! This project is built **by** its community **for**
learners, and almost every kind of skill is useful. You do **not** need to know Rust.

Be kind: everyone here follows the [Code of Conduct](CODE_OF_CONDUCT.md).

## Pick your track

| I... | You can | Start here |
|------|---------|------------|
| **speak Portuguese** (especially from Portugal) | review and fix vocab, IPA, dialogue, grammar notes; write new content | [Track 1](#track-1-i-speak-portuguese) |
| am a **pixel artist** | draw tiles, characters, Bichos, UI | [Track 2](#track-2-im-a-pixel-artist) |
| am a **musician or voice actor** | compose chiptune loops, record pronunciation clips | [Track 3](#track-3-im-a-musician-or-voice-actor) |
| am a **Rust developer** | fix bugs, build features, improve tools and mobile support | [Track 4](#track-4-im-a-rust-dev) |
| am a **teacher or curriculum designer** | design lessons, exercises and the learning path | [Track 5](#track-5-im-a-teacher-or-curriculum-designer) |
| want the **game in my language** | translate the interface | [docs/LOCALISATION.md](docs/LOCALISATION.md) |

Small, well-scoped starter tasks for every track: [docs/GOOD_FIRST_ISSUES.md](docs/GOOD_FIRST_ISSUES.md).

## The rules that keep the project healthy

1. **Licence.** The whole project, code *and* content, text, art and audio data, is under the
   **Apache License 2.0 only**. By submitting a contribution you agree that, per **section 5 of the
   Apache License 2.0**, it is licensed under those terms with no additional conditions. Do not submit
   anything you do not have the right to license that way.
2. **Original work only.** No sprites, music, maps, names or trademarks from any commercial game
   franchise (this is a creature-collecting game in the classic handheld style, not a clone of any franchise:
   the words Bicho, Bichodex and Selo are ours). No copied dictionary entries or textbook text. Third-party code or assets must be permissively
   licensed (MIT, Apache-2.0, BSD, CC0, OFL fonts; **no** GPL, AGPL, CC BY-SA or NC) and listed in
   [THIRD_PARTY.md](THIRD_PARTY.md).
3. **European Portuguese.** Content is pt-PT, not pt-BR. Read
   [docs/PT_PT_GUIDELINES.md](docs/PT_PT_GUIDELINES.md) first. If you are not sure of a form, use the
   safest well-known pt-PT one and set `needs_native_review: true`.
4. **No web or WASM targets.** The game is native-only on five platforms.
5. **Run the linter** before you open a pull request:
   `cargo run -p falabichos-tools --bin falabichos-lint` (content, art, music and UI strings).

## Track 1: I speak Portuguese

The most valuable thing you can do is **review**. Nothing in the repo has been reviewed by a native
speaker yet.

1. Find flagged items: `cargo run -p falabichos-tools --bin falabichos-lint -- --review`
   (or search for `needs_native_review: true`).
2. Open the file (e.g. `assets/content/vocab/cafe.ron`), fix anything wrong, then change
   `needs_native_review: true` to `false` (or delete the line) **only if you are confident**.
3. Open a pull request, or use the *Vocabulary fix / native review* issue template if you prefer
   not to edit files.

Things to check: is this the word people actually say in Portugal? Is the accent right? Is the IPA
(European, Lisbon standard) right? Does the example sound natural? Is the register (`tu` vs `o senhor`)
right for the speaker? Does the English gloss say what it means?

You can also write new vocabulary, dialogue and grammar. The formats are plain text with comments:
[docs/CONTENT_FORMAT.md](docs/CONTENT_FORMAT.md) and [docs/HOW_TO_ADD.md](docs/HOW_TO_ADD.md).
Scaffold a file with `cargo run -p falabichos-tools --bin falabichos-new -- vocab weather`.

## Track 2: I'm a pixel artist

Everything visual is defined as small text files or PNGs under `assets/art/`. Read
[docs/BICHO_DESIGN.md](docs/BICHO_DESIGN.md) (style guide, palette, sizes, naming) and
[docs/ART_PIPELINE.md](docs/ART_PIPELINE.md).

- Draw in the **31-colour palette** (`assets/art/palette.txt`), 16x16 tiles and sprites, 1 px outline.
- Write `.px` text files, **or** export RGBA PNGs (frames stacked vertically): a PNG with the same name
  replaces the `.px` file.
- Check your work: `cargo run -p falabichos-tools --bin falabichos-preview -- out/` writes contact sheets.
- New Bicho? Open a *New Bicho idea* issue first, or use `falabichos-new bicho <id>`.

## Track 3: I'm a musician or voice actor

- **Music / SFX**: chiptune is written as text in `assets/music/*.song` (see the format in
  `crates/falabichos-media/src/synth.rs` and [docs/ART_PIPELINE.md](docs/ART_PIPELINE.md#music)).
  `falabichos-preview` renders every song to a WAV so you can listen without launching the game.
- **Voice**: pronunciation is the heart of the project. Record native-speaker clips following the
  [recording spec in docs/AUDIO_GUIDE.md](docs/AUDIO_GUIDE.md) (mono Ogg Vorbis, one word per file, named
  after the vocab item). `falabichos-lint --missing-audio` lists what is still needed. By contributing a
  recording you license it under Apache-2.0.

## Track 4: I'm a Rust dev

```sh
git clone https://github.com/falabichos/falabichos && cd falabichos
cargo run -p falabichos                 # play
cargo xtask ci                          # fmt --check, clippy -D warnings, tests, lint, headless sim
```

- Read [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). In short: game logic is engine-agnostic
  (`falabichos-core`, `falabichos-game`), rendering is a software canvas, Bevy is a thin shell. Most
  features do not touch Bevy at all and can be tested headlessly.
- Add tests. `falabichos-core` has property tests (SRS invariants); `falabichos-game/tests/headless.rs`
  drives the whole game with synthetic input; `cargo run -p falabichos -- --headless-sim` plays through the story.
- `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` must pass.
- Every Rust file starts with `// SPDX-License-Identifier: Apache-2.0`; every `Cargo.toml` has
  `license = "Apache-2.0"` (workspace inheritance is fine).
- Keep the dependency tree modest and permissively licensed; update THIRD_PARTY.md.
- AI coding assistants: see [AGENTS.md](AGENTS.md).

## Track 5: I'm a teacher or curriculum designer

You know what makes learners stumble. Help us with:

- the **order** concepts are introduced and which words go together (Bicho word lists in
  `assets/content/bichos/`);
- **exercises** with good distractors and clear explanations (`assets/content/grammar/`);
- **Mestre tests** that check a real-life skill (ordering food, asking directions, ...);
- pointing out pt-PT vs pt-BR traps for English speakers (`docs/PT_PT_GUIDELINES.md`);
- playtesting with real learners and filing issues about confusing exercises.

Open an issue or a pull request; no Rust needed.

## Pull requests

1. Fork, create a branch, make your change. Keep it focused: one theme per pull request.
2. Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`,
   `content:`, `art:`, `docs:`, ...).
3. Fill in the pull request template. CI runs formatting, clippy, tests, content lint and the headless playthrough.
4. A maintainer or a native-speaker reviewer will review. Content PRs from native speakers get fast
   merges; please be patient with code PRs.

## Questions

Open a Discussion or an issue. No question is too small, and "I speak Portuguese but have never used Git"
is a perfectly good reason to ask for help.
