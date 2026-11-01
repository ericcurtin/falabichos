// SPDX-License-Identifier: Apache-2.0
//! `falabichos-preview <outdir>`: render contact sheets (tiles, bichos, characters, font) as PNG and
//! every song as WAV, so artists and musicians can check their work without running the game.

use falabichos_assets::Assets;
use falabichos_media::synth::{SAMPLE_RATE, Song, wav_bytes};
use falabichos_media::{ArtBank, Canvas};
use std::path::Path;

fn sheet(bank: &ArtBank, cat: &str, per_row: usize, scale: i32) -> Canvas {
    let names = bank.names(cat);
    let rows = names.len().div_ceil(per_row).max(1);
    let cell = 20;
    let mut c = Canvas::new(per_row * cell, rows * cell);
    c.clear(bank.pal('k'));
    for (i, n) in names.iter().enumerate() {
        let s = bank.get(cat, n).unwrap();
        let (x, y) = ((i % per_row) * cell + 2, (i / per_row) * cell + 2);
        c.blit(s, 0, x as i32, y as i32, 1, false);
    }
    c.upscaled(scale as usize)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = Path::new(args.first().map(String::as_str).unwrap_or("preview"));
    std::fs::create_dir_all(out).expect("create outdir");
    let local = Path::new("assets");
    let assets = if local.is_dir() {
        Assets::from_dir(local).unwrap_or_else(|_| Assets::embedded())
    } else {
        Assets::embedded()
    };
    let bank = match ArtBank::load(&assets) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("art errors:\n{}", e.join("\n"));
            std::process::exit(1);
        }
    };
    for (cat, per_row) in [("tiles", 8), ("bichos", 8), ("chars", 8), ("seals", 4)] {
        let png = sheet(&bank, cat, per_row, 4).to_png();
        std::fs::write(out.join(format!("{cat}.png")), png).expect("write png");
        println!("wrote {}/{cat}.png", out.display());
    }
    // font sample
    let mut c = Canvas::new(240, 80);
    c.clear(bank.pal('n'));
    let w = bank.pal('W');
    let lines = [
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        "abcdefghijklmnopqrstuvwxyz 0123456789",
        "Olá! Pão, coração, avó, avô, é, à, ü",
        "Português: ão õe ç ã ê ú í Ó É Á Ç",
        "IPA: ɐ ɛ ɔ ə ɨ ʃ ʒ ʁ ɾ ɲ ʎ ŋ ɫ ˈ ː ɐ̃ ẽ",
    ];
    for (i, l) in lines.iter().enumerate() {
        bank.font.draw(&mut c, l, 2, 2 + i as i32 * 14, w, 1);
    }
    std::fs::write(out.join("font.png"), c.upscaled(4).to_png()).expect("write png");
    println!("wrote {}/font.png", out.display());
    for p in assets.list("music/").filter(|p| p.ends_with(".song")) {
        let name = p.trim_start_matches("music/").trim_end_matches(".song");
        match Song::parse(assets.text(p).unwrap_or("")) {
            Ok(s) => {
                std::fs::write(
                    out.join(format!("{name}.wav")),
                    wav_bytes(&s.render(), SAMPLE_RATE),
                )
                .expect("write wav");
                println!("wrote {}/{name}.wav ({:.1}s)", out.display(), s.duration_secs());
            }
            Err(e) => eprintln!("{p}: {e}"),
        }
    }
}
