// SPDX-License-Identifier: Apache-2.0
//! Integration tests: the real game loop driven by synthetic input, no display required.

use falabichos_assets::Assets;
use falabichos_game::sim;
use falabichos_game::{Game, GameConfig, Input, MemoryStorage};
use falabichos_media::Canvas;
use std::sync::Arc;

fn assets() -> Arc<Assets> {
    Arc::new(Assets::embedded())
}

#[test]
fn core_simulation_passes() {
    let content = falabichos_content::load_strict(&assets()).expect("content loads");
    let log = sim::core_sim(&content).expect("core sim");
    assert!(log.iter().any(|l| l.contains("caught the Galo")));
}

#[test]
fn full_playthrough_reaches_the_finale() {
    let log = sim::playthrough(assets(), None).unwrap_or_else(|e| panic!("playthrough failed: {e}"));
    assert!(log.iter().any(|l| l.contains("finale reached")), "{log:?}");
    assert!(
        log.iter().any(|l| l.contains("save -> quit -> resume OK")),
        "{log:?}"
    );
    assert!(
        log.iter().any(|l| l.contains("Mestre Duarte beaten")),
        "Part 2 town 3 must be reachable and beatable: {log:?}"
    );
}

#[test]
fn boots_to_title_and_draws_nonblank_frames() {
    let mut g = Game::new(
        assets(),
        Box::new(MemoryStorage::default()),
        GameConfig::default(),
    )
    .unwrap();
    assert_eq!(g.scene_name(), "title");
    for _ in 0..5 {
        g.tick(&Input::default());
    }
    let mut c = Canvas::new(falabichos_game::LOGICAL_W, falabichos_game::LOGICAL_H);
    g.draw(&mut c);
    let distinct: std::collections::HashSet<[u8; 4]> = (0..c.h)
        .flat_map(|y| (0..c.w).map(move |x| (x, y)))
        .map(|(x, y)| c.get(x, y))
        .collect();
    assert!(
        distinct.len() > 8,
        "title frame should not be flat ({} colours)",
        distinct.len()
    );
}

#[test]
fn every_map_renders_without_panicking() {
    // Place the player at the first walkable tile of every map and draw one frame.
    let a = assets();
    let content = falabichos_content::load_strict(&a).unwrap();
    for id in content.maps.keys() {
        let mut g = Game::new(
            a.clone(),
            Box::new(MemoryStorage::default()),
            GameConfig::default(),
        )
        .unwrap();
        g.debug_teleport(id);
        let mut c = Canvas::new(falabichos_game::LOGICAL_W, falabichos_game::LOGICAL_H);
        g.tick(&Input::default());
        g.draw(&mut c);
    }
}

#[test]
fn large_text_and_high_contrast_render() {
    let mut g = Game::new(
        assets(),
        Box::new(MemoryStorage::default()),
        GameConfig::default(),
    )
    .unwrap();
    g.settings.high_contrast = true;
    g.settings.text_size = falabichos_core::save::TextSize::Large;
    g.settings.reduce_motion = true;
    g.new_game();
    for _ in 0..200 {
        g.tick(&Input::default());
    }
    let mut c = Canvas::new(falabichos_game::LOGICAL_W, falabichos_game::LOGICAL_H);
    g.draw(&mut c);
    assert_eq!(g.scene_name(), "dialogue");
}
