# Changelog

All notable changes are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html) from 1.0 onwards.

## [Unreleased]

### Added
- Vertical slice: two towns (Vila Azul, Setecolinas), 10 maps, 9 Bichos, 96 vocabulary items, 12 grammar
  concepts with 63 exercises, 2 Mestres with Selos, 5 trainers, story arc and finale.
- `falabichos-core`: SM-2-style spaced repetition with learning steps and property tests, exercise
  generation and checking (choice, type, order, article, listen/IPA), accent- and typo-tolerant answer
  matching, conversation-battle state machine, progress and seals, versioned saves with migrations,
  UI string tables.
- `falabichos-game`: engine-agnostic game (overworld, dialogue with gloss and word lookup, battles,
  Bichodex, notebook, seals, settings) rendered by a software canvas, plus a headless simulator that plays
  the story end to end.
- `falabichos` (Bevy 0.19.1) client: integer-scaled 240x160 display, keyboard, gamepad and touch controls,
  procedural chiptune audio, fullscreen, mobile entry points and lifecycle handling.
- Original pixel-art pipeline (31-colour palette, `.px` sprites, recolourable characters, 5x7 bitmap font
  with pt-PT diacritics and IPA) and chiptune synthesiser (`.song` files).
- Tools: `falabichos-lint`, `falabichos-mapcheck`, `falabichos-preview`, `falabichos-new`; `cargo xtask`
  for CI, packaging (Linux, macOS, Windows) and mobile builds (Android, iOS).
- Android (Gradle + GameActivity) and iOS (Xcode) projects; GitHub Actions for CI and tagged releases;
  Dependabot.
- Community documents, issue and pull-request templates, 25+ good first issues, platform status matrix.

### Known limitations
- No native-speaker review of the Portuguese content yet (every vocab item is flagged).
- No recorded pronunciation clips yet (the slot system, loader and lint are in place).
- macOS, Windows, Android and iOS builds were written from the official documentation but not built by the
  author (see docs/PLATFORM_STATUS.md).
