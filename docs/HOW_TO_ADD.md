# How to add a town, a Bicho or a lesson

All three are data files; none needs Rust. Use the scaffolder to get started, then edit the files.

```sh
cargo run -p falabichos-tools --bin falabichos-new -- vocab weather      # assets/content/vocab/weather.ron
cargo run -p falabichos-tools --bin falabichos-new -- grammar por_para   # assets/content/grammar/por_para.ron
cargo run -p falabichos-tools --bin falabichos-new -- bicho nuvem        # content/bichos/nuvem.ron + art/bichos/nuvem.px
cargo run -p falabichos-tools --bin falabichos-new -- town ribeira       # a town folder with a starter map and dialogue
cargo run -p falabichos-tools --bin falabichos-lint                      # must report 0 errors
```

The scaffolder refuses to overwrite existing files. Field-by-field reference: [CONTENT_FORMAT.md](CONTENT_FORMAT.md).

## Add vocabulary (a "lesson" of words)

1. `falabichos-new vocab weather` creates `assets/content/vocab/weather.ron` with one commented example.
2. Fill in 8-16 words: `id`, `pt` (no article), `en`, `category: "weather"`, `gender` for nouns, `ipa`, `pron_tags`, and an `example` sentence of
   3-8 words (this also unlocks the *order the words* exercise). Keep `needs_native_review: true`.
3. Read [PT_PT_GUIDELINES.md](PT_PT_GUIDELINES.md): pt-PT forms and post-1990 spelling.
4. Words are only used in the game once a **Bicho embodies them** (or a Mestre draws from the category), see below.
5. Run the linter.

## Add a Bicho

1. `falabichos-new bicho nuvem` creates `assets/content/bichos/nuvem.ron` and `assets/art/bichos/nuvem.px`.
2. In the `.ron`: set a unique `dex_no`, a `theme`, the Portuguese flavour line **with an English version**, and the `vocab` ids it embodies (at least 3).
   For a grammar Bicho use `kind: Grammar` and `grammar: ["concept_id"]`.
3. Draw the art: [BICHO_DESIGN.md](BICHO_DESIGN.md). Preview with `falabichos-preview out/` and open `out/bichos.png`.
4. Make it appear: add `(species: "nuvem", weight: 20)` to a map's `encounters` (that map needs tall-grass tiles `"`).
5. Lint, then play: `cargo run -p falabichos` and walk into the grass.

## Add a grammar lesson

1. `falabichos-new grammar por_para`.
2. Write the **rules** (short bullets), 2-4 **examples**, a **pt_pt_note**, and at least **5 exercises** with clear **explanations**.
   Mix kinds: `FillBlank`/`Choice` for recognition, `Type` for production, `Order` for word order. Use `hint` sparingly.
3. Attach it to a grammar Bicho (`grammar: ["por_para"]`) so players meet it in battles, and optionally to a Mestre's `pool`.
   Every concept automatically appears in the in-game **Notebook**.

## Add a town

1. `falabichos-new town ribeira` creates `towns/ribeira/{town.ron, maps/ribeira_outside.ron, dialogues/ribeira.ron}`.
2. Edit the map rows (every row the same length; characters from `tiles.ron`). Add houses with `warps` into interior maps,
   NPCs, signs and tall-grass `encounters`. Check with `falabichos-mapcheck ribeira_outside`.
3. Add a `challenges/` file with a **Mestre** (`kind: Mestre`, a `seal`, `reward_flag`) and a few trainers; add the Selo to `content/seals.ron`
   (and a 16x16 `art/seals/<id>.px`).
4. Connect it to the world: add a `warp` from the previous map's edge to your map and back.
   Gate it with an NPC that has `show_when: [NotFlag("beat_previous_mestre")]`, like the old man in Vila Azul.
5. New characters: add a line to `art/chars.txt` (pick a body template and palette letters) and use its id as an NPC `art`.
6. Lint. The linter checks warps, dialogue ids, reachability of every door and NPC, art and font coverage.
7. Play it. Run `cargo run -p falabichos -- --headless-sim` to make sure the main story path still works.

## Checklist before opening a PR

- [ ] `falabichos-lint` reports 0 errors
- [ ] English gloss on every line, IPA on every word, pt-PT (not pt-BR)
- [ ] `needs_native_review: true` unless a native speaker checked it
- [ ] IDs are snake_case and new (never rename existing ones)
- [ ] Art uses only the palette; text only uses characters in the font (the linter checks)
