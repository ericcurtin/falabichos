# Falabichos: final report

Date of verification: see `git log -1`. Everything below was produced in this repository; commands were run for real unless a section says **not done**.

## 1. What was built

**Falabichos**: an open-source (Apache-2.0) native game that teaches **European Portuguese (pt-PT)** to English speakers through a top-down, 16x16, creature-collecting
adventure where the "battles" are friendly *conversations*.

| Area | Delivered |
|------|-----------|
| Game | Title, new game / continue, overworld (grid-locked walking, NPCs, signs, doors/warps with fades, tall-grass encounters, trainers with line-of-sight), dialogue (typewriter text, English gloss on demand, word-by-word lookup), conversation battles (wild catch, trainer/Mestre tests, practice), Bichodex + party, grammar Notebook, Seals, Settings (text size, high contrast, reduce motion, volume, gloss default, accent/typo forgiveness, fullscreen), saves with versioned migrations, settings persistence, autosave on suspend |
| Learning engine | SM-2-style **spaced repetition with learning steps** (property-tested), exercise generator with a difficulty ladder (choice, article/gender, IPA reading, listening with real clips when present, order-the-words, typed answers), accent-insensitive and typo-tolerant answer matching (policy per setting), hints, Bicho level derived from SRS mastery |
| Content (data files) | 2 towns (**Vila Azul** fishing village, **Setecolinas** hill city with trams), 10 maps, 9 Bichos (7 vocabulary + 2 grammar), **96 vocabulary items**, **12 grammar concepts / 63 exercises**, 2 Mestres + 2 Selos, 5 trainers, 56 dialogues (108 lines) each with English gloss, pronunciation tips for 10 pt-PT learning points |
| Graphics / audio | Original 31-colour pixel art defined as text (`.px`), 40 tiles, 9 Bichos, 16 recolourable characters, 2 seals, original 5x7 bitmap font with pt-PT diacritics and IPA; 19 original chiptune songs/SFX synthesised in Rust at runtime; native-speaker clip slot system + loader + lint |
| Platforms | Bevy 0.19.1 client for Linux, macOS, Windows, Android (Gradle + GameActivity), iOS (Xcode); keyboard, gamepad, touch (on-screen D-pad/A/B/MENU/HINT, tap-to-advance, safe-area-aware landscape layout, DPI-aware), pause/resume lifecycle, per-platform data directories |
| Tools | `falabichos-lint` (content, art, font coverage, music, UI strings, audio, native-review list), `-mapcheck`, `-preview` (sprite sheets + WAV), `-new` (scaffolder); `cargo xtask ci / desktop / package-linux / package-macos / package-windows / android / ios / check-targets` (all with `--dry-run`) |
| CI | `.github/workflows/ci.yml` (fmt, clippy, tests on ubuntu/macos/windows, content lint, headless sim, Android APK/AAB job, iOS simulator job), `release.yml` (tagged release for all five platforms), Dependabot |
| Community | README, CONTRIBUTING (5 tracks), CODE_OF_CONDUCT (Contributor Covenant 2.1), SECURITY, GOVERNANCE, ROADMAP, CHANGELOG (Keep a Changelog), CREDITS, THIRD_PARTY, AGENTS.md, 5 issue templates + PR template, `docs/` (ARCHITECTURE, CONTENT_FORMAT with worked examples, HOW_TO_ADD, LOCALISATION, PT_PT_GUIDELINES, BICHO_DESIGN, ART_PIPELINE, AUDIO_GUIDE, BUILDING, PLATFORM_STATUS, GOOD_FIRST_ISSUES with **36** tasks, PROGRESS) |

Size: 8 workspace crates, ~13,300 lines of Rust, 126 asset files, 193 tracked files.

**Chosen Bevy version: `=0.19.1`**, pinned exactly (default features off). Bevy 0.20.1 is on crates.io but requires a newer Rust than the 1.95.0 pinned here.

## 2. How to run, per platform

| Platform | Run |
|----------|-----|
| Linux | `cargo run -p falabichos` (needs Bevy system libraries: package lists in README). Immutable hosts: inside a toolbox (README, docs/BUILDING.md) |
| macOS | `cargo run -p falabichos`; package: `cargo xtask package-macos` |
| Windows | `cargo run -p falabichos` (MSVC toolchain); package: `cargo xtask package-windows` |
| Android | `cargo xtask android` (JDK 17, SDK, NDK 26, cargo-ndk, Gradle) then `adb install` |
| iOS | `cargo xtask ios` (simulator) / `--device` (needs signing) |
| Any, no display | `cargo run -p falabichos -- --headless-sim`, `-- --screenshots out/` |

## 3. Verified: exact commands and results

All on one Linux x86_64 host, inside the Fedora 42 toolbox container `falabichos-dev` (no sudo, no GPU, no sound device).

| Command | Result |
|---------|--------|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, no warnings |
| `cargo test --workspace` | **85 passed, 0 failed, 0 ignored** (incl. proptest SRS invariants, save migration, answer matching, exercise generation for every vocab item at every difficulty, scaffold templates lint clean, headless playthrough, every map renders, UI-key coverage, touch layout, xtask plans) |
| `cargo run -p falabichos-tools --bin falabichos-lint -- --strict --quiet` | exit 0: **0 errors, 0 warnings** (1 info: no audio clips yet) |
| `cargo run -p falabichos -- --headless-sim` | exit 0, **ALL HEADLESS CHECKS PASSED**: title → new game → intro → Grandma → library (Notebook) → wild encounter with a deliberately wrong first answer → catch → exercise tour covering Choice, Article, Grammar, IPA-listening, Order and **Type** exercises through the real UI (normal and large text) → Bichodex/Notebook/Seals/Settings → Practice (SRS) → **save → quit → resume** (position, party, 65 SRS cards restored) → Mestre Joaquim (Selo do Polvo) → road (blocked gate verified first) → Setecolinas → Mestra Beatriz (Selo do Elétrico) → finale |
| core sim (inside the same run) | caught the Galo; the missed word is due again in 580 s; Galo level 1 → 3 after 14 simulated days of on-time reviews; v0 save migrated |
| `xvfb-run` + lavapipe: `falabichos --smoke --autoquit 20 --window-shot ...` (debug and the **release** binary) | starts, uploads the frame to the GPU texture, runs scripted input, saves, exits 0, **no panics** |
| `--window-shot` | real GPU-rendered window PNG at 4x integer scale with letterboxing: `docs/screenshots/window-xvfb-lavapipe.png` |
| `--touch` under Xvfb | on-screen controls render: `docs/screenshots/touch-controls.png` |
| `cargo xtask package-linux` | release build (`lto = "thin"`, `codegen-units = 1`, `strip`), 4m19s, 14 MB `dist/falabichos-0.1.0-linux-x86_64.tar.gz`; its binary passes `--headless-sim` and the Xvfb run |
| `cargo check --target {x86_64-pc-windows-gnu, aarch64-linux-android, aarch64-apple-darwin, aarch64-apple-ios}` on the 5 engine-agnostic crates | all exit 0 |
| `cargo check --target x86_64-pc-windows-gnu -p falabichos` (whole Bevy client) | exit 0 |
| Screenshots of 14 real screens | `docs/screenshots/` (`--screenshots`), looked at and used to fix 6 layout/UX bugs |

## 4. NOT verified (explicitly)

- **macOS**: client not built or run. (`cargo check` for the client stops in `blake3`'s C build script: no Apple toolchain on Linux. The 5 game crates check clean for `aarch64-apple-darwin`.)
- **Windows**: whole client *type-checks* for `x86_64-pc-windows-gnu`, but nothing was linked, packaged or run; the MSVC target and `package-windows` are untouched.
- **Android**: the 5 game crates check clean for `aarch64-linux-android`; the client does not (needs the NDK's clang for `android-activity`). The Gradle project, `cargo-ndk` flow, manifest, `MainActivity` and the `cfg(target_os = "android")` code were **never compiled**.
- **iOS**: the 5 game crates check clean for `aarch64-apple-ios`; the client needs Xcode (`xcrun`). The Xcode project is **hand-written and was never opened**; the `main_rs` entry point and Run Script phase were never executed.
- CI workflows for macOS, Windows, Android and iOS were written from official documentation and **have never run** (no remote, nothing was pushed).
- **Audio output was never heard** (no sound device; Bevy logs an ALSA warning and continues). The synthesiser and loader are unit-tested; nobody has listened to the music.
- Physical GPU, Wayland, gamepads, real touch screens, real safe-area insets, real mobile suspend/resume.
- No native Portuguese speaker has reviewed anything.

Full matrix: [PLATFORM_STATUS.md](PLATFORM_STATUS.md).

## 5. Known issues and risks

1. **Portuguese accuracy** is the biggest risk (see section 6). The author is not a native speaker; IPA is a careful broad transcription but unreviewed.
2. **Mobile and macOS/Windows are untested.** Expect first-run fixes (Gradle plugin versions, Xcode project structure, NDK flags).
3. **Safe-area insets** use conservative defaults on phones (winit 0.30 has no query); overridable with `FALABICHOS_SAFE_INSETS`.
4. **Bevy is thin by design**: the game does not use Bevy's ECS for game logic. Upgrading Bevy only touches `crates/falabichos`.
5. Typed answers on mobile use the in-game letter grid (no system keyboard).
6. The UI is English only; the learning gloss is English only (the structure for more languages exists: `assets/i18n/<lang>`, but the bitmap font covers Latin scripts only).
7. The `.px`/RON content was initially authored with throwaway scripts outside the repo. The committed text files are the source of truth; no generators are part of the product.
8. Placeholder contact addresses in CODE_OF_CONDUCT.md and SECURITY.md (`*.invalid`) must be replaced before publishing.
9. The repository URL in badges and Cargo metadata points to github.com/ericcurtin/falabichos.

## 6. Native-review items

132 items carry `needs_native_review: true` and are listed by `falabichos-lint --review`: **96 vocabulary items** (all of them: pt, IPA, example sentence, notes), **12 grammar concepts** (rules, examples, every exercise), **9 Bichos** (flavour text), **2 towns**, **13 dialogue lines** (the least certain phrasings, e.g. *Ainda não é desta*, *Treina mais, pá*, the Mestre Joaquim intro). Highest priority for a native check: `clitics.ron`, `tu_voce.ron`, the IPA of nasal and `ão/ões` words, `pequeno-almoço`, and the register of NPC dialogue.

## 7. Remaining roadmap

Towns 3-8 (Porto, Alentejo, Algarve, Azores, Madeira, final Mestre), native-speaker audio for all vocabulary, more grammar (pretérito perfeito, por/para, subjunctive), review calendar, tap-to-select battles on touch, OS safe-area insets (newer Bevy), community UI translations. See [../ROADMAP.md](../ROADMAP.md).

## 8. Hard requirements: confirmation

**1. Rust only.** All game code, tools and build automation are Rust (8 crates incl. `xtask`). No Python/JS/C#. The only non-Rust sources are the platform shells that those platforms mandate: a 9-line Objective-C `main.m` (iOS entry), one Java `MainActivity` (Android `GameActivity` host), Gradle build files, an Xcode project file, YAML for CI. (A throwaway Python authoring aid was used *outside the repo* to draft the initial text assets; it is not part of the product.)

**2. Five platforms targeted:** Linux, macOS, Windows, Android, iOS: architecture, code, build config, packaging scripts and CI all target them. **Verified: Linux only** (section 3/4).

**3. No WASM anywhere.** Output of `grep -RIn -i "wasm" --exclude-dir=target --exclude-dir=.git --exclude=Cargo.lock .`:

```
./GOVERNANCE.md:31:- Native-only: no web/WASM build targets.
./xtask/src/main.rs:411:                assert!(!c.show().to_lowercase().contains("wasm"), "{t}");
./AGENTS.md:9:2. **No web/WASM**: no `wasm32` targets, wasm-bindgen, trunk, `cfg(target_arch = "wasm32")` code in our crates.
./crates/falabichos/Cargo.toml:32:# game needs is enabled (2D sprite rendering, windowing, audio, gamepad). No web/wasm features.
./CONTRIBUTING.md:35:4. **No web or WASM targets.** The game is native-only on five platforms.
./docs/BUILDING.md:19:input, audio (Ogg/WAV) and gamepad. There is no web/WASM feature anywhere.
```

Every hit is either a policy statement (GOVERNANCE, AGENTS, CONTRIBUTING, BUILDING, the Cargo.toml comment), the unit test in `xtask` that *asserts* no build command mentions it, or this very report quoting the grep and its output (re-running the grep also matches those quoted lines in `docs/FINAL_REPORT.md`). There is no wasm code, dependency, target, feature or job in the project. **`Cargo.lock`** also contains 33 lines naming `wasm-bindgen`, `wasm-bindgen-futures`, `wasm-bindgen-macro`, `-macro-support`, `-shared` and `tracing-wasm`: these are *target-conditional* dependencies of Bevy and its dependencies (`cfg(target_arch = "wasm32")`), which Cargo records in the lockfile for every possible platform. They are never compiled for a native target. Proof: `cargo tree -p falabichos --target <T> -e normal,build` lists **0** wasm/web-sys/js-sys crates for each of x86_64-unknown-linux-gnu (399 crates), x86_64-pc-windows-gnu (354), aarch64-apple-darwin (369), aarch64-linux-android (380) and aarch64-apple-ios (362). No wasm target is installed or referenced, no `trunk`/`wasm-bindgen` tooling, no web CI job.

**4. Licence is Apache-2.0 only.** `LICENSE` is the canonical unmodified Apache-2.0 text (md5 `3b83ef96387f14655fc854ddc3c6bd57`, byte-identical to the copy shipped by the OS's licence packages); `NOTICE` present; workspace `license = "Apache-2.0"` and **all 8 crates** inherit it (`license.workspace = true`); **every `.rs` file starts with `// SPDX-License-Identifier: Apache-2.0`** (checked: none missing); CONTRIBUTING.md states contributions are under Apache-2.0 per section 5. No MIT/dual licensing and no CC BY-SA for project material. Resolved dependency licences for the Linux build (`cargo tree --format "{l}"`): only `MIT`, `Apache-2.0` (also `OR` combinations and `WITH LLVM-exception`), `Zlib`, `BSD-2/3-Clause`, `ISC`, `Unlicense`, `MIT-0`, `0BSD`, `CC0-1.0`, `Unicode-3.0`; `r-efi` is dual-licensed `MIT OR Apache-2.0 OR LGPL-2.1-or-later` and is used under MIT/Apache-2.0. No GPL/AGPL/CC BY-SA/NC. Recorded in THIRD_PARTY.md (no third-party font or audio is bundled; the font and all art/audio are original).

**5. No Nintendo / Game Freak / franchise assets or names.** Output of `grep -RIn -i "pok" --exclude-dir=target --exclude-dir=.git --exclude=Cargo.lock .` (excluding this report):

```
./assets/content/towns/vila_azul/dialogues/vila_azul.ron:29:  ... "Já falaste com a Professora Amélia? ..." en: "Have you spoken to Professor Amélia yet? ..."
```

The single hit is the English word "s**pok**en" in a dialogue gloss ("Have you **spoken** to Professor Amélia yet?"), a false positive. Re-running the grep also matches the lines of this report that quote the command. `Cargo.lock` has no match. The game's own vocabulary is Bicho / Bichodex / Selo / Mestre; all art, music and names are original; the README states the independence disclaimer. During development a franchise keyword accidentally appeared twice (a desktop-entry keyword and one sentence in CONTRIBUTING.md); both were caught by this grep and removed before the final commit.
