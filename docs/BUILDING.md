# Building and packaging

Rust toolchain: **1.95.0** (pinned in `rust-toolchain.toml`; rustup installs it automatically). Bevy is pinned exactly at **0.19.1**.
All build automation is Rust: `cargo xtask <command>` (`cargo xtask help`). Add `--dry-run` to any command to print the steps without running them.

| Command | What it does |
|---------|--------------|
| `cargo xtask ci` | `fmt --check`, `clippy -D warnings`, tests, content lint, headless playthrough |
| `cargo xtask desktop` | release build of the client for this machine |
| `cargo xtask check-targets` | `cargo check` the pure-Rust crates for Windows and Android |
| `cargo xtask package-linux` | `dist/falabichos-<ver>-linux-x86_64.tar.gz` |
| `cargo xtask package-macos` | universal `dist/Falabichos.app` + `.dmg` (**run on macOS**) |
| `cargo xtask package-windows` | `dist/falabichos-<ver>-windows-x86_64.zip` (**run on Windows**, needs PowerShell 7) |
| `cargo xtask android [--aab]` | cargo-ndk + Gradle → APK (or AAB) |
| `cargo xtask ios [--device]` | static library + Xcode build for the simulator (or a device) |

Profiles (workspace `Cargo.toml`): **dev** builds workspace crates at `opt-level = 1` and all dependencies at `opt-level = 3`, so `cargo run` is playable;
**release** uses `lto = "thin"`, `codegen-units = 1`, `strip = true`. Bevy default features are **off**; the client enables only 2D rendering, windowing,
input, audio (Ogg/WAV) and gamepad. There is no web/WASM feature anywhere.

## Linux

System packages (names for Debian/Ubuntu, Fedora and Arch are in the [README](../README.md#linux)): a C toolchain, `pkg-config`, ALSA, udev, Wayland, xkbcommon, X11
(`libX11 libXcursor libXi libXrandr`), a GL or Vulkan loader. Optional: `clang` and `mold` for faster links.

```sh
cargo run -p falabichos                  # debug
cargo run --release -p falabichos
```

### Immutable distributions (Silverblue, Kinoite, Bazzite, ...) via toolbox

```sh
toolbox create falabichos-dev
toolbox run -c falabichos-dev sudo dnf install -y gcc clang mold pkgconf-pkg-config alsa-lib-devel systemd-devel wayland-devel \
  libxkbcommon-devel libX11-devel libXcursor-devel libXi-devel libXrandr-devel mesa-libGL-devel mesa-libEGL-devel \
  vulkan-loader-devel mesa-vulkan-drivers xorg-x11-server-Xvfb
toolbox run -c falabichos-dev cargo run -p falabichos
```

(Without `sudo` rights, `podman exec --user root <container> dnf install ...` also works for a rootless toolbox container.)
Your home directory, and therefore `~/.cargo` and `~/.rustup`, is shared with the container. Set `CARGO_TARGET_DIR` *inside* the container call if you want a custom target directory
(environment variables are not inherited): `toolbox run -c falabichos-dev env CARGO_TARGET_DIR=... cargo ...`.

### Running without a display (CI, containers)

```sh
cargo run -p falabichos -- --headless-sim          # no window, no GPU, no audio device
cargo run -p falabichos -- --screenshots out/      # PNGs of the real screens, no window
# with a virtual display and software Vulkan (lavapipe):
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.x86_64.json WGPU_BACKEND=vulkan \
  xvfb-run -a cargo run -p falabichos -- --smoke --autoquit 20 --window-shot out/window.png --window-shot-after 14
```

`--window-shot` captures the GPU-rendered window via Bevy's screenshot API (it verifies the texture upload and integer scaling).

## macOS

Xcode command line tools + rustup. `cargo run -p falabichos` works directly (Metal). `cargo xtask package-macos` builds `aarch64-apple-darwin` and
`x86_64-apple-darwin`, merges them with `lipo`, wraps them in `Falabichos.app` (using `packaging/macos/Info.plist`) and creates a `.dmg`.
The result is **unsigned**; to distribute, sign and notarise it with your Apple Developer ID (`codesign --deep --options runtime ...`, `xcrun notarytool submit ...`).
You can add these as CI secrets in `.github/workflows/release.yml`.

## Windows

Install rustup with the **MSVC** toolchain and the Visual Studio Build Tools ("Desktop development with C++"). `cargo run -p falabichos`.
`cargo xtask package-windows` zips the release `.exe` with the licence files. Code signing (Authenticode) is not set up.
The `x86_64-pc-windows-gnu` target is used here only to type-check the code from Linux (`cargo check --target x86_64-pc-windows-gnu`).

## Android

Prerequisites: JDK 17, Android SDK (platform 34, build-tools 34), **NDK 26**, **cargo-ndk** (`cargo install cargo-ndk --locked`), Gradle 8.7+ on `PATH`,
and the Rust targets `rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android`.
Set `ANDROID_HOME` and `ANDROID_NDK_HOME`.

```sh
cargo xtask android            # release APK: mobile/android/app/build/outputs/apk/release/
cargo xtask android --debug    # debug APK
cargo xtask android --aab      # Play Store bundle
adb install -r mobile/android/app/build/outputs/apk/release/app-release-unsigned.apk
```

`cargo xtask android` runs `cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o mobile/android/app/src/main/jniLibs build -p falabichos --lib --features android-game-activity`
and then Gradle. The Android project (`mobile/android`) uses **GameActivity** (`androidx.games:games-activity`); `#[bevy_main]` generates `android_main`.
Release APKs are unsigned; sign with `apksigner` or configure `signingConfigs` in `mobile/android/app/build.gradle` from CI secrets.
Saves go to the app's internal data directory (`AndroidApp::internal_data_path`). The activity is locked to sensor-landscape and fullscreen; the game keeps
its touch controls inside conservative safe margins (see PLATFORM_STATUS.md for the safe-area limitation).

## iOS

macOS with Xcode 15+. `rustup target add aarch64-apple-ios aarch64-apple-ios-sim` (and `x86_64-apple-ios` for Intel simulators).

```sh
cargo xtask ios                # simulator (aarch64-apple-ios-sim), unsigned
cargo xtask ios --device       # hardware: needs a signing team/profile configured in Xcode
open mobile/ios/Falabichos.xcodeproj
```

The Xcode project (`mobile/ios`) has a **Run Script** build phase that runs `cargo build --release --lib -p falabichos --target <triple>`, produces
`libfalabichos.a` (the crate is built as `staticlib`) and links it with `main.m`, which calls the Rust `main_rs`. Orientation is landscape only.
For a device build set your `DEVELOPMENT_TEAM` in Xcode. Saves go to `~/Library/Application Support/Falabichos` inside the app sandbox.

## Environment variables and flags

| | |
|--|--|
| `--data-dir DIR` / `FALABICHOS_DATA_DIR` | where `save.ron` and `settings.ron` are stored |
| `--assets DIR` / `FALABICHOS_ASSETS` | load content/art/music from a directory instead of the embedded copy (modding, live editing) |
| `FALABICHOS_SAFE_INSETS=l,t,r,b` | override the touch-control safe margins (logical px) |
| `--touch` | show the touch controls on desktop (testing) |
| `--fullscreen`, F11 | fullscreen |

Default data directories: Linux `$XDG_DATA_HOME/falabichos` or `~/.local/share/falabichos`; macOS `~/Library/Application Support/Falabichos`;
Windows `%APPDATA%\Falabichos`; Android the app's internal storage; iOS the app container.
