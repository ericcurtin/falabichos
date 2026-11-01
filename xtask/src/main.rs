// SPDX-License-Identifier: Apache-2.0
//! `cargo xtask`: Rust automation for building, checking and packaging Falabichos on every platform.
//!
//! Every command builds a *plan* (a list of external commands). `--dry-run` prints the plan without
//! running it, which is also how the plans are unit-tested.

use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cmd {
    pub program: String,
    pub args: Vec<String>,
    pub dir: Option<PathBuf>,
    pub env: Vec<(String, String)>,
}

impl Cmd {
    fn new(program: &str, args: &[&str]) -> Cmd {
        Cmd {
            program: program.into(),
            args: args.iter().map(|s| s.to_string()).collect(),
            dir: None,
            env: vec![],
        }
    }
    fn cargo(args: &[&str]) -> Cmd {
        Cmd::new("cargo", args)
    }
    fn in_dir(mut self, d: impl Into<PathBuf>) -> Cmd {
        self.dir = Some(d.into());
        self
    }
    fn env(mut self, k: &str, v: &str) -> Cmd {
        self.env.push((k.into(), v.into()));
        self
    }
    fn show(&self) -> String {
        let mut s = String::new();
        if let Some(d) = &self.dir {
            s.push_str(&format!("(in {}) ", d.display()));
        }
        for (k, v) in &self.env {
            s.push_str(&format!("{k}={v} "));
        }
        s.push_str(&self.program);
        for a in &self.args {
            s.push(' ');
            s.push_str(a);
        }
        s
    }
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const ANDROID_ABIS: [(&str, &str); 3] = [
    ("arm64-v8a", "aarch64-linux-android"),
    ("armeabi-v7a", "armv7-linux-androideabi"),
    ("x86_64", "x86_64-linux-android"),
];

/// Must match `minSdk` in mobile/android/app/build.gradle (oboe/cpal link `-laaudio`, API 26+).
const ANDROID_MIN_SDK: &str = "28";

pub fn plan(task: &str, extra: &[String], host_os: &str) -> Result<Vec<Cmd>, String> {
    let release = !extra.iter().any(|a| a == "--debug");
    Ok(match task {
        "ci" => vec![
            Cmd::cargo(&["fmt", "--all", "--", "--check"]),
            Cmd::cargo(&["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]),
            Cmd::cargo(&["test", "--workspace"]),
            Cmd::cargo(&[
                "run",
                "-q",
                "-p",
                "falabichos-tools",
                "--bin",
                "falabichos-lint",
                "--",
                "--quiet",
            ]),
            Cmd::cargo(&["run", "-q", "-p", "falabichos", "--", "--headless-sim"]),
        ],
        "lint" => vec![Cmd::cargo(&[
            "run",
            "-q",
            "-p",
            "falabichos-tools",
            "--bin",
            "falabichos-lint",
        ])],
        "desktop" => {
            let mut a = vec!["build", "-p", "falabichos"];
            if release {
                a.push("--release");
            }
            vec![Cmd::cargo(&a)]
        }
        "run" => vec![Cmd::cargo(&["run", "-p", "falabichos"])],
        "check-targets" => vec![
            Cmd::cargo(&[
                "check",
                "--target",
                "x86_64-pc-windows-gnu",
                "-p",
                "falabichos-core",
                "-p",
                "falabichos-assets",
                "-p",
                "falabichos-content",
                "-p",
                "falabichos-media",
                "-p",
                "falabichos-game",
            ]),
            Cmd::cargo(&[
                "check",
                "--target",
                "aarch64-linux-android",
                "-p",
                "falabichos-core",
                "-p",
                "falabichos-assets",
                "-p",
                "falabichos-content",
                "-p",
                "falabichos-media",
                "-p",
                "falabichos-game",
            ]),
        ],
        "package-linux" => {
            let target = "x86_64-unknown-linux-gnu";
            let name = format!("falabichos-{VERSION}-linux-x86_64");
            vec![
                Cmd::cargo(&["build", "--release", "-p", "falabichos", "--target", target]),
                Cmd::new(
                    "sh",
                    &[
                        "-c",
                        &format!(
                            "set -e; rm -rf dist/{name}; mkdir -p dist/{name}; cp \"$(cargo metadata --format-version 1 --no-deps | sed -n 's/.*\"target_directory\":\"\\([^\"]*\\)\".*/\\1/p')/{target}/release/falabichos\" dist/{name}/; cp LICENSE NOTICE README.md dist/{name}/; tar -C dist -czf dist/{name}.tar.gz {name}"
                        ),
                    ],
                ),
            ]
        }
        "package-macos" => {
            let name = format!("Falabichos-{VERSION}-macos");
            vec![
                Cmd::cargo(&[
                    "build",
                    "--release",
                    "-p",
                    "falabichos",
                    "--target",
                    "aarch64-apple-darwin",
                ]),
                Cmd::cargo(&[
                    "build",
                    "--release",
                    "-p",
                    "falabichos",
                    "--target",
                    "x86_64-apple-darwin",
                ]),
                Cmd::new(
                    "sh",
                    &[
                        "-c",
                        &format!(
                            "set -e; T=\"$(cargo metadata --format-version 1 --no-deps | sed -n 's/.*\"target_directory\":\"\\([^\"]*\\)\".*/\\1/p')\"; \
                     APP=dist/Falabichos.app; rm -rf \"$APP\"; mkdir -p \"$APP/Contents/MacOS\" \"$APP/Contents/Resources\"; \
                     lipo -create \"$T/aarch64-apple-darwin/release/falabichos\" \"$T/x86_64-apple-darwin/release/falabichos\" -output \"$APP/Contents/MacOS/falabichos\"; \
                     cp packaging/macos/Info.plist \"$APP/Contents/Info.plist\"; cp LICENSE NOTICE \"$APP/Contents/Resources/\"; \
                     hdiutil create -volname Falabichos -srcfolder \"$APP\" -ov -format UDZO dist/{name}.dmg"
                        ),
                    ],
                ),
            ]
        }
        "package-windows" => {
            let name = format!("falabichos-{VERSION}-windows-x86_64");
            vec![
                Cmd::cargo(&[
                    "build",
                    "--release",
                    "-p",
                    "falabichos",
                    "--target",
                    "x86_64-pc-windows-msvc",
                ]),
                Cmd::new(
                    "pwsh",
                    &[
                        "-NoProfile",
                        "-Command",
                        &format!(
                            "$ErrorActionPreference='Stop'; $t=(cargo metadata --format-version 1 --no-deps | ConvertFrom-Json).target_directory; \
                     New-Item -ItemType Directory -Force dist/{name} | Out-Null; \
                     Copy-Item \"$t/x86_64-pc-windows-msvc/release/falabichos.exe\" dist/{name}/; Copy-Item LICENSE,NOTICE,README.md dist/{name}/; \
                     Compress-Archive -Force -Path dist/{name} -DestinationPath dist/{name}.zip"
                        ),
                    ],
                ),
            ]
        }
        "android" => {
            let mut v = vec![];
            let mut ndk: Vec<&str> = vec!["ndk"];
            for (abi, _) in ANDROID_ABIS {
                ndk.push("-t");
                ndk.push(abi);
            }
            ndk.extend([
                "--platform",
                ANDROID_MIN_SDK,
                "-o",
                "mobile/android/app/src/main/jniLibs",
                "build",
                "-p",
                "falabichos",
                "--lib",
                "--features",
                "android-game-activity",
            ]);
            if release {
                ndk.push("--release");
            }
            v.push(Cmd::cargo(&ndk));
            let gradle = if host_os == "windows" {
                "gradle.bat"
            } else {
                "gradle"
            };
            let task = if extra.iter().any(|a| a == "--aab") {
                "bundleRelease"
            } else if release {
                "assembleRelease"
            } else {
                "assembleDebug"
            };
            v.push(Cmd::new(gradle, &[task]).in_dir("mobile/android"));
            v
        }
        "ios" => {
            let sim = !extra.iter().any(|a| a == "--device");
            let target = if sim {
                "aarch64-apple-ios-sim"
            } else {
                "aarch64-apple-ios"
            };
            let sdk = if sim { "iphonesimulator" } else { "iphoneos" };
            vec![
                Cmd::cargo(&[
                    "build",
                    "--release",
                    "-p",
                    "falabichos",
                    "--lib",
                    "--target",
                    target,
                ]),
                Cmd::new(
                    "xcodebuild",
                    &[
                        "-project",
                        "mobile/ios/Falabichos.xcodeproj",
                        "-target",
                        "Falabichos",
                        "-sdk",
                        sdk,
                        "-configuration",
                        "Release",
                        // the Rust library is arm64 only; Xcode would also link x86_64
                        "ARCHS=arm64",
                        "CODE_SIGNING_ALLOWED=NO",
                        "build",
                    ],
                )
                .env("FALABICHOS_RUST_TARGET", target),
            ]
        }
        other => return Err(format!("unknown task `{other}`. Try `cargo xtask help`.")),
    })
}

const HELP: &str = "cargo xtask <command> [--dry-run] [--debug]\n\n\
  ci               fmt --check, clippy -D warnings, tests, content lint, headless sim\n\
  lint             validate all game content (falabichos-lint)\n\
  desktop          build the desktop client (release)\n\
  run              run the desktop client\n\
  check-targets    cargo check the pure-Rust crates for Windows (gnu) and Android\n\
  package-linux    dist/falabichos-<ver>-linux-x86_64.tar.gz\n\
  package-macos    dist/Falabichos.app and a .dmg (run on macOS)\n\
  package-windows  dist/falabichos-<ver>-windows-x86_64.zip (run on Windows)\n\
  android          build the APK with cargo-ndk + Gradle (--aab for an app bundle)\n\
  ios              build for the iOS simulator (--device for hardware) with Xcode\n\n\
  --dry-run        print the commands without running them\n";

fn which(prog: &str) -> bool {
    let sep = if cfg!(windows) { ';' } else { ':' };
    std::env::var("PATH")
        .map(|p| {
            p.split(sep).any(|d| {
                Path::new(d).join(prog).exists() || Path::new(d).join(format!("{prog}.exe")).exists()
            })
        })
        .unwrap_or(false)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let task = args.first().map(String::as_str).unwrap_or("help");
    if matches!(task, "help" | "--help" | "-h") {
        print!("{HELP}");
        return;
    }
    let dry = args.iter().any(|a| a == "--dry-run");
    let extra: Vec<String> = args.iter().skip(1).cloned().collect();
    let cmds = match plan(task, &extra, std::env::consts::OS) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    for c in &cmds {
        println!("$ {}", c.show());
        if dry {
            continue;
        }
        if !which(&c.program) {
            eprintln!(
                "error: `{}` was not found on PATH. See docs/BUILDING.md for the prerequisites of `cargo xtask {task}`.",
                c.program
            );
            std::process::exit(127);
        }
        let mut cmd = Command::new(&c.program);
        cmd.args(&c.args).current_dir(
            c.dir
                .as_ref()
                .map(|d| root.join(d))
                .unwrap_or_else(|| root.clone()),
        );
        for (k, v) in &c.env {
            cmd.env(k, v);
        }
        match cmd.status() {
            Ok(s) if s.success() => {}
            Ok(s) => std::process::exit(s.code().unwrap_or(1)),
            Err(e) => {
                eprintln!("failed to run {}: {e}", c.program);
                std::process::exit(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ci_plan_has_all_gates() {
        let p = plan("ci", &[], "linux").unwrap();
        let text: Vec<String> = p.iter().map(Cmd::show).collect();
        assert!(text[0].contains("fmt --all -- --check"));
        assert!(text[1].contains("clippy --workspace --all-targets -- -D warnings"));
        assert!(text.iter().any(|t| t.contains("falabichos-lint")));
        assert!(text.iter().any(|t| t.contains("--headless-sim")));
    }

    #[test]
    fn android_plan_builds_three_abis_then_gradle() {
        let p = plan("android", &[], "linux").unwrap();
        let ndk = p[0].show();
        for abi in ["arm64-v8a", "armeabi-v7a", "x86_64"] {
            assert!(ndk.contains(abi), "{ndk}");
        }
        assert!(ndk.contains("android-game-activity"));
        assert!(ndk.contains("--platform 28"), "{ndk}");
        assert_eq!(p[1].args, vec!["assembleRelease"]);
        assert_eq!(p[1].dir.as_deref(), Some(Path::new("mobile/android")));
        let aab = plan("android", &["--aab".into()], "linux").unwrap();
        assert_eq!(aab[1].args, vec!["bundleRelease"]);
        assert_eq!(plan("android", &[], "windows").unwrap()[1].program, "gradle.bat");
    }

    #[test]
    fn ios_targets_simulator_by_default() {
        let p = plan("ios", &[], "macos").unwrap();
        assert!(p[0].show().contains("aarch64-apple-ios-sim"));
        let d = plan("ios", &["--device".into()], "macos").unwrap();
        assert!(d[0].show().contains("--target aarch64-apple-ios"));
        assert!(!d[0].show().contains("-sim"));
        // the hand-written Xcode project has no shared scheme: build by target
        assert!(p[1].show().contains("ARCHS=arm64"));
        assert!(p[1].show().contains("-target Falabichos") && !p[1].show().contains("-scheme"));
    }

    #[test]
    fn no_web_targets_anywhere() {
        for t in [
            "ci",
            "desktop",
            "check-targets",
            "package-linux",
            "package-macos",
            "package-windows",
            "android",
            "ios",
        ] {
            for c in plan(t, &[], "linux").unwrap() {
                assert!(!c.show().to_lowercase().contains("wasm"), "{t}");
            }
        }
    }

    #[test]
    fn unknown_task_errors() {
        assert!(plan("nope", &[], "linux").is_err());
    }
}
