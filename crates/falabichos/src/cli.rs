// SPDX-License-Identifier: Apache-2.0
//! Command-line flags (desktop; harmless on mobile where there are none).

use std::path::PathBuf;

#[derive(Debug, Default, Clone)]
pub struct Args {
    /// Run the headless simulation and exit (no window).
    pub headless_sim: bool,
    /// Replay the playthrough and write PNG screenshots here, then exit (no window).
    pub screenshots: Option<PathBuf>,
    /// Exit automatically after this many seconds (CI boot test).
    pub autoquit: Option<f32>,
    /// Feed scripted key presses (title -> new game -> walking) for smoke tests.
    pub smoke: bool,
    /// Save a screenshot of the real window (GPU-rendered) to this PNG path, after `window_shot_after` s.
    pub window_shot: Option<PathBuf>,
    pub window_shot_after: f32,
    /// Show touch controls even on desktop.
    pub touch: bool,
    pub data_dir: Option<PathBuf>,
    pub assets_dir: Option<PathBuf>,
    pub fullscreen: bool,
    pub help: bool,
    pub version: bool,
}

pub const HELP: &str = "falabichos [options]\n\
  --headless-sim        run the scripted headless playthrough (no window) and exit\n\
  --screenshots DIR     replay the playthrough, write PNG screenshots to DIR, exit\n\
  --autoquit SECS       quit automatically after SECS seconds\n\
  --smoke               inject scripted key presses (for CI boot tests)\n\
  --window-shot FILE    save a PNG of the real window after --window-shot-after SECS (default 8)\n\
  --touch               show the on-screen touch controls\n\
  --fullscreen          start fullscreen\n\
  --data-dir DIR        where saves and settings are stored\n\
  --assets DIR          load assets from DIR instead of the embedded copy (modding)\n\
  --version, --help\n";

pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut a = Args {
        window_shot_after: 8.0,
        ..Args::default()
    };
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--headless-sim" => a.headless_sim = true,
            "--screenshots" => a.screenshots = Some(PathBuf::from(value("--screenshots")?)),
            "--autoquit" => {
                a.autoquit = Some(
                    value("--autoquit")?
                        .parse()
                        .map_err(|_| "--autoquit needs a number")?,
                )
            }
            "--smoke" => a.smoke = true,
            "--window-shot" => a.window_shot = Some(PathBuf::from(value("--window-shot")?)),
            "--window-shot-after" => {
                a.window_shot_after = value("--window-shot-after")?
                    .parse()
                    .map_err(|_| "--window-shot-after needs a number")?
            }
            "--touch" => a.touch = true,
            "--fullscreen" => a.fullscreen = true,
            "--data-dir" => a.data_dir = Some(PathBuf::from(value("--data-dir")?)),
            "--assets" => a.assets_dir = Some(PathBuf::from(value("--assets")?)),
            "--help" | "-h" => a.help = true,
            "--version" | "-V" => a.version = true,
            // Android/iOS and the OS may pass extra arguments (e.g. macOS `-psn_...`): ignore unknown ones.
            _ => {}
        }
    }
    Ok(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &[&str]) -> Result<Args, String> {
        parse(s.iter().map(|x| x.to_string()))
    }

    #[test]
    fn parses_flags() {
        let a = p(&[
            "--headless-sim",
            "--autoquit",
            "2.5",
            "--data-dir",
            "/tmp/x",
            "--touch",
        ])
        .unwrap();
        assert!(a.headless_sim && a.touch);
        assert_eq!(a.autoquit, Some(2.5));
        assert_eq!(a.data_dir.unwrap().to_str(), Some("/tmp/x"));
    }

    #[test]
    fn errors_and_unknowns() {
        assert!(p(&["--autoquit"]).is_err());
        assert!(p(&["--autoquit", "abc"]).is_err());
        assert!(p(&["-psn_0_1234", "--smoke"]).unwrap().smoke);
    }
}
