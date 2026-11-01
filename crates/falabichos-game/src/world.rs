// SPDX-License-Identifier: Apache-2.0
//! The overworld: grid-locked walking, NPCs, signs, warps, tall-grass encounters, trainer sight lines.

use crate::dialogue::DialogueRun;
use crate::input::Pad;
use crate::menus::MenuState;
use crate::ui::Ui;
use crate::{Game, Scene};
use falabichos_core::model::{Dir, MapDef, Npc, TileKind};
use falabichos_core::progress::Effect;
use falabichos_media::Canvas;

pub(crate) const TILE: i32 = 16;

#[derive(Debug, Clone)]
pub(crate) enum PendingBattle {
    Wild(String),
    Challenge(String),
}

#[derive(Debug, Clone)]
pub(crate) enum After {
    Warp {
        map: String,
        x: i32,
        y: i32,
        facing: Dir,
    },
}

#[derive(Debug, Clone)]
pub(crate) enum Fade {
    None,
    Out { t: i32, then: After },
    In { t: i32 },
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Walk {
    pub dir: Dir,
    pub prog: i32,
}

#[derive(Debug, Clone)]
pub(crate) struct NpcState {
    pub def: Npc,
    pub x: i32,
    pub y: i32,
    pub facing: Dir,
    pub walk: Option<Walk>,
    pub timer: i32,
    pub home: (i32, i32),
}

#[derive(Debug, Clone)]
pub(crate) enum TrainerSeq {
    Alert { npc: usize, t: i32 },
    Approach { npc: usize },
}

pub(crate) struct World {
    pub map_id: String,
    pub x: i32,
    pub y: i32,
    pub facing: Dir,
    pub walk: Option<Walk>,
    pub parity: bool,
    pub turn: i32,
    pub npcs: Vec<NpcState>,
    pub dialogue: Option<DialogueRun>,
    pub fade: Fade,
    pub banner: Option<(String, i32)>,
    pub trainer: Option<TrainerSeq>,
    pub flash: Option<(PendingBattle, i32)>,
    pub bump_cd: i32,
    pub forced_walk: Option<Dir>,
    pub last_town: Option<String>,
}

impl World {
    pub fn new(content: &falabichos_core::Content, map: &str, x: i32, y: i32, facing: Dir) -> World {
        let mut w = World {
            map_id: map.to_string(),
            x,
            y,
            facing,
            walk: None,
            parity: false,
            turn: 0,
            npcs: vec![],
            dialogue: None,
            fade: Fade::None,
            banner: None,
            trainer: None,
            flash: None,
            bump_cd: 0,
            forced_walk: None,
            last_town: None,
        };
        w.reset_npcs(content);
        w
    }

    pub fn reset_npcs(&mut self, content: &falabichos_core::Content) {
        self.npcs = content
            .maps
            .get(&self.map_id)
            .map(|m| {
                m.npcs
                    .iter()
                    .map(|n| NpcState {
                        def: n.clone(),
                        x: n.x,
                        y: n.y,
                        facing: n.facing,
                        walk: None,
                        timer: 40 + (n.x * 7 + n.y * 13) % 60,
                        home: (n.x, n.y),
                    })
                    .collect()
            })
            .unwrap_or_default();
    }
}

fn dir_from_pad(pad: &Pad) -> Option<Dir> {
    let h = &pad.held;
    if h.up {
        Some(Dir::Up)
    } else if h.down {
        Some(Dir::Down)
    } else if h.left {
        Some(Dir::Left)
    } else if h.right {
        Some(Dir::Right)
    } else {
        None
    }
}

impl Game {
    pub(crate) fn map(&self) -> &MapDef {
        self.content
            .maps
            .get(&self.world.map_id)
            .expect("current map exists (validated)")
    }

    fn tile_kind(&self, x: i32, y: i32) -> Option<TileKind> {
        let m = self.map();
        m.tile_char(x, y)
            .and_then(|c| self.content.tile(c))
            .map(|t| t.kind)
    }

    fn npc_visible(&self, n: &NpcState) -> bool {
        self.save.progress.eval(&n.def.show_when)
    }

    fn npc_at(&self, x: i32, y: i32) -> Option<usize> {
        self.world
            .npcs
            .iter()
            .position(|n| n.x == x && n.y == y && self.npc_visible(n))
    }

    fn player_target(&self) -> (i32, i32) {
        match self.world.walk {
            Some(w) => (self.world.x + w.dir.delta().0, self.world.y + w.dir.delta().1),
            None => (self.world.x, self.world.y),
        }
    }

    fn player_blocked(&self, x: i32, y: i32) -> bool {
        !matches!(self.tile_kind(x, y), Some(TileKind::Floor | TileKind::TallGrass))
            || self.npc_at(x, y).is_some()
    }

    /// Set up the current map after a change (music, NPCs, banner).
    pub(crate) fn enter_current_map(&mut self, announce: bool) {
        let content = self.content.clone();
        self.world.reset_npcs(&content);
        let m = self.map().clone();
        self.set_music(m.music.as_deref());
        let town = m.town.clone();
        let show = announce || m.indoor || town != self.world.last_town;
        if show {
            let text = if m.indoor {
                m.name.clone()
            } else {
                town.as_ref()
                    .and_then(|t| content.towns.get(t))
                    .map(|t| t.name.clone())
                    .unwrap_or(m.name.clone())
            };
            self.world.banner = Some((text, 120));
        }
        self.world.last_town = town;
        self.world.trainer = None;
        self.world.walk = None;
    }

    pub(crate) fn begin_warp(&mut self, map: String, x: i32, y: i32, facing: Dir) {
        if matches!(self.world.fade, Fade::Out { .. }) {
            return;
        }
        let indoor_change = self.content.maps.get(&map).map(|m| m.indoor) != Some(self.map().indoor);
        if indoor_change {
            self.sfx("sfx_door");
        }
        self.world.fade = Fade::Out {
            t: 0,
            then: After::Warp { map, x, y, facing },
        };
    }

    pub(crate) fn begin_battle_transition(&mut self, pb: PendingBattle) {
        self.sfx("sfx_encounter");
        let t = if self.settings.reduce_motion { 12 } else { 40 };
        self.world.flash = Some((pb, t));
        self.set_music(None);
    }

    pub(crate) fn run_effects(&mut self, fx: Vec<Effect>) {
        for e in fx {
            match e {
                Effect::Heal => {
                    self.save.confidence = None;
                    self.sfx("sfx_levelup");
                    let msg = self.t("toast.healed");
                    self.toast(msg);
                }
                Effect::Challenge(id) => self.begin_battle_transition(PendingBattle::Challenge(id)),
                Effect::Warp { map, x, y } => self.begin_warp(map, x, y, Dir::Down),
                Effect::Save => {
                    let ok = self.save_game();
                    let msg = self.t(if ok { "toast.saved" } else { "toast.save_failed" });
                    self.sfx(if ok { "sfx_save" } else { "sfx_wrong" });
                    self.toast(msg);
                }
            }
        }
    }

    pub(crate) fn tick_world(&mut self, pad: &Pad) {
        // banner countdown
        if let Some((_, t)) = &mut self.world.banner {
            *t -= 1;
            if *t <= 0 {
                self.world.banner = None;
            }
        }
        // fades lock input
        match std::mem::replace(&mut self.world.fade, Fade::None) {
            Fade::Out { t, then } => {
                if t + 1 >= 12 {
                    match then {
                        After::Warp { map, x, y, facing } => {
                            self.world.map_id = map;
                            self.world.x = x;
                            self.world.y = y;
                            self.world.facing = facing;
                            self.enter_current_map(false);
                            self.world.fade = Fade::In { t: 0 };
                        }
                    }
                } else {
                    self.world.fade = Fade::Out { t: t + 1, then };
                }
                return;
            }
            Fade::In { t } => {
                if t + 1 < 12 {
                    self.world.fade = Fade::In { t: t + 1 };
                }
                return;
            }
            Fade::None => {}
        }
        // encounter flash
        if let Some((pb, t)) = self.world.flash.take() {
            if t > 1 {
                self.world.flash = Some((pb, t - 1));
            } else {
                self.start_battle(pb);
            }
            return;
        }
        if self.world.dialogue.is_some() {
            self.tick_dialogue(pad);
            return;
        }
        self.update_npcs();
        if self.world.trainer.is_some() {
            self.tick_trainer();
            return;
        }
        // player
        if let Some(mut w) = self.world.walk {
            w.prog += if pad.held.b { 2 } else { 1 };
            if w.prog >= TILE {
                let (dx, dy) = w.dir.delta();
                self.world.x += dx;
                self.world.y += dy;
                self.world.walk = None;
                self.world.parity = !self.world.parity;
                self.save.steps += 1;
                self.on_arrive();
            } else {
                self.world.walk = Some(w);
            }
            return;
        }
        if self.world.bump_cd > 0 {
            self.world.bump_cd -= 1;
        }
        if pad.start {
            self.sfx("sfx_confirm");
            let m = MenuState::new(&self.t("menu.title"));
            self.scene = Scene::Menu(m);
            return;
        }
        if pad.a {
            self.interact();
            return;
        }
        if let Some(d) = self.world.forced_walk.take() {
            self.start_walk(d);
            return;
        }
        match dir_from_pad(pad) {
            Some(d) => {
                if self.world.facing != d {
                    self.world.facing = d;
                    self.world.turn = 6;
                    return;
                }
                if self.world.turn > 0 {
                    self.world.turn -= 1;
                    return;
                }
                self.start_walk(d);
            }
            None => self.world.turn = 0,
        }
    }

    fn start_walk(&mut self, d: Dir) {
        let (dx, dy) = d.delta();
        let (tx, ty) = (self.world.x + dx, self.world.y + dy);
        self.world.facing = d;
        if self.player_blocked(tx, ty) {
            if self.world.bump_cd == 0 {
                self.sfx("sfx_bump");
                self.world.bump_cd = 24;
            }
            return;
        }
        self.world.walk = Some(Walk { dir: d, prog: 0 });
    }

    fn on_arrive(&mut self) {
        let (x, y) = (self.world.x, self.world.y);
        let map = self.map().clone();
        if let Some(w) = map.warps.iter().find(|w| w.x == x && w.y == y) {
            let facing = w.facing.unwrap_or(self.world.facing);
            self.begin_warp(w.map.clone(), w.to_x, w.to_y, facing);
            return;
        }
        if let Some(t) = map
            .triggers
            .iter()
            .find(|t| t.x == x && t.y == y && self.save.progress.eval(&t.when))
        {
            if t.push_back {
                self.world.forced_walk = Some(self.world.facing.opposite());
            }
            let id = t.dialogue.clone();
            self.start_dialogue(&id, None);
            return;
        }
        if self.check_trainers() {
            return;
        }
        if self.tile_kind(x, y) == Some(TileKind::TallGrass)
            && !map.encounters.is_empty()
            && self.rng.chance(map.encounter_rate)
        {
            let total: u32 = map.encounters.iter().map(|e| e.weight).sum();
            let mut roll = self.rng.below(total.max(1) as usize) as u32;
            for e in &map.encounters {
                if roll < e.weight {
                    self.begin_battle_transition(PendingBattle::Wild(e.species.clone()));
                    return;
                }
                roll -= e.weight;
            }
        }
    }

    // ------------------------------------------------------------------ NPCs and trainers

    fn update_npcs(&mut self) {
        let in_seq = self.world.trainer.is_some();
        for i in 0..self.world.npcs.len() {
            if let Some(mut w) = self.world.npcs[i].walk {
                w.prog += 1;
                self.world.npcs[i].walk = if w.prog >= TILE { None } else { Some(w) };
                continue;
            }
            let n = &self.world.npcs[i];
            if !n.def.wander || in_seq || !self.npc_visible(n) {
                continue;
            }
            let timer = n.timer - 1;
            self.world.npcs[i].timer = timer;
            if timer > 0 {
                continue;
            }
            self.world.npcs[i].timer = 50 + self.rng.below(90) as i32;
            let d = [Dir::Up, Dir::Down, Dir::Left, Dir::Right][self.rng.below(4)];
            let (dx, dy) = d.delta();
            let (tx, ty) = (self.world.npcs[i].x + dx, self.world.npcs[i].y + dy);
            let home = self.world.npcs[i].home;
            self.world.npcs[i].facing = d;
            if (tx - home.0).abs() > 2 || (ty - home.1).abs() > 2 {
                continue;
            }
            if self.npc_can_step(tx, ty) {
                let n = &mut self.world.npcs[i];
                n.x = tx;
                n.y = ty;
                n.walk = Some(Walk { dir: d, prog: 0 });
            }
        }
    }

    fn npc_can_step(&self, x: i32, y: i32) -> bool {
        if self.player_blocked(x, y) {
            return false;
        }
        let (px, py) = (self.world.x, self.world.y);
        let (tx, ty) = self.player_target();
        !((x == px && y == py) || (x == tx && y == ty))
    }

    fn trainer_defeated(&self, n: &NpcState) -> bool {
        n.def
            .trainer
            .as_ref()
            .and_then(|t| self.content.challenges.get(&t.challenge))
            .map(|c| self.save.progress.has_flag(&c.reward_flag))
            .unwrap_or(true)
    }

    /// Line-of-sight check after the player stepped. Returns true if a trainer spotted the player.
    fn check_trainers(&mut self) -> bool {
        let (px, py) = (self.world.x, self.world.y);
        for i in 0..self.world.npcs.len() {
            let n = &self.world.npcs[i];
            let Some(tr) = &n.def.trainer else { continue };
            if !self.npc_visible(n) || self.trainer_defeated(n) {
                continue;
            }
            let (dx, dy) = n.facing.delta();
            for step in 1..=tr.sight as i32 {
                let (tx, ty) = (n.x + dx * step, n.y + dy * step);
                if (tx, ty) == (px, py) {
                    self.world.trainer = Some(TrainerSeq::Alert { npc: i, t: 0 });
                    self.sfx("sfx_hint");
                    return true;
                }
                if !matches!(
                    self.tile_kind(tx, ty),
                    Some(TileKind::Floor | TileKind::TallGrass)
                ) {
                    break;
                }
            }
        }
        false
    }

    fn tick_trainer(&mut self) {
        let Some(seq) = self.world.trainer.clone() else {
            return;
        };
        match seq {
            TrainerSeq::Alert { npc, t } => {
                if t >= 30 {
                    self.world.trainer = Some(TrainerSeq::Approach { npc });
                } else {
                    self.world.trainer = Some(TrainerSeq::Alert { npc, t: t + 1 });
                }
            }
            TrainerSeq::Approach { npc } => {
                if self.world.npcs[npc].walk.is_some() {
                    return;
                }
                let (nx, ny) = (self.world.npcs[npc].x, self.world.npcs[npc].y);
                let (px, py) = (self.world.x, self.world.y);
                let dist = (nx - px).abs() + (ny - py).abs();
                if dist <= 1 {
                    let tr = self.world.npcs[npc].def.trainer.clone().expect("trainer");
                    // face each other
                    let toward = if px > nx {
                        Dir::Right
                    } else if px < nx {
                        Dir::Left
                    } else if py > ny {
                        Dir::Down
                    } else {
                        Dir::Up
                    };
                    self.world.npcs[npc].facing = toward;
                    self.world.facing = toward.opposite();
                    self.world.trainer = None;
                    self.start_dialogue(&tr.spot_dialogue, Some(tr.challenge));
                    return;
                }
                let d = self.world.npcs[npc].facing;
                let (dx, dy) = d.delta();
                let n = &mut self.world.npcs[npc];
                n.x += dx;
                n.y += dy;
                n.walk = Some(Walk { dir: d, prog: 0 });
            }
        }
    }

    fn interact(&mut self) {
        let (dx, dy) = self.world.facing.delta();
        let (tx, ty) = (self.world.x + dx, self.world.y + dy);
        let map = self.map().clone();
        // NPC in front, or across a counter
        let mut npc = self.npc_at(tx, ty);
        if npc.is_none() && self.tile_kind(tx, ty) == Some(TileKind::Counter) {
            npc = self.npc_at(tx + dx, ty + dy);
        }
        if let Some(i) = npc {
            let n = &self.world.npcs[i];
            let dlg = n.def.dialogue.clone();
            let facing_back = self.world.facing.opposite();
            if let Some(id) = dlg {
                self.world.npcs[i].facing = facing_back;
                self.start_dialogue(&id, None);
            }
            return;
        }
        if let Some(s) = map.signs.iter().find(|s| s.x == tx && s.y == ty) {
            let id = s.dialogue.clone();
            self.start_dialogue(&id, None);
        }
    }

    // ------------------------------------------------------------------ drawing

    pub(crate) fn draw_world(&self, c: &mut Canvas, ui: &Ui) {
        let w = &self.world;
        let Some(map) = self.content.maps.get(&w.map_id) else {
            return;
        };
        c.clear(self.bank.pal('K'));
        let (mw, mh) = (map.width() as i32 * TILE, map.height() as i32 * TILE);
        let (pox, poy) = match w.walk {
            Some(wk) => (wk.dir.delta().0 * wk.prog, wk.dir.delta().1 * wk.prog),
            None => (0, 0),
        };
        let focus_x = w.x * TILE + pox + TILE / 2;
        let focus_y = w.y * TILE + poy + TILE / 2;
        let cam = |focus: i32, size: i32, view: i32| -> i32 {
            if size <= view {
                -(view - size) / 2
            } else {
                (focus - view / 2).clamp(0, size - view)
            }
        };
        let (cx, cy) = (cam(focus_x, mw, 240), cam(focus_y, mh, 160));
        let anim = if self.settings.reduce_motion {
            0
        } else {
            (self.tick / 24) as usize
        };
        // tiles
        let (tx0, ty0) = (cx.div_euclid(TILE), cy.div_euclid(TILE));
        for ty in ty0..=ty0 + 10 {
            for tx in tx0..=tx0 + 15 {
                let Some(ch) = map.tile_char(tx, ty) else { continue };
                let Some(def) = self.content.tile(ch) else {
                    continue;
                };
                let art = &def.art[anim % def.art.len()];
                let (sx, sy) = (tx * TILE - cx, ty * TILE - cy);
                match self.bank.tile(art) {
                    Some(s) => c.blit(s, 0, sx, sy, 1, false),
                    None => c.rect_fill(sx, sy, TILE, TILE, [255, 0, 255, 255]),
                }
            }
        }
        // sprites sorted by feet position
        struct Spr<'a> {
            y: i32,
            art: &'a str,
            dir: Dir,
            walk: Option<Walk>,
            parity: bool,
            x: i32,
            yy: i32,
            alert: bool,
        }
        let mut sprites: Vec<Spr> = vec![];
        for (i, n) in w.npcs.iter().enumerate() {
            if !self.npc_visible(n) {
                continue;
            }
            let (ox, oy) = match n.walk {
                Some(wk) => (
                    -wk.dir.delta().0 * (TILE - wk.prog),
                    -wk.dir.delta().1 * (TILE - wk.prog),
                ),
                None => (0, 0),
            };
            let alert = matches!(w.trainer, Some(crate::world::TrainerSeq::Alert { npc, .. }) if npc == i);
            sprites.push(Spr {
                y: n.y * TILE + oy,
                art: &n.def.art,
                dir: n.facing,
                walk: n.walk,
                parity: (n.x + n.y) % 2 == 0,
                x: n.x * TILE + ox,
                yy: n.y * TILE + oy,
                alert,
            });
        }
        sprites.push(Spr {
            y: w.y * TILE + poy,
            art: "player",
            dir: w.facing,
            walk: w.walk,
            parity: w.parity,
            x: w.x * TILE + pox,
            yy: w.y * TILE + poy,
            alert: false,
        });
        sprites.sort_by_key(|s| s.y);
        for s in &sprites {
            let (frame, flip) = char_frame(s.dir, s.walk, s.parity);
            let (sx, sy) = (s.x - cx, s.yy - cy - 3);
            // soft shadow
            c.rect_fill(sx + 4, sy + 14, 8, 2, [0, 0, 0, 70]);
            match self.bank.character(s.art) {
                Some(sp) => c.blit(sp, frame, sx, sy, 1, flip),
                None => c.rect_fill(sx + 3, sy + 2, 10, 12, [255, 0, 255, 255]),
            }
            if s.alert {
                c.rect_fill(sx + 5, sy - 9, 7, 9, self.bank.pal('W'));
                c.rect_outline(sx + 5, sy - 9, 7, 9, self.bank.pal('K'));
                ui.text(c, sx + 5, sy - 11, "!", self.bank.pal('R'));
            }
        }
        // tall grass covers the player's feet
        {
            let (gx, gy) = (w.x, w.y);
            if let Some(ch) = map.tile_char(gx, gy)
                && let Some(def) = self.content.tile(ch)
                && def.kind == TileKind::TallGrass
                && let Some(s) = self.bank.tile(&def.art[anim % def.art.len()])
            {
                c.blit_region(s, 0, (0, 8, 16, 8), gx * TILE - cx, gy * TILE - cy + 8);
            }
        }
        // banner
        if let Some((text, t)) = &w.banner {
            let wpx = text.chars().count() as i32 * 6 + 16;
            let y = (-16 + (120 - *t) * 2).min(2).min(-16 + *t * 2);
            ui.panel(c, (240 - wpx) / 2, y, wpx, 16);
            ui.text(c, (240 - wpx) / 2 + 8, y + 3, text, ui.col.text);
        }
        if let Some(d) = &w.dialogue {
            self.draw_dialogue(c, ui, d);
        }
        // fades / flash
        match &w.fade {
            Fade::Out { t, .. } => c.fade_to(self.bank.pal('K'), (*t * 255 / 12).min(255) as u8),
            Fade::In { t } => c.fade_to(self.bank.pal('K'), (255 - *t * 255 / 12).max(0) as u8),
            Fade::None => {}
        }
        if let Some((_, t)) = &w.flash {
            if self.settings.reduce_motion {
                c.fade_to(self.bank.pal('K'), 150);
            } else if (*t / 4) % 2 == 0 {
                c.fade_to(self.bank.pal('W'), 190);
            } else {
                c.fade_to(self.bank.pal('K'), 120);
            }
        }
    }
}

/// Frame index and mirroring for a character sprite sheet (down, up, left; right = mirrored left).
fn char_frame(dir: Dir, walk: Option<Walk>, parity: bool) -> (usize, bool) {
    let stepping = walk.map(|w| (4..12).contains(&w.prog)).unwrap_or(false);
    match dir {
        Dir::Down => (usize::from(stepping), stepping && parity),
        Dir::Up => (2 + usize::from(stepping), stepping && parity),
        Dir::Left => (4 + usize::from(stepping), false),
        Dir::Right => (4 + usize::from(stepping), true),
    }
}
