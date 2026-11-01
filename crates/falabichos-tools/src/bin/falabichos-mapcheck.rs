// SPDX-License-Identifier: Apache-2.0
//! `falabichos-mapcheck <map_id>`: print an annotated ASCII view of a map and its problems.
//! Overlay legend: `@` NPC, `>` warp, `?` sign, `!` trigger.

use falabichos_assets::Assets;
use falabichos_content::{Severity, load};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(id) = args.first().filter(|a| !a.starts_with("--")) else {
        eprintln!("usage: falabichos-mapcheck <map_id|--list> [--assets DIR]");
        std::process::exit(2);
    };
    let local = Path::new("assets");
    let assets = match args
        .iter()
        .position(|a| a == "--assets")
        .and_then(|i| args.get(i + 1))
    {
        Some(d) => Assets::from_dir(Path::new(d)).expect("readable assets dir"),
        None if local.is_dir() => Assets::from_dir(local).unwrap_or_else(|_| Assets::embedded()),
        None => Assets::embedded(),
    };
    let loaded = load(&assets);
    if id == "--list" {
        for m in loaded.content.maps.values() {
            println!("{} ({}x{}) {}", m.id, m.width(), m.height(), m.name);
        }
        return;
    }
    let Some(m) = loaded.content.maps.get(id.as_str()) else {
        eprintln!(
            "unknown map `{id}`. Known maps: {}",
            loaded.content.maps.keys().cloned().collect::<Vec<_>>().join(", ")
        );
        std::process::exit(2);
    };
    let mut grid: Vec<Vec<char>> = m.rows.iter().map(|r| r.chars().collect()).collect();
    let mut put = |x: i32, y: i32, c: char| {
        if let Some(row) = grid.get_mut(y as usize)
            && let Some(cell) = row.get_mut(x as usize)
        {
            *cell = c;
        }
    };
    for n in &m.npcs {
        put(n.x, n.y, '@');
    }
    for w in &m.warps {
        put(w.x, w.y, '>');
    }
    for s in &m.signs {
        put(s.x, s.y, '?');
    }
    for t in &m.triggers {
        put(t.x, t.y, '!');
    }
    println!("{} ({}) {}x{}", m.name, m.id, m.width(), m.height());
    print!("    ");
    for x in 0..m.width() {
        print!("{}", x % 10);
    }
    println!();
    for (y, row) in grid.iter().enumerate() {
        println!("{y:>3} {}", row.iter().collect::<String>());
    }
    println!("\nwarps:");
    for w in &m.warps {
        println!("  ({},{}) -> {} ({},{})", w.x, w.y, w.map, w.to_x, w.to_y);
    }
    println!("npcs:");
    for n in &m.npcs {
        println!("  {} at ({},{}) dialogue={:?}", n.id, n.x, n.y, n.dialogue);
    }
    let mine: Vec<_> = loaded
        .diagnostics
        .iter()
        .filter(|d| d.location == format!("map:{}", m.id))
        .collect();
    println!("\nproblems:");
    if mine.is_empty() {
        println!("  none");
    }
    for d in &mine {
        println!("  {d}");
    }
    if mine.iter().any(|d| d.severity == Severity::Error) {
        std::process::exit(1);
    }
}
