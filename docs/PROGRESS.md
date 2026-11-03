# Falabichos: living progress log

Read this first if you are resuming work (human or AI). Then `git log --oneline`.

## Environment notes (how this was built)

- The host had no Bevy dev headers and no sudo. All Linux builds ran inside a rootless **toolbox** container `falabichos-dev`
  (Fedora 42). Packages were installed with `podman exec --user root falabichos-dev dnf install ...`.
- `CARGO_TARGET_DIR` is **not** inherited by toolbox; wrappers set it explicitly: `toolbox run -c falabichos-dev env CARGO_TARGET_DIR=... cargo ...`.
- Bevy is pinned **=0.19.1** (0.20.1 exists on crates.io but needs a newer rustc than the pinned 1.95).
- Virtual display for runs: `xvfb-run` + `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.x86_64.json WGPU_BACKEND=vulkan` (lavapipe).
- Long builds (first Bevy build ~2 min debug, ~4.5 min release with thin LTO) were run detached and polled.

## Architecture decisions

- Bevy is a thin shell. Game logic **and** software rendering of the 240x160 frame are in engine-agnostic crates
  (`falabichos-core`, `falabichos-media`, `falabichos-game`), so the headless sim and screenshots exercise the real game with no GPU.
- Own 5x7 bitmap font (no third-party font licence, perfect pt-PT diacritics and IPA).
- Assets are embedded with `include_bytes!` (build.rs): self-contained binary, no file access needed on mobile; `--assets DIR` overrides.
- No Python or JS in the product. (Python was used only as a throwaway authoring aid *outside the repo* to generate the initial `.px`/`.ron` text;
  the committed files are the source of truth now.)

## Milestones

- [x] 1. Preflight: toolchain, container, Bevy spike (window + sprite under Xvfb/lavapipe)
- [x] 2. Workspace skeleton, LICENSE (canonical Apache-2.0), NOTICE, rust-toolchain, profiles
- [x] 3. `falabichos-core` (model, normalise, SRS + proptests, exercises, battle, progress, saves + migration, i18n), `-content` (load + validate), `-tools`
- [x] 4. `falabichos-media` (palette, `.px`, font, canvas, synth) + original art and music assets
- [x] 5. `falabichos-game` (title, world, dialogue + gloss + word lookup, battles, dex, notebook, seals, settings) + headless sim
- [x] 5b. Bevy client: integer-scaled display, keyboard / gamepad / touch, audio, lifecycle, `--smoke`, `--window-shot`
- [x] 6. Vertical-slice content: 2 towns, 10 maps, 9 Bichos, 96 vocab, 12 grammar concepts (63 exercises), 2 Mestres, 5 trainers
- [x] 7. `xtask`, Android (Gradle + GameActivity) and iOS (Xcode) glue, CI + release workflows, Dependabot
- [x] 8. Community docs, 36 good first issues, platform status
- [x] 9. Final verification, `docs/FINAL_REPORT.md`
- [x] 10. Part 2, town 3 (Ribeira Dourada): 5 maps, 4 Bichos, 28 words, 3 grammar concepts, Mestre Duarte, 5 trainers, wired into progression and the headless sim

## What is NOT done (honest list)

- macOS, Windows, Android, iOS were **not built or run** (no hardware/SDKs). See [PLATFORM_STATUS.md](PLATFORM_STATUS.md).
- No native-speaker review of any Portuguese (132 items flagged `needs_native_review`).
- No recorded audio clips; audio output was never *heard* (no sound device in the container).
- Real safe-area insets (winit 0.31) are not available in Bevy 0.19.
- Towns 4-8 and the rest of the story (see ROADMAP.md). Town 3 exists but, like everything else, is flagged for native review.

## Bugs the tests/screenshots caught during development (kept as lessons)

- `cargo xtask android` called a Gradle wrapper that does not exist (fixed; test now pins `gradle`/`gradle.bat`).
- iOS plan used `-scheme` for a hand-written project with no shared scheme (now `-target`, with a test).
- Bichodex cursor started on an unseen `???` row so A did nothing (now starts on the first met Bicho).
- IPA brackets `[ ]` were missing from the bitmap font and rendered as `?` (added; code string literals scanned).
- "Large text" did not apply in battles (fixed; the sim now draws every battle screen with large text).
- Stale toasts overlapped the battle HUD (cleared when a battle starts).
