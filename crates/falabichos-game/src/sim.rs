// SPDX-License-Identifier: Apache-2.0
//! Headless simulation: scripted playthroughs that need no display, no GPU and no audio.
//!
//! * [`core_sim`] drives `falabichos-core` directly (battle / catch / SRS / save / migration).
//! * [`playthrough`] drives the real [`Game`] with synthetic key presses, from the title screen
//!   to the end of the vertical slice, using a path-finder over the real map data.
//! * [`screenshots`] replays the same script and writes PNGs of key screens.

use crate::battle_ui::BPhase;
use crate::input::Input;
use crate::storage::Storage;
use crate::{Game, GameConfig, Scene};
use falabichos_assets::Assets;
use falabichos_core::battle::{Battle, Outcome, settle};
use falabichos_core::exercise::{Answer, ExKind, Exercise};
use falabichos_core::model::{Dir, TileKind};
use falabichos_core::normalize::AnswerPolicy;
use falabichos_core::progress::{Progress, bicho_level};
use falabichos_core::save::{PlayerState, SaveGame};
use falabichos_core::srs::{DAY, SrsStore};
use falabichos_core::{Content, Rng};
use falabichos_media::Canvas;
use std::collections::{BinaryHeap, HashMap};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

/// Storage shared between two `Game` instances (to test save -> quit -> resume).
#[derive(Clone, Default)]
pub struct SharedStorage(pub Arc<Mutex<HashMap<String, String>>>);

impl Storage for SharedStorage {
    fn read(&self, name: &str) -> Option<String> {
        self.0.lock().ok()?.get(name).cloned()
    }
    fn write(&mut self, name: &str, data: &str) -> Result<(), String> {
        self.0
            .lock()
            .map_err(|e| e.to_string())?
            .insert(name.to_string(), data.to_string());
        Ok(())
    }
    fn remove(&mut self, name: &str) {
        if let Ok(mut m) = self.0.lock() {
            m.remove(name);
        }
    }
}

/// Produce the answer that is correct for an exercise (used by the sim and by tests).
pub fn solve(ex: &Exercise) -> Answer {
    match ex.kind {
        ExKind::Choice => Answer::Choice(ex.correct[0]),
        ExKind::Type => Answer::Text(ex.accepted[0].clone()),
        ExKind::Order => {
            let canon: Vec<&str> = ex.accepted[0].split_whitespace().collect();
            let mut used = vec![false; ex.options.len()];
            let mut idx = vec![];
            for w in canon {
                if let Some(i) = (0..ex.options.len()).find(|&i| !used[i] && ex.options[i] == w) {
                    used[i] = true;
                    idx.push(i);
                }
            }
            Answer::Order(idx)
        }
    }
}

// ------------------------------------------------------------------------------------ core sim

/// A scripted playthrough through the core library only.
pub fn core_sim(content: &Content) -> Result<Vec<String>, String> {
    let mut log = vec![];
    let mut rng = Rng::new(2024);
    let mut progress = Progress::default();
    let mut srs = SrsStore::default();
    let mut now: i64 = 1_700_000_000;
    let policy = AnswerPolicy::default();

    // 1. wild encounter with the Galo: one wrong answer, then all correct -> caught
    let mut b = Battle::wild(content, "galo", &mut progress, &srs, policy, None, &mut rng, now)
        .map_err(|e| e.to_string())?;
    let mut first = true;
    while !b.is_over() {
        let ex = b.current.clone().ok_or("no exercise")?;
        let ans = if first {
            first = false;
            match ex.kind {
                ExKind::Choice => Answer::Choice(
                    (0..ex.options.len())
                        .find(|i| !ex.correct.contains(i))
                        .unwrap_or(0),
                ),
                _ => Answer::Text("zzz".into()),
            }
        } else {
            solve(&ex)
        };
        b.answer(&ans, &mut srs, now).ok_or("answer rejected")?;
        b.advance(content, &srs, &mut rng, now);
        now += 20;
    }
    if b.outcome != Some(Outcome::Caught) {
        return Err(format!("expected Caught, got {:?}", b.outcome));
    }
    settle(&b, content, &mut progress, now);
    if !progress.has_bicho("galo") {
        return Err("Galo not in party".into());
    }
    log.push(format!(
        "caught the Galo after {} questions ({} right, {} wrong)",
        b.asked, b.correct, b.wrong
    ));

    // 2. the wrong answer comes back soon (learning step), the right ones are scheduled later
    let missed = b.missed.first().cloned().ok_or("expected one missed skill")?;
    let card = srs.card(&missed).ok_or("missed card exists")?;
    if card.due - now > 700 {
        return Err("a failed item should return within minutes".into());
    }
    log.push(format!(
        "failed item `{missed}` is due again in {} s",
        (card.due - now).max(0)
    ));

    // 3. spaced repetition levels the Bicho up over a simulated fortnight
    let lv0 = bicho_level(content, &srs, "galo");
    for day in 0..14 {
        now += DAY;
        let mut guard = 0;
        while srs.due_count(now) > 0 && guard < 80 {
            let mut p = Battle::practice(content, &progress, &srs, policy, 10, &mut rng, now)
                .map_err(|e| e.to_string())?;
            while !p.is_over() {
                let ex = p.current.clone().ok_or("no exercise")?;
                p.answer(&solve(&ex), &mut srs, now).ok_or("rejected")?;
                p.advance(content, &srs, &mut rng, now);
                now += 5;
            }
            guard += 1;
        }
        let _ = day;
    }
    let lv1 = bicho_level(content, &srs, "galo");
    if lv1 <= lv0 {
        return Err(format!("SRS practice should raise the level ({lv0} -> {lv1})"));
    }
    log.push(format!(
        "Galo level {lv0} -> {lv1} after two weeks of on-time reviews"
    ));

    // 4. Mestre test
    let mut m = Battle::challenge(content, "mestre_joaquim", &srs, policy, &mut rng, now)
        .map_err(|e| e.to_string())?;
    while !m.is_over() {
        let ex = m.current.clone().ok_or("no exercise")?;
        m.answer(&solve(&ex), &mut srs, now).ok_or("rejected")?;
        m.advance(content, &srs, &mut rng, now);
    }
    settle(&m, content, &mut progress, now);
    if !progress.seals.contains("selo_polvo") || !progress.has_flag("beat_joaquim") {
        return Err("Mestre Joaquim should award the seal".into());
    }
    log.push("beat Mestre Joaquim: Selo do Polvo awarded".into());

    // 5. save, load, migrate
    let mut save = SaveGame::new(
        PlayerState {
            map: "vila_azul".into(),
            x: 6,
            y: 6,
            facing: Dir::Down,
        },
        now,
    );
    save.progress = progress.clone();
    save.srs = srs.clone();
    let text = save.to_ron().map_err(|e| e.to_string())?;
    let back = falabichos_core::save::load(&text).map_err(|e| e.to_string())?;
    if back != save {
        return Err("save round trip changed the data".into());
    }
    let old = falabichos_core::save::load(
        r#"(map: "vila_azul", x: 1, y: 2, caught: ["galo"], flags: ["met_avo"])"#,
    )
    .map_err(|e| e.to_string())?;
    if !old.progress.has_bicho("galo") || old.version != falabichos_core::save::SAVE_VERSION {
        return Err("v0 migration failed".into());
    }
    log.push(format!(
        "save round-trip OK ({} bytes), v0 migration OK",
        text.len()
    ));
    Ok(log)
}

// ------------------------------------------------------------------------------------ UI driver

/// Drives a [`Game`] with synthetic input.
pub struct Driver {
    pub game: Game,
    clock: Arc<AtomicI64>,
    pub log: Vec<String>,
    shots: Option<PathBuf>,
    shot_count: usize,
    pub ticks: u64,
    wrong_next: bool,
    large_shot: bool,
    seen_kinds: Vec<(ExKind, bool)>,
}

type R<T> = Result<T, String>;

impl Driver {
    pub fn new(assets: Arc<Assets>, storage: SharedStorage, seed: u64) -> R<Driver> {
        let mut game = Game::new(
            assets,
            Box::new(storage),
            GameConfig {
                can_quit: true,
                touch: false,
                seed: Some(seed),
            },
        )?;
        let clock = Arc::new(AtomicI64::new(1_700_000_000));
        let c2 = clock.clone();
        game.set_clock(move || c2.load(Ordering::Relaxed));
        Ok(Driver {
            game,
            clock,
            log: vec![],
            shots: None,
            shot_count: 0,
            ticks: 0,
            wrong_next: false,
            large_shot: false,
            seen_kinds: vec![],
        })
    }

    pub fn with_screenshots(mut self, dir: PathBuf) -> Driver {
        let _ = std::fs::create_dir_all(&dir);
        self.shots = Some(dir);
        self
    }

    fn tick(&mut self, i: &Input) {
        self.game.tick(i);
        self.ticks += 1;
        // one simulated second per 4 ticks: learning steps pass during a session
        if self.ticks.is_multiple_of(4) {
            self.clock.fetch_add(1, Ordering::Relaxed);
        }
        self.game.take_events();
    }

    fn idle(&mut self, n: usize) {
        for _ in 0..n {
            self.tick(&Input::default());
        }
    }

    fn tap(&mut self, f: impl Fn(&mut Input)) {
        let mut i = Input::default();
        f(&mut i);
        self.tick(&i);
        self.idle(2);
    }

    fn tap_a(&mut self) {
        self.tap(|i| i.a = true);
    }

    fn capture(&mut self, name: &str) {
        let Some(dir) = self.shots.clone() else { return };
        let mut c = Canvas::new(crate::LOGICAL_W, crate::LOGICAL_H);
        self.game.draw(&mut c);
        self.shot_count += 1;
        let path = dir.join(format!("{:02}-{name}.png", self.shot_count));
        let _ = std::fs::write(path, c.upscaled(3).to_png());
    }

    fn pos(&self) -> (String, i32, i32) {
        (
            self.game.world.map_id.clone(),
            self.game.world.x,
            self.game.world.y,
        )
    }

    fn busy(&self) -> bool {
        let w = &self.game.world;
        !matches!(w.fade, crate::world::Fade::None)
            || w.flash.is_some()
            || w.trainer.is_some()
            || w.walk.is_some()
    }

    /// Resolve whatever is on screen (dialogue, battle, fade) until we are back in the free world.
    fn settle(&mut self) -> R<()> {
        for _ in 0..4000 {
            match self.game.scene_name() {
                "world" if !self.busy() => return Ok(()),
                "world" => self.idle(1),
                "dialogue" => {
                    self.tap_a();
                }
                "battle" => self.battle_step()?,
                _ => self.tap(|i| i.b = true),
            }
        }
        Err(format!(
            "settle() did not finish; scene={}",
            self.game.scene_name()
        ))
    }

    fn battle_step(&mut self) -> R<()> {
        let phase = match &self.game.scene {
            Scene::Battle(b) => b.phase.clone(),
            _ => return Ok(()),
        };
        match phase {
            BPhase::Ask => {
                let ex = self
                    .game
                    .battle_exercise()
                    .cloned()
                    .ok_or("battle without exercise")?;
                // Render the question screen (normal and large text): a layout panic would fail the sim.
                let mut scratch = Canvas::new(crate::LOGICAL_W, crate::LOGICAL_H);
                self.game.draw(&mut scratch);
                if !self.large_shot && self.game.settings.text_size == falabichos_core::save::TextSize::Large
                {
                    self.large_shot = true;
                    self.capture("battle-large-text");
                }
                let wrong = std::mem::take(&mut self.wrong_next);
                if !self.seen_kinds.contains(&(ex.kind, false)) {
                    self.seen_kinds.push((ex.kind, false));
                    let name = match ex.kind {
                        ExKind::Choice => "battle-choice",
                        ExKind::Type => "battle-type",
                        ExKind::Order => "battle-order",
                    };
                    self.capture(name);
                }
                self.answer_exercise(&ex, wrong);
            }
            BPhase::Feedback { t: 12 } => {
                // capture the first feedback screen once
                if !self.seen_kinds.contains(&(ExKind::Choice, true)) {
                    self.seen_kinds.push((ExKind::Choice, true));
                    self.capture("battle-feedback");
                }
                self.tap_a();
            }
            _ => self.tap_a(),
        }
        Ok(())
    }

    fn answer_exercise(&mut self, ex: &Exercise, wrong: bool) {
        match ex.kind {
            ExKind::Choice => {
                let target = if wrong {
                    (0..ex.options.len())
                        .find(|i| !ex.correct.contains(i))
                        .unwrap_or(0)
                } else {
                    ex.correct[0]
                };
                for _ in 0..target {
                    self.tap(|i| i.down = true);
                }
                self.tap_a();
            }
            ExKind::Type => {
                let text = if wrong {
                    "zzz".to_string()
                } else {
                    ex.accepted[0].clone()
                };
                let mut i = Input {
                    typed: text,
                    ..Input::default()
                };
                self.tick(&i);
                i = Input {
                    enter: true,
                    ..Input::default()
                };
                self.tick(&i);
                self.idle(2);
            }
            ExKind::Order => {
                let Answer::Order(idx) = solve(ex) else { return };
                let n = ex.options.len();
                let mut cursor = 0usize;
                for target in idx {
                    let steps = (target + n + 1 - cursor) % (n + 1);
                    for _ in 0..steps {
                        self.tap(|i| i.right = true);
                    }
                    self.tap_a();
                    cursor = target;
                    if cursor >= n {
                        cursor = n;
                    }
                }
                // all tiles chosen: the cursor jumped to "Check"
                self.tap_a();
            }
        }
    }

    /// Hold a direction until the player moves one tile (or something interrupts).
    fn step(&mut self, d: Dir) -> bool {
        let before = self.pos();
        let mut input = Input::default();
        match d {
            Dir::Up => input.up = true,
            Dir::Down => input.down = true,
            Dir::Left => input.left = true,
            Dir::Right => input.right = true,
        }
        for _ in 0..60 {
            self.tick(&input);
            if self.pos() != before || self.game.scene_name() != "world" {
                self.tick(&Input::default());
                return true;
            }
        }
        false
    }

    fn walkable(&self, x: i32, y: i32) -> Option<u32> {
        let m = self.game.content.maps.get(&self.game.world.map_id)?;
        let t = self.game.content.tile(m.tile_char(x, y)?)?;
        match t.kind {
            TileKind::Floor => Some(1),
            TileKind::TallGrass => Some(6),
            _ => None,
        }
    }

    fn npc_tiles(&self) -> Vec<(i32, i32)> {
        self.game
            .world
            .npcs
            .iter()
            .filter(|n| self.game.save.progress.eval(&n.def.show_when))
            .map(|n| (n.x, n.y))
            .collect()
    }

    /// Dijkstra over the current map (tall grass costs more so the path avoids random encounters).
    fn path_to(&self, goal: (i32, i32)) -> Option<Vec<Dir>> {
        let start = (self.game.world.x, self.game.world.y);
        let blocked = self.npc_tiles();
        let mut dist: HashMap<(i32, i32), u32> = HashMap::new();
        let mut prev: HashMap<(i32, i32), ((i32, i32), Dir)> = HashMap::new();
        let mut heap = BinaryHeap::new();
        dist.insert(start, 0);
        heap.push(std::cmp::Reverse((0u32, start)));
        while let Some(std::cmp::Reverse((d, p))) = heap.pop() {
            if p == goal {
                break;
            }
            if d > *dist.get(&p).unwrap_or(&u32::MAX) {
                continue;
            }
            for dir in [Dir::Up, Dir::Down, Dir::Left, Dir::Right] {
                let (dx, dy) = dir.delta();
                let q = (p.0 + dx, p.1 + dy);
                let Some(cost) = self.walkable(q.0, q.1) else {
                    continue;
                };
                if blocked.contains(&q) {
                    continue;
                }
                // never route *through* a warp tile unless it is the goal
                let is_warp = self.game.map().warps.iter().any(|w| (w.x, w.y) == q);
                if is_warp && q != goal {
                    continue;
                }
                let nd = d + cost;
                if nd < *dist.get(&q).unwrap_or(&u32::MAX) {
                    dist.insert(q, nd);
                    prev.insert(q, (p, dir));
                    heap.push(std::cmp::Reverse((nd, q)));
                }
            }
        }
        if !dist.contains_key(&goal) {
            return None;
        }
        let mut path = vec![];
        let mut cur = goal;
        while cur != start {
            let (p, d) = prev[&cur];
            path.push(d);
            cur = p;
        }
        path.reverse();
        Some(path)
    }

    fn goto(&mut self, goal: (i32, i32)) -> R<()> {
        let map = self.game.world.map_id.clone();
        for _ in 0..3000 {
            self.settle()?;
            if self.game.world.map_id != map {
                return Ok(());
            }
            if (self.game.world.x, self.game.world.y) == goal {
                return Ok(());
            }
            let path = self
                .path_to(goal)
                .ok_or_else(|| format!("no path to {goal:?} on {map} from {:?}", self.pos()))?;
            let d = path[0];
            if !self.step(d) {
                // blocked by a wandering NPC: wait a bit
                self.idle(20);
            }
        }
        Err(format!("goto {goal:?} did not converge"))
    }

    /// Walk through a warp of the current map that leads to `target`.
    fn enter(&mut self, target: &str) -> R<()> {
        let warp = self
            .game
            .map()
            .warps
            .iter()
            .find(|w| w.map == target)
            .cloned()
            .ok_or_else(|| format!("no warp to {target} on {}", self.game.world.map_id))?;
        self.goto((warp.x, warp.y))?;
        self.settle()?;
        if self.game.world.map_id != target {
            return Err(format!(
                "expected to arrive in {target}, am in {}",
                self.game.world.map_id
            ));
        }
        Ok(())
    }

    fn talk_to(&mut self, npc_id: &str) -> R<()> {
        let n = self
            .game
            .world
            .npcs
            .iter()
            .find(|n| n.def.id == npc_id)
            .map(|n| (n.x, n.y))
            .ok_or_else(|| format!("npc {npc_id} not on {}", self.game.world.map_id))?;
        let content = self.game.content.clone();
        let map = self.game.map().clone();
        let kind = |x: i32, y: i32| map.tile_char(x, y).and_then(|c| content.tile(c)).map(|t| t.kind);
        let cands = [
            (0, 1, Dir::Up),
            (-1, 0, Dir::Right),
            (1, 0, Dir::Left),
            (0, -1, Dir::Down),
        ];
        let mut spot = None;
        for (dx, dy, face) in cands {
            let (x, y) = (n.0 + dx, n.1 + dy);
            if self.walkable(x, y).is_some() && self.path_to((x, y)).is_some() {
                spot = Some(((x, y), face));
                break;
            }
            // across a counter
            if kind(x, y) == Some(TileKind::Counter) {
                let (x2, y2) = (n.0 + dx * 2, n.1 + dy * 2);
                if self.walkable(x2, y2).is_some() && self.path_to((x2, y2)).is_some() {
                    spot = Some(((x2, y2), face));
                    break;
                }
            }
        }
        let ((x, y), face) = spot.ok_or_else(|| format!("cannot reach {npc_id}"))?;
        self.goto((x, y))?;
        self.settle()?;
        // turn to face the NPC
        self.step(face);
        self.idle(4);
        self.tap_a();
        if self.game.scene_name() != "dialogue" {
            return Err(format!(
                "talking to {npc_id} did not open a dialogue (scene {})",
                self.game.scene_name()
            ));
        }
        Ok(())
    }

    fn talk_and_finish(&mut self, npc_id: &str) -> R<()> {
        self.talk_to(npc_id)?;
        self.settle()
    }

    fn hunt_wild(&mut self, near: (i32, i32)) -> R<()> {
        self.goto(near)?;
        for i in 0..600 {
            if self.game.in_battle() || self.busy() {
                return self.settle();
            }
            let d = if i % 2 == 0 { Dir::Right } else { Dir::Left };
            self.step(d);
        }
        Err("no wild encounter in 600 steps".into())
    }

    fn open_menu_item(&mut self, index: usize) {
        self.tap(|i| i.start = true);
        for _ in 0..index {
            self.tap(|i| i.down = true);
        }
        self.tap_a();
    }
}

fn check(cond: bool, msg: &str) -> R<()> {
    if cond { Ok(()) } else { Err(msg.to_string()) }
}

/// Play the whole vertical slice: title -> new game -> first Bicho -> save/resume -> Mestre 1 ->
/// road -> Mestra 2 -> finale. Returns a log of milestones.
pub fn playthrough(assets: Arc<Assets>, shots: Option<PathBuf>) -> R<Vec<String>> {
    let storage = SharedStorage::default();
    let mut d = Driver::new(assets.clone(), storage.clone(), 7)?;
    if let Some(dir) = shots {
        d = d.with_screenshots(dir);
    }
    let mut log: Vec<String> = vec![];

    // ---- title and new game
    check(
        d.game.scene_name() == "title",
        "game should start on the title screen",
    )?;
    d.idle(30);
    d.capture("title");
    d.tap_a(); // New game
    check(
        d.game.scene_name() == "dialogue",
        "new game opens the intro dialogue",
    )?;
    // exercise the gloss: toggle it and look up a word
    d.idle(120);
    d.tap(|i| i.select = true);
    d.tap(|i| i.right = true);
    d.capture("dialogue-gloss");
    d.settle()?;
    log.push("title -> new game -> intro dialogue OK".into());

    // ---- grandma, library, first Bicho
    d.talk_to("avo")?;
    d.idle(100);
    d.capture("dialogue-avo");
    d.settle()?;
    check(
        d.game.progress().has_flag("met_avo"),
        "talking to Grandma sets met_avo",
    )?;
    d.enter("vila_azul")?;
    d.idle(40);
    d.capture("town");
    // the east gate is closed until the first Seal
    let gate = d
        .game
        .map()
        .warps
        .iter()
        .find(|w| w.map == "rota_colinas")
        .map(|w| (w.x, w.y))
        .ok_or("east warp")?;
    check(
        d.path_to(gate).is_none(),
        "the old man blocks the road east before the first Seal",
    )?;
    d.enter("biblioteca")?;
    d.talk_and_finish("amelia")?;
    check(
        d.game.progress().has_flag("got_caderno"),
        "Professor Amelia hands over the Notebook",
    )?;
    d.enter("vila_azul")?;
    log.push("walked the town, talked to Grandma and the Professor, gate blocked OK".into());

    d.wrong_next = true;
    d.hunt_wild((3, 15))?;
    check(
        d.game.scene_name() == "world",
        "back in the world after the battle",
    )?;
    check(
        !d.game.progress().party.is_empty() || d.game.save.confidence.is_some(),
        "the first battle changed the game state",
    )?;
    // keep hunting until one Bicho has been caught (the first battle had a deliberate mistake)
    let mut tries = 0;
    while d.game.progress().party.is_empty() && tries < 12 {
        d.hunt_wild((3, 15))?;
        tries += 1;
        if d.game.save.confidence.is_some() {
            // restore confidence at Grandma's if we are low
            d.talk_here_heal()?;
        }
    }
    check(!d.game.progress().party.is_empty(), "caught a first Bicho")?;
    log.push(format!(
        "wild encounter -> exercise battle (first answer deliberately wrong) -> caught {} (party size {})",
        d.game.progress().party[0].species,
        d.game.progress().party.len()
    ));

    // ---- every exercise flavour through the real UI (typing, ordering, articles, IPA)
    let kinds = d.exercise_tour()?;
    log.push(format!("exercise tour covered {kinds}"));

    // ---- Bichodex, notebook, seals, settings screens
    d.open_menu_item(0);
    d.capture("menu-dex");
    d.tap_a();
    d.capture("dex-detail");
    d.tap(|i| i.right = true);
    d.capture("dex-words");
    d.tap(|i| i.b = true);
    d.tap(|i| i.b = true);
    d.tap(|i| i.b = true);
    check(d.game.scene_name() == "world", "menus close back to the world")?;
    d.open_menu_item(3);
    d.tap_a();
    d.capture("notebook");
    d.settle()?;
    d.open_menu_item(4);
    d.capture("seals");
    d.settle()?;
    d.open_menu_item(5);
    check(d.game.scene_name() == "calendar", "the review calendar opens")?;
    d.capture("calendar");
    d.settle()?;
    d.open_menu_item(7);
    d.capture("settings");
    d.settle()?;
    log.push("Bichodex / Notebook / Seals / Review calendar / Settings screens open and close OK".into());

    // ---- practice session
    d.open_menu_item(2);
    check(
        d.game.scene_name() == "battle",
        "Practice starts a review session",
    )?;
    d.settle()?;
    log.push("practice session (SRS review) completed".into());

    // ---- save, quit, resume
    d.open_menu_item(6);
    d.idle(5);
    check(storage.read("save.ron").is_some(), "Save writes save.ron")?;
    let (map, x, y) = d.pos();
    let party: Vec<String> = d
        .game
        .progress()
        .party
        .iter()
        .map(|b| b.species.clone())
        .collect();
    let srs_cards = d.game.save.srs.cards.len();
    d.settle()?;
    let mut d2 = Driver::new(assets.clone(), storage.clone(), 99)?;
    check(d2.game.has_save(), "a new session sees the save")?;
    d2.tap_a(); // first entry is "Continue"
    check(d2.game.scene_name() == "world", "Continue resumes the game")?;
    check(d2.pos() == (map.clone(), x, y), "position restored")?;
    let party2: Vec<String> = d2
        .game
        .progress()
        .party
        .iter()
        .map(|b| b.species.clone())
        .collect();
    check(party2 == party, "party restored")?;
    check(d2.game.save.srs.cards.len() == srs_cards, "SRS cards restored")?;
    log.push(format!(
        "save -> quit -> resume OK (party {party:?}, {srs_cards} SRS cards, at {map}({x},{y}))"
    ));
    let mut d = d2;
    d.shots = None;

    // ---- Mestre Joaquim
    d.enter_town_from_here()?;
    d.enter("restaurante_polvo")?;
    d.talk_to("joaquim")?;
    d.settle()?;
    check(
        d.game.progress().has_flag("beat_joaquim"),
        "Mestre Joaquim defeated",
    )?;
    check(
        d.game.progress().seals.contains("selo_polvo"),
        "Selo do Polvo awarded",
    )?;
    log.push("Mestre Joaquim beaten: Selo do Polvo".into());
    d.enter("vila_azul")?;

    // ---- the road east (trainers may challenge on the way)
    d.enter("rota_colinas")?;
    d.enter("setecolinas")?;
    d.talk_and_finish("lisboeta").ok();
    check(
        d.path_to((31, 8)).is_none(),
        "the guard blocks the road to Ribeira Dourada until the second Selo",
    )?;
    d.enter("estacao_eletrico")?;
    d.talk_to("beatriz")?;
    d.settle()?;
    check(
        d.game.progress().has_flag("beat_beatriz"),
        "Mestra Beatriz defeated",
    )?;
    check(d.game.progress().seals.len() == 2, "two Seals")?;
    d.settle()?;
    check(d.game.progress().has_flag("finale_seen"), "the finale played")?;
    log.push("road -> Setecolinas -> Mestra Beatriz beaten: Selo do Eletrico, finale reached".into());

    // ---- Part 2, town 3: Ribeira Dourada
    d.enter("setecolinas")?;
    check(
        d.path_to((31, 8)).is_some(),
        "the guard is gone after the second Selo",
    )?;
    d.enter("rota_ribeira")?;
    d.enter("ribeira_dourada")?;
    d.talk_and_finish("portuense").ok();
    // a wild encounter in the park of the new town
    d.hunt_wild((4, 10))?;
    d.settle()?;
    check(
        d.game.scene_name() == "world",
        "back in the world after the wild encounter in Ribeira Dourada",
    )?;
    d.enter("posto_turismo")?;
    d.talk_to("duarte")?;
    d.settle()?;
    check(
        d.game.progress().has_flag("beat_duarte"),
        "Mestre Duarte defeated",
    )?;
    check(
        d.game.progress().seals.contains("selo_ponte"),
        "Selo da Ponte awarded",
    )?;
    check(d.game.progress().seals.len() == 3, "three Seals")?;
    log.push("Part 2: road -> Ribeira Dourada -> Mestre Duarte beaten: Selo da Ponte".into());
    let (seen, caught) = d.game.progress().dex_counts();
    log.push(format!(
        "final state: {caught} Bichos caught, {seen} seen, {} SRS cards, {} steps",
        d.game.save.srs.cards.len(),
        d.game.save.steps
    ));
    Ok(log)
}

impl Driver {
    /// Seed well-known SRS cards so the difficulty ladder yields typed / ordered answers, then battle
    /// grammar and vocab Bichos until every answer style has been used through the UI.
    fn exercise_tour(&mut self) -> R<String> {
        use falabichos_core::srs::{Card, Phase};
        let now = self.game.now();
        for species in ["camaleao", "sardinha", "gaivota"] {
            for sk in self.game.content.species_skills(species) {
                let card = Card {
                    phase: Phase::Review,
                    seen: 10,
                    reps: 4,
                    interval_days: 5.0,
                    due: now + 30 * DAY,
                    ..Card::default()
                };
                self.game.save.srs.cards.insert(sk, card);
            }
        }
        let mut styles: Vec<String> = vec![];
        for round in 0..30 {
            let species = ["camaleao", "sardinha", "gaivota"][round % 3];
            self.game.save.confidence = None;
            // alternate text sizes: every question screen is drawn below, so a layout panic fails the sim
            self.game.settings.text_size = if round % 2 == 0 {
                falabichos_core::save::TextSize::Large
            } else {
                falabichos_core::save::TextSize::Normal
            };
            self.game
                .start_battle(crate::world::PendingBattle::Wild(species.to_string()));
            // answer everything through the UI; remember which kinds we met
            for _ in 0..4000 {
                if let Some(ex) = self.game.battle_exercise() {
                    let tag = format!("{:?}/{:?}", ex.kind, ex.flavor);
                    if !styles.contains(&tag) {
                        styles.push(tag);
                    }
                }
                if self.game.scene_name() != "battle" {
                    break;
                }
                self.battle_step()?;
            }
            self.settle()?;
            let have = |k: &str| styles.iter().any(|t| t.starts_with(k));
            if have("Type") && have("Order") && have("Choice") && styles.iter().any(|t| t.contains("Article"))
            {
                break;
            }
        }
        self.game.settings.text_size = falabichos_core::save::TextSize::Normal;
        let need = ["Type", "Order", "Choice"];
        for n in need {
            if !styles.iter().any(|t| t.starts_with(n)) {
                return Err(format!("never met a {n} exercise (saw {styles:?})"));
            }
        }
        styles.sort();
        Ok(styles.join(", "))
    }

    /// Walk back to the starting town from wherever we resumed (interior or exterior).
    fn enter_town_from_here(&mut self) -> R<()> {
        for _ in 0..4 {
            self.settle()?;
            if self.game.world.map_id == "vila_azul" {
                return Ok(());
            }
            let back = self
                .game
                .map()
                .warps
                .iter()
                .find(|w| w.map == "vila_azul")
                .cloned();
            match back {
                Some(w) => {
                    self.goto((w.x, w.y))?;
                    self.settle()?;
                }
                None => return Err(format!("cannot get back to town from {}", self.game.world.map_id)),
            }
        }
        Err("could not return to town".into())
    }

    /// Visit Grandma to restore confidence (used when the hunt left us low).
    fn talk_here_heal(&mut self) -> R<()> {
        self.enter_town_from_here()?;
        self.enter("casa_avo")?;
        self.talk_and_finish("avo")?;
        self.enter("vila_azul")
    }
}

/// Run every headless check and return the report lines.
pub fn run_all(assets: Arc<Assets>) -> R<Vec<String>> {
    let mut out = vec![];
    let content = falabichos_content::load_strict(&assets)
        .map_err(|e| e.iter().map(|d| d.to_string()).collect::<Vec<_>>().join("\n"))?;
    out.push("== core simulation".to_string());
    out.extend(core_sim(&content)?.into_iter().map(|l| format!("  {l}")));
    out.push("== full playthrough through the real game loop (synthetic input, no display)".to_string());
    out.extend(playthrough(assets, None)?.into_iter().map(|l| format!("  {l}")));
    out.push("ALL HEADLESS CHECKS PASSED".to_string());
    Ok(out)
}

/// Replay the playthrough and save PNG screenshots of key screens into `dir`.
pub fn screenshots(assets: Arc<Assets>, dir: PathBuf) -> R<usize> {
    let n_before = std::fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0);
    playthrough(assets, Some(dir.clone()))?;
    Ok(std::fs::read_dir(&dir)
        .map(|d| d.count())
        .unwrap_or(0)
        .saturating_sub(n_before))
}
