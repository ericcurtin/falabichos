# Platform status: what was actually verified

This is the honest matrix. **"Built" and "ran" mean it really happened on the author's machine, not that a CI file exists.** If a cell says "not done" it was not done.

Verification environment: one Linux x86_64 host (Fedora-based, immutable), **no sudo, no GPU display**. Linux builds were made inside a
Fedora 42 toolbox container. There was no macOS, Windows, Android SDK/NDK or Xcode available.

Legend: ✅ done and passed · ❌ attempted, failed (reason given) · ⬜ not done (not possible here)

| Platform | `cargo check` pure-Rust crates ¹ | `cargo check` whole client ² | Built (client binary / package) | Ran |
|----------|:---:|:---:|:---:|:---:|
| **Linux x86_64** | ✅ (host + container) | ✅ | ✅ debug **and** release (`cargo xtask package-linux`: thin LTO, stripped, 14 MB `.tar.gz`) | ✅ under Xvfb + software Vulkan (lavapipe) |
| **Windows x86_64** | ✅ `x86_64-pc-windows-gnu` | ✅ `x86_64-pc-windows-gnu` | ⬜ never linked; MSVC target untouched | ⬜ |
| **macOS arm64** | ✅ `aarch64-apple-darwin` | ❌ stops in `blake3`'s C build script: no Apple C toolchain/SDK on Linux | ⬜ | ⬜ |
| **Android arm64** | ✅ `aarch64-linux-android` | ❌ stops in `android-activity`'s C++ build script: no NDK clang on this host | ⬜ the Gradle project has never been run | ⬜ |
| **iOS arm64** | ✅ `aarch64-apple-ios` | ❌ stops at `xcrun --show-sdk-path`: no Xcode | ⬜ the Xcode project has never been opened | ⬜ |

¹ `falabichos-core`, `-assets`, `-content`, `-media`, `-game`: i.e. **all the game logic, rendering and scenes**.
² Type-checking the Bevy client crate (window, input, audio, mobile entry points).

### What this means

- **Linux** is the only platform the author can vouch for end to end. Details below.
- **Windows**: all the game logic and the whole Bevy client *type-check* for a Windows target, which catches most portability mistakes, but nothing was linked or run.
  Bevy supports Windows officially; CI (`.github/workflows/ci.yml`) is configured to build and test it on `windows-latest`.
- **macOS, Android, iOS**: *all the game logic and rendering compile for these targets*. The platform glue is **written but unverified**:
  - Android: `mobile/android` (Gradle + `GameActivity`), `cargo xtask android` (cargo-ndk), `#[bevy_main]` → `android_main`, internal-storage data directory,
    sensor-landscape manifest, immersive mode. The client's `cfg(target_os = "android")` code has *not* been compiled.
  - iOS: `mobile/ios/Falabichos.xcodeproj` is **hand-written** (`project.pbxproj`) and may need a small fix when first opened in Xcode; the `main_rs`
    entry point (`cfg(target_os = "ios")`) has not been compiled.
  - macOS: `cargo xtask package-macos` (`lipo` + `.app` + `.dmg`) has not been run.
  - The CI jobs for these platforms were written following the official docs and **have never executed**.
- **Expect first-run fixes** on macOS, Windows, Android and iOS. Reports and PRs are very welcome.

## Linux: exact verification record

All commands were run in the repository; the container is `toolbox` `falabichos-dev` (Fedora 42).

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace` | see FINAL_REPORT.md for the final count (all passing) |
| `cargo run -p falabichos-tools --bin falabichos-lint` | 0 errors, 0 warnings |
| `falabichos --headless-sim` (debug **and** the packaged release binary) | passes: title → new game → first Bicho → save → resume → Mestre 1 → road → Mestra 2 → finale |
| `xvfb-run ... falabichos --smoke --autoquit 20` with lavapipe | starts, renders, runs scripted input, saves, exits 0, **no panics** |
| `--window-shot` | PNG of the real GPU-rendered window (4× integer scale, letterboxed): `docs/screenshots/window-xvfb-lavapipe.png` |
| `--touch` under Xvfb | on-screen D-pad / A / B / MENU / HINT render correctly: `docs/screenshots/touch-controls.png` |

**Not verified on Linux:** audio *output* (the container has no sound device: Bevy logs an ALSA warning and continues; the synthesiser and the clip loader are covered by unit tests, but nobody has *heard* the music);
a physical GPU; Wayland sessions (X11 only, via Xvfb); gamepads (code path is standard Bevy `Gamepad` input, no controller was attached); real touch screens.

## Known platform limitations

- **Safe areas (notches, home indicator)**: Bevy 0.19 uses winit 0.30, which has no safe-area query. On phones the touch controls use conservative built-in margins
  (`platform::safe_insets`), overridable with `FALABICHOS_SAFE_INSETS=left,top,right,bottom`. The proper fix is an OS query once Bevy moves to winit 0.31.
- **DPI**: the frame uses physical-pixel integer scaling (`scale_factor` aware) and the touch layout scales with the window's DPI factor; verified only by unit tests and on a 1× virtual display.
- **Pause/resume**: `AppLifecycle::WillSuspend/Suspended` → `Game::on_suspend()` autosaves (when not mid-dialogue). Tested in logic only; the lifecycle events themselves can only fire on a device.
- **Quit on mobile** is hidden from the title menu (`GameConfig::can_quit`).
