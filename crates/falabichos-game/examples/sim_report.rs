// SPDX-License-Identifier: Apache-2.0
//! `cargo run -p falabichos-game --example sim_report [screenshot_dir]`: print the headless report.
use falabichos_assets::Assets;
use std::sync::Arc;

fn main() {
    let assets = Arc::new(Assets::embedded());
    if let Some(dir) = std::env::args().nth(1) {
        match falabichos_game::sim::screenshots(assets.clone(), dir.into()) {
            Ok(n) => println!("wrote {n} screenshots"),
            Err(e) => {
                eprintln!("screenshot run failed: {e}");
                std::process::exit(1);
            }
        }
    }
    match falabichos_game::sim::run_all(assets) {
        Ok(lines) => lines.iter().for_each(|l| println!("{l}")),
        Err(e) => {
            eprintln!("FAILED: {e}");
            std::process::exit(1);
        }
    }
}
