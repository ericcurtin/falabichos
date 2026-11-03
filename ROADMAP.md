# Roadmap

Status key: **done**, **in progress**, **next**, **later**. Want to take something? Open an issue.

## Part 1: the vertical slice (done, v0.1)

- Overworld, dialogue with gloss and word lookup, conversation battles (choice, type, order, article, IPA),
  catch mechanic, SRS-driven Bicho levels, Practice, Bichodex, Notebook, Seals, Settings, saves with migrations.
- Towns: **Vila Azul** (fishing village, Mestre Joaquim: ordering food) and **Setecolinas** (hill city with
  trams, Mestra Beatriz: transport and polite register).
- 9 Bichos, 96 vocabulary items, 12 grammar concepts, headless simulation, linter, art and audio pipelines.

## Part 2 progress (branch `roadmap-progress`)

- Town 3, **Ribeira Dourada**, is in (see the table below). Totals now: 3 towns, 15 maps, 13 Bichos,
  124 vocabulary items, 15 grammar concepts (89 exercises), 3 Mestres and Selos, 10 trainers.

## Part 2: the rest of the journey

| Town | Inspired by | Theme of the Mestre | Status |
|------|-------------|---------------------|--------|
| 3. **Ribeira Dourada** | Porto, riverside, azulejo churches, port-wine cellars | Directions and the city (ao pé de, à esquerda), `ter` and `haver` | **done** (v0.2 content: 5 maps, Mestre Duarte, 5 trainers, 4 Bichos, 28 words, 3 grammar concepts; all flagged for native review, never played on a real device) |
| 4. **Planície do Sol** | Alentejo plains, cork oaks, white villages | Food and farming, numbers and prices, pretérito perfeito | later |
| 5. **Praia Dourada** | Algarve coast, cliffs and beaches | Weather and holidays, `por` vs `para` | later |
| 6. **Ilha Verde** | Azores | Nature and pronouncing the islands' vowels, `ser/estar` revision | later |
| 7. **Ilha da Madeira**-style island | Madeira, levadas and terraces | Time, dates, health, present subjunctive basics | later |
| 8. **Casa do Grande Mestre** | the "capital" finale | A final conversation exam across everything | later |

## Learning features

- **done** Review calendar: a *Review* screen in the pause menu shows what is due now, within 24 h and in 24 to 48 h, plus a 7-day histogram (`SrsStore::calendar` in core, rolling 24 h windows because the engine has no time-zone data).
- **next** More exercise types: listening dictation, sentence translation with hints per word.
- **next** Per-concept "mini-lesson" screens before a grammar Bicho's first battle.
- **later** Verb conjugation tables as a game mode (pretérito perfeito, imperfeito, futuro).
- **later** Adaptive difficulty per learner (use accent / typo policy as a ladder).

## Audio and pronunciation

- **next** Native-speaker recordings for all 96 vocabulary items (see `docs/AUDIO_GUIDE.md`).
- **later** Recorded dialogue lines, listening-only battles.
- **later** Optional speech recognition **only if** a permissively licensed, offline, pure-Rust option appears.

## Platform and engine

- **next** Real safe-area insets from the OS (needs winit 0.31 / a newer Bevy; today phones use conservative defaults).
- **next** Verify and polish macOS, Windows, Android and iOS builds on real hardware; automate store packaging.
- **later** Upgrade Bevy (0.19.1 today; 0.20 needs a newer Rust toolchain than we pin).
- **later** Touch-native menus (tap to select an answer) and haptics.
- **later** Controller glyphs per vendor.

## Community

- **next** Translations of the UI: Spanish, French, German, Italian, Dutch, Polish, Chinese, ...
- **next** A contributors' content dashboard (lint output rendered as a web page *outside* the game; the game itself stays native-only).
- **later** Teacher tools: export a learner's SRS state as a progress report.

See also [docs/GOOD_FIRST_ISSUES.md](docs/GOOD_FIRST_ISSUES.md).
