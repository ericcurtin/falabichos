// SPDX-License-Identifier: Apache-2.0
//! Per-platform glue: data directory and safe-area insets.

use crate::cli::Args;
use falabichos_game::touch_art::Insets;
use std::path::PathBuf;

pub fn is_mobile() -> bool {
    cfg!(any(target_os = "android", target_os = "ios"))
}

/// Where saves and settings live.
///
/// Priority: `--data-dir`, `FALABICHOS_DATA_DIR`, the Android app's internal storage, then the
/// platform default from [`falabichos_game::default_data_dir`].
pub fn data_dir(args: &Args) -> PathBuf {
    if let Some(d) = &args.data_dir {
        return d.clone();
    }
    #[cfg(target_os = "android")]
    {
        if let Some(p) = bevy::android::ANDROID_APP
            .get()
            .and_then(|a| a.internal_data_path())
        {
            return p;
        }
    }
    falabichos_game::default_data_dir()
}

/// Safe-area insets in *logical* pixels (notch, rounded corners, home indicator).
///
/// winit 0.30 (used by Bevy 0.19) has no safe-area query, so on phones we use conservative
/// landscape defaults. `FALABICHOS_SAFE_INSETS=left,top,right,bottom` overrides them (handy for
/// testing a particular device). When Bevy moves to winit 0.31 this should query the OS instead.
pub fn safe_insets() -> Insets {
    if let Ok(v) = std::env::var("FALABICHOS_SAFE_INSETS") {
        let n: Vec<f32> = v.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        if n.len() == 4 {
            return Insets {
                left: n[0],
                top: n[1],
                right: n[2],
                bottom: n[3],
            };
        }
    }
    if cfg!(target_os = "ios") {
        Insets {
            left: 47.0,
            top: 0.0,
            right: 47.0,
            bottom: 21.0,
        }
    } else if cfg!(target_os = "android") {
        Insets {
            left: 32.0,
            top: 0.0,
            right: 32.0,
            bottom: 12.0,
        }
    } else {
        Insets::default()
    }
}
