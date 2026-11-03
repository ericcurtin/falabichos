# Good first issues

Small, well-scoped tasks. Each says **what to do**, **where**, and **how to know it is done**. Pick one, comment on (or open) an issue so nobody duplicates work, and send a PR.
You never need to understand the whole project. Tags: 🗣 Portuguese · 🎨 art · 🎵 audio · 🦀 Rust · 🎓 teaching · 🌍 translation · 📚 docs.

## 🗣 Portuguese (no Rust needed)

1. **Review the café vocabulary.** Open `assets/content/vocab/cafe.ron`; check every `pt`, `ipa`, `example` and `note`; fix mistakes and set `needs_native_review: false` on entries you are sure of. *Done when:* the file is reviewed and `falabichos-lint` is clean.
2. **Review the greetings vocabulary** (`vocab/greetings.ron`), especially register notes (`tu` vs `o senhor`), and whether *como está* / *como estás* are glossed well.
3. **Review the IPA of 20 words** of your choice (any vocab file). Is it standard European Portuguese (Lisbon)? Fix vowel reductions, nasal vowels and `s/z` endings. *Done when:* fixed entries have `needs_native_review: false`.
4. **Proof-read the Vila Azul dialogues** (`assets/content/towns/vila_azul/dialogues/vila_azul.ron`): are they natural? Are the English glosses faithful and the per-word `words:` glosses right?
5. **Proof-read the Setecolinas dialogues** the same way (`towns/setecolinas/dialogues/setecolinas.ron`).
6. **Review one grammar concept** (`assets/content/grammar/clitics.ron` is the one most worth a native check): rules, examples, `pt_pt_note`, every exercise and explanation.
7. **Add 12 weather words** (`falabichos-new vocab weather`): *chuva, sol, vento, nuvem, frio, calor, nevoeiro, trovoada, ...* with gender, IPA, an example sentence and `needs_native_review: true`.
8. **Add 12 family words** (`falabichos-new vocab family`): *pai, mãe, irmão, irmã, tio, tia, primo, prima, neto, ...*, noting that the plural of *irmão* is *irmãos*.
9. **Add 12 body and health words** (`falabichos-new vocab body`): *cabeça, mão, pé, olho, dor, médico, farmácia, ...*
10. **Write 20 more pt-PT vs pt-BR pairs** for `docs/PT_PT_GUIDELINES.md` section 2 (clothing, kitchen, school...). Cite nothing copied: write them yourself.

## 🎓 Teaching and curriculum

11. **Write a new grammar concept: *por* vs *para*** (`falabichos-new grammar por_para`): 4 rules, 3 examples, 6 exercises with explanations.
12. ~~**Write a new concept: *ter* and *haver***~~ (done: `grammar/ter_haver.ron`; a native-speaker review of it is still very welcome).
13. **Write a new concept: *pretérito perfeito* of regular verbs** (*falei, falaste, falou, falámos, falaram*: note the accent in *falámos*, which is pt-PT).
14. **Audit the difficulty ladder.** Play a few battles and list in an issue any exercises that are confusing, ambiguous, or have two right answers.
15. **Design a Mestre test for the next town** (Alentejo-inspired: food, farming, numbers and prices): write the `challenge` RON with a pool of vocabulary categories and grammar concepts and a theme line. (The Porto-inspired "directions" Mestre is done: `towns/ribeira_dourada/challenges`.)

## 🎨 Art

16. **Draw 3 new tiles** (cobbled street corner, a *calçada portuguesa* wave pattern, a tram stop sign) as 16×16 `.px` in `assets/art/tiles/`; add them to `content/tiles.ron`; check with `falabichos-preview`.
17. **Draw a new Bicho** for an unused concept from [BICHO_DESIGN.md](BICHO_DESIGN.md) (e.g. *Caravela*): `falabichos-new bicho caravela`, original 16×16 art, a theme and a vocab list.
18. **Make a second animation frame for an existing Bicho** (e.g. the Galo flapping): add a `---` frame to its `.px`.
19. **Draw a new NPC body template** (`assets/art/chars/_body_c.px`, e.g. an older person with a cane or a child), 6 frames, role digits only.
20. **Design the game's icon** as a 16×16 `.px` (and a 256×256 PNG) for desktop packaging in `packaging/`: original, azulejo style.

## 🎵 Audio

21. **Record 15 greetings** per [AUDIO_GUIDE.md](AUDIO_GUIDE.md) (`audio/pt-PT/greetings/ola.ogg`, `bom_dia.ogg`, ...). *Done when:* `falabichos-lint` shows them counted and no misnamed-clip warnings.
22. **Compose a 30-second chiptune theme for a new town** (`assets/music/ribeira.song`); render it with `falabichos-preview` and listen.
23. **Write 3 new sound effects** (a tram bell, a rooster, a wave) as `sfx_*.song` and propose where to use them.

## 🦀 Rust

24. **Language picker in Settings.** Add a "Language" row to `crates/falabichos-game/src/menus.rs` that cycles through the folders under `assets/i18n/` (needs `Assets::list`) and saves `ui_lang`. *Done when:* a test switches language and a string changes.
25. **Review calendar.** In the Bichodex or menu, show how many SRS cards are due now / tomorrow (`SrsStore::due_count`). Add a unit test in `falabichos-core`.
26. **Add a `--list-content` flag** to `falabichos-lint` that prints each vocab category with item counts and the percentage with audio.
27. **Gamepad rumble / haptics hook:** emit a `GameEvent::Haptic` on wrong answers and handle it for gamepads in the Bevy client (behind a setting). Keep Bevy usage thin.
28. **Tap-to-select in battle** for touch: let a tap on an option row choose it directly (the shell converts tap positions to logical pixels; extend `Input` or add a `pointer` field). Add a headless test.
29. **Property test:** add a `proptest` that `check_text` never accepts an answer whose folded edit distance exceeds the typo budget, in `crates/falabichos-core/src/normalize.rs`.
30. **Lint rule:** warn when a dialogue line's English gloss is identical to the Portuguese text (a copy-paste mistake), in `crates/falabichos-content/src/validate.rs`, with a test.
31. **Map checker output as PNG:** add `--png out.png` to `falabichos-mapcheck` that renders the map with the real tiles using `falabichos-media::Canvas`.
32. **CI:** add a `cargo deny` / licence-check step that fails on non-permissive dependency licences (config in `deny.toml`), and document it.

## 🌍 Translation

33. **Translate the UI to a language you speak** (`cp -r assets/i18n/en assets/i18n/<code>`; see [LOCALISATION.md](LOCALISATION.md)). *Done when:* the linter shows no missing keys and no missing glyphs (add glyphs to `font.txt` if needed).
34. **Add glyphs for ¿ ¡ « » ß œ** to `assets/art/font.txt` (5×8 each) so more European languages work.

## 📚 Docs

35. **Add screenshots to the README** from `cargo run -p falabichos -- --screenshots docs/screenshots/` and pick the best six.
36. **Write a "play-testing script"** (`docs/PLAYTEST.md`): a 15-minute checklist a teacher can follow with a learner and a form to report confusion.
