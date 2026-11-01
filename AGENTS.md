# AGENTS.md: guide for AI coding assistants

Falabichos is an open-source, native-only (Linux, macOS, Windows, Android, iOS) Rust game that teaches
**European Portuguese (pt-PT)** to English speakers. Read this before changing anything.

## Hard rules

1. **Rust only** for game code, tools and build automation. No Python/JS/C# in the product (tiny CI shell is OK).
2. **No web/WASM**: no `wasm32` targets, wasm-bindgen, trunk, `cfg(target_arch = "wasm32")` code in our crates.
3. **Licence**: everything we own is **Apache-2.0 only**. Every Rust file starts with
   `// SPDX-License-Identifier: Apache-2.0`; every `Cargo.toml` has `license = "Apache-2.0"` (via
   `license.workspace = true`). No MIT/dual licensing, no CC BY-SA for our material. New dependencies must be
   permissive (MIT, Apache-2.0, BSD, ISC, Zlib, CC0, OFL fonts) and be added to THIRD_PARTY.md.
4. **Original art/audio/names only.** Never reference or imitate any commercial game franchise. The game's
   vocabulary is Bicho / Bichodex / Selo / Mestre.
5. **European Portuguese**, post-1990 spelling (see `docs/PT_PT_GUIDELINES.md`). When unsure, use the safest
   pt-PT form and set `needs_native_review: true`. Never "fix" pt-PT into pt-BR.
6. Do not weaken tests or lints to make something pass.

## Commands

```sh
cargo run -p falabichos                                   # play (needs Bevy system libs on Linux)
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p falabichos-tools --bin falabichos-lint       # validate content, art, music, UI strings
cargo run -p falabichos -- --headless-sim                 # scripted playthrough with no display
cargo run -p falabichos -- --screenshots out/             # PNG screenshots from the same script
cargo xtask ci                                            # all of the above
```

On an immutable Linux host build inside a container (see README, "Building from source").

## Architecture in one minute

```
assets/ (data)  ->  falabichos-assets (embed)  ->  falabichos-content (load + validate -> Content)
falabichos-core (NO engine): model, SRS, exercises, battle, progress, saves, i18n
falabichos-media (NO engine): .px art, font, software Canvas, chiptune synth
falabichos-game (NO engine): Game::tick(Input) + Game::draw(&mut Canvas) + GameEvent; headless sim
falabichos (Bevy shell): window, GPU texture upload, input, audio, mobile entry points
```

- The game renders to a 240x160 RGBA `Canvas` in software; Bevy only uploads it as one texture and scales
  it by an integer factor. That is why screenshots and the headless playthrough need no GPU.
- Game logic runs at a fixed 60 Hz in `Game::tick`. Time (`now`) is injected; randomness is a seeded
  `Rng`: everything is deterministic and testable.
- `Scene` (title, world, menu, battle, dex, notebook, seals, settings) is an enum in `falabichos-game`;
  dialogue is an overlay inside the world.

## Where things live

| To change... | Edit |
|--------------|------|
| Vocabulary | `assets/content/vocab/*.ron` |
| Grammar concepts and exercises | `assets/content/grammar/*.ron` |
| A Bicho | `assets/content/bichos/<id>.ron` + `assets/art/bichos/<id>.px` |
| A town, map, NPC, dialogue, trainer | `assets/content/towns/<town>/...` (maps are ASCII rows; tile legend is `assets/content/tiles.ron`) |
| UI text | `assets/i18n/en/ui.lang` (keys are checked against the code by a unit test) |
| Art | `assets/art/**` (`.px` text or PNG) and `assets/art/palette.txt` (31 colours max) |
| Music/SFX | `assets/music/*.song` |
| Exercise generation / SRS / battle rules | `crates/falabichos-core/src/{exercise,srs,battle}.rs` |
| Screens | `crates/falabichos-game/src/{world,dialogue,battle_ui,menus}.rs` |
| Platform glue | `crates/falabichos/src`, `mobile/` |

## Conventions

- Content files are RON with `IMPLICIT_SOME` (write `ipa: "x"`, not `Some("x")`); unknown fields are errors.
- IDs are lowercase `snake_case` ASCII and are never renamed after release (saves refer to them).
- New content needs: English gloss on every line, `ipa` on every word, `needs_native_review: true` unless a
  native speaker checked it. Run the linter; it must report 0 errors.
- UI strings: add the key to `assets/i18n/en/ui.lang` and use `self.t("key")`; a test fails on missing keys.
- Save-format changes: bump `SAVE_VERSION`, keep the old struct under `legacy`, add a migration and a test.
- Keep Bevy usage thin; put logic in `falabichos-core` / `falabichos-game` where it can be tested headlessly.
- Be honest in docs: say what was *not* built or run (see `docs/PLATFORM_STATUS.md`).

## Testing

- Unit and property tests live next to the code (`falabichos-core` has `proptest` SRS invariants).
- `crates/falabichos-game/tests/headless.rs` and `sim.rs` drive the real game with synthetic input and a
  path-finder over the real maps, from the title screen to the finale.
- Visual changes: run `--screenshots`, then look at the PNGs.
