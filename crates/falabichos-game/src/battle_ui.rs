// SPDX-License-Identifier: Apache-2.0
//! Conversation-battle screens: asking, answering (choice / type / order), feedback, outcomes.

use crate::input::Pad;
use crate::ui::{Ui, clip};
use crate::world::PendingBattle;
use crate::{Game, GameEvent, Scene};
use falabichos_core::battle::{Battle, BattleKind, Outcome, SettleEvent, settle};
use falabichos_core::exercise::{Answer, ExKind, Exercise, Flavor, Hint, Judgement};
use falabichos_core::model::Lang;
use falabichos_core::normalize::Verdict;
use falabichos_core::progress::bicho_level;
use falabichos_media::Canvas;
use falabichos_media::font::LINE_HEIGHT;

#[derive(Debug, Clone)]
pub(crate) enum BPhase {
    Intro { t: i32 },
    Ask,
    Feedback { t: i32 },
    Outro { t: i32 },
}

pub(crate) struct BattleScene {
    pub battle: Battle,
    /// Art for the opponent: Bicho sprite id or character id.
    pub opp_art: String,
    pub opp_is_char: bool,
    pub phase: BPhase,
    pub cursor: usize,
    pub input: String,
    pub kb: (usize, usize),
    pub chosen: Vec<usize>,
    pub eliminated: Vec<usize>,
    pub hint: Option<String>,
    pub judgement: Option<Judgement>,
    pub trust_gain: u8,
    pub shake: i32,
    pub levels_before: Vec<(String, u8)>,
    pub outro: Vec<String>,
    pub flee_prompt: bool,
    pub listened: bool,
}

const KB_ROWS: [&str; 3] = ["abcdefghijklm", "nopqrstuvwxyz", "áàâãéêíóôõúçü"];
const KB_BUTTONS: [&str; 4] = ["SPACE", "-", "DEL", "OK"];
const MAX_INPUT: usize = 28;

impl BattleScene {
    fn reset_question(&mut self) {
        self.cursor = 0;
        self.input.clear();
        self.kb = (0, 0);
        self.chosen.clear();
        self.eliminated.clear();
        self.hint = None;
        self.judgement = None;
        self.trust_gain = 0;
        self.listened = false;
    }
}

impl Game {
    fn party_levels(&self) -> Vec<(String, u8)> {
        self.save
            .progress
            .party
            .iter()
            .map(|b| {
                (
                    b.species.clone(),
                    bicho_level(&self.content, &self.save.srs, &b.species),
                )
            })
            .collect()
    }

    fn new_battle_scene(&self, battle: Battle, opp_art: String, opp_is_char: bool) -> Box<BattleScene> {
        Box::new(BattleScene {
            battle,
            opp_art,
            opp_is_char,
            phase: BPhase::Intro { t: 0 },
            cursor: 0,
            input: String::new(),
            kb: (0, 0),
            chosen: vec![],
            eliminated: vec![],
            hint: None,
            judgement: None,
            trust_gain: 0,
            shake: 0,
            levels_before: self.party_levels(),
            outro: vec![],
            flee_prompt: false,
            listened: false,
        })
    }

    pub(crate) fn start_battle(&mut self, pb: PendingBattle) {
        self.toast = None;
        let now = self.now();
        let policy = self.settings.policy();
        let content = self.content.clone();
        match pb {
            PendingBattle::Wild(species) => {
                let start_conf = self.save.confidence;
                match Battle::wild(
                    &content,
                    &species,
                    &mut self.save.progress,
                    &self.save.srs,
                    policy,
                    start_conf,
                    &mut self.rng,
                    now,
                ) {
                    Ok(b) => {
                        let art = content.species[&species].art.clone();
                        let cry = content.species[&species].cry_hz;
                        self.scene = Scene::Battle(self.new_battle_scene(b, art, false));
                        self.set_music(Some("battle"));
                        self.emit(GameEvent::Cry(cry));
                    }
                    Err(e) => self.battle_error(e.to_string()),
                }
            }
            PendingBattle::Challenge(id) => {
                match Battle::challenge(&content, &id, &self.save.srs, policy, &mut self.rng, now) {
                    Ok(b) => {
                        let art = content.challenges[&id].art.clone();
                        self.scene = Scene::Battle(self.new_battle_scene(b, art, true));
                        self.set_music(Some("battle"));
                    }
                    Err(e) => self.battle_error(e.to_string()),
                }
            }
        }
    }

    fn battle_error(&mut self, msg: String) {
        self.toast(msg);
        self.scene = Scene::World;
        let m = self.map().music.clone();
        self.set_music(m.as_deref());
    }

    pub(crate) fn start_practice(&mut self) {
        self.toast = None;
        let now = self.now();
        let policy = self.settings.policy();
        let content = self.content.clone();
        match Battle::practice(
            &content,
            &self.save.progress,
            &self.save.srs,
            policy,
            8,
            &mut self.rng,
            now,
        ) {
            Ok(b) => {
                let art = self
                    .save
                    .progress
                    .party
                    .first()
                    .and_then(|p| content.species.get(&p.species))
                    .map(|s| s.art.clone())
                    .unwrap_or_else(|| "galo".to_string());
                self.scene = Scene::Battle(self.new_battle_scene(b, art, false));
                self.set_music(Some("battle"));
            }
            Err(e) => {
                self.toast(e.to_string());
                self.scene = Scene::World;
            }
        }
    }

    /// The exercise currently shown in battle (for tests and the simulator).
    pub fn battle_exercise(&self) -> Option<&Exercise> {
        match &self.scene {
            Scene::Battle(b) => b.battle.current.as_ref(),
            _ => None,
        }
    }

    pub fn in_battle(&self) -> bool {
        matches!(self.scene, Scene::Battle(_))
    }

    pub(crate) fn tick_battle(&mut self, mut s: Box<BattleScene>, pad: &Pad) {
        let now = self.now();
        if s.shake > 0 {
            s.shake -= 1;
        }
        match s.phase.clone() {
            BPhase::Intro { t } => {
                if pad.a || t > 70 {
                    s.phase = BPhase::Ask;
                } else {
                    s.phase = BPhase::Intro { t: t + 1 };
                }
            }
            BPhase::Ask => {
                if s.flee_prompt {
                    if pad.a {
                        s.battle.flee();
                        self.sfx("sfx_cancel");
                        s.flee_prompt = false;
                        self.begin_outro(&mut s);
                    } else if pad.b {
                        s.flee_prompt = false;
                        self.sfx("sfx_cancel");
                    }
                } else {
                    self.tick_ask(&mut s, pad, now);
                }
            }
            BPhase::Feedback { t } => {
                s.phase = BPhase::Feedback { t: t + 1 };
                if t > 8 && (pad.a || pad.enter_pressed()) {
                    if s.battle.is_over() {
                        self.begin_outro(&mut s);
                    } else {
                        let content = self.content.clone();
                        s.battle.advance(&content, &self.save.srs, &mut self.rng, now);
                        s.reset_question();
                        s.phase = BPhase::Ask;
                        self.sfx("sfx_blip");
                    }
                }
            }
            BPhase::Outro { t } => {
                s.phase = BPhase::Outro { t: t + 1 };
                if t > 20 && pad.a {
                    self.finish_battle(s);
                    return;
                }
            }
        }
        self.scene = Scene::Battle(s);
    }

    fn tick_ask(&mut self, s: &mut BattleScene, pad: &Pad, now: i64) {
        let Some(ex) = s.battle.current.clone() else {
            s.battle.flee();
            self.begin_outro(s);
            return;
        };
        if ex.flavor == Flavor::Listen && !s.listened {
            s.listened = true;
            if let Some(a) = &ex.audio {
                self.emit(GameEvent::Clip(a.clone()));
            }
        }
        if pad.select {
            let mut rng = self.rng.clone();
            if let Some(h) = s.battle.use_hint(&mut rng) {
                self.rng = rng;
                self.sfx("sfx_hint");
                match h {
                    Hint::Eliminate(v) => {
                        s.eliminated = v;
                        if s.eliminated.contains(&s.cursor) {
                            s.cursor = (0..ex.options.len())
                                .find(|i| !s.eliminated.contains(i))
                                .unwrap_or(0);
                        }
                    }
                    Hint::Text(t) => s.hint = Some(t),
                }
            } else {
                self.sfx("sfx_bump");
                let m = self.t("battle.no_hints");
                self.toast(m);
            }
        }
        if pad.b && s.chosen.is_empty() && ex.kind != ExKind::Type
            || (pad.b && ex.kind == ExKind::Type && s.input.is_empty())
        {
            s.flee_prompt = true;
            self.sfx("sfx_select");
            return;
        }
        let mut answer: Option<Answer> = None;
        match ex.kind {
            ExKind::Choice => {
                let n = ex.options.len();
                let step = |dir: i32, cur: &mut usize| {
                    for _ in 0..n {
                        *cur = (*cur as i32 + dir).rem_euclid(n as i32) as usize;
                        if !s.eliminated.contains(cur) {
                            break;
                        }
                    }
                };
                let mut cur = s.cursor;
                if pad.down {
                    step(1, &mut cur);
                    self.sfx("sfx_select");
                }
                if pad.up {
                    step(-1, &mut cur);
                    self.sfx("sfx_select");
                }
                s.cursor = cur;
                if pad.start && ex.flavor == Flavor::Listen {
                    if let Some(a) = &ex.audio {
                        self.emit(GameEvent::Clip(a.clone()));
                    }
                } else if pad.a && !s.eliminated.contains(&s.cursor) {
                    answer = Some(Answer::Choice(s.cursor));
                }
            }
            ExKind::Type => {
                for ch in pad.typed.chars() {
                    if s.input.chars().count() < MAX_INPUT
                        && (ch.is_alphabetic() || ch == ' ' || ch == '-')
                        && self.bank.font.covers(ch)
                    {
                        s.input.push(ch);
                    }
                }
                if pad.backspace {
                    s.input.pop();
                }
                // grid navigation
                let cols = |r: usize| {
                    if r == 3 {
                        KB_BUTTONS.len()
                    } else {
                        KB_ROWS[r].chars().count()
                    }
                };
                if pad.right {
                    s.kb.1 = (s.kb.1 + 1) % cols(s.kb.0);
                    self.sfx("sfx_blip");
                }
                if pad.left {
                    s.kb.1 = (s.kb.1 + cols(s.kb.0) - 1) % cols(s.kb.0);
                    self.sfx("sfx_blip");
                }
                if pad.down || pad.up {
                    let old = s.kb;
                    let nr = if pad.down {
                        (old.0 + 1) % 4
                    } else {
                        (old.0 + 3) % 4
                    };
                    let nc = if nr == 3 {
                        (old.1 * 4 / 13).min(3)
                    } else if old.0 == 3 {
                        (old.1 * 13 / 4 + 1).min(12)
                    } else {
                        old.1.min(cols(nr) - 1)
                    };
                    s.kb = (nr, nc);
                    self.sfx("sfx_blip");
                }
                if pad.a {
                    if s.kb.0 < 3 {
                        if s.input.chars().count() < MAX_INPUT
                            && let Some(ch) = KB_ROWS[s.kb.0].chars().nth(s.kb.1)
                        {
                            s.input.push(ch);
                            self.sfx("sfx_blip");
                        }
                    } else {
                        match s.kb.1 {
                            0 => s.input.push(' '),
                            1 => s.input.push('-'),
                            2 => {
                                s.input.pop();
                            }
                            _ => answer = Some(Answer::Text(s.input.clone())),
                        }
                    }
                }
                if pad.enter && !s.input.trim().is_empty() {
                    answer = Some(Answer::Text(s.input.clone()));
                }
                if pad.b && !s.input.is_empty() {
                    s.input.pop();
                }
            }
            ExKind::Order => {
                let n = ex.options.len();
                let total = n + 1; // last entry = "Check"
                if pad.right || pad.down {
                    s.cursor = (s.cursor + 1) % total;
                    self.sfx("sfx_blip");
                }
                if pad.left || pad.up {
                    s.cursor = (s.cursor + total - 1) % total;
                    self.sfx("sfx_blip");
                }
                if pad.a {
                    if s.cursor == n {
                        if !s.chosen.is_empty() {
                            answer = Some(Answer::Order(s.chosen.clone()));
                        }
                    } else if !s.chosen.contains(&s.cursor) {
                        s.chosen.push(s.cursor);
                        self.sfx("sfx_confirm");
                        if s.chosen.len() == n {
                            s.cursor = n;
                        }
                    }
                }
                if pad.b && !s.chosen.is_empty() {
                    s.chosen.pop();
                    self.sfx("sfx_cancel");
                }
            }
        }
        if let Some(a) = answer {
            let content = self.content.clone();
            let _ = &content;
            if let Some(report) = s.battle.answer(&a, &mut self.save.srs, now) {
                s.trust_gain = report.trust_gain;
                if report.judgement.correct {
                    self.sfx("sfx_correct");
                } else {
                    self.sfx("sfx_wrong");
                    s.shake = 12;
                }
                if let Some(a) = &ex.audio {
                    self.emit(GameEvent::Clip(a.clone()));
                } else if let Some(v) = self.content.vocab.get(&ex.skill)
                    && self.content.has_clip(&v.audio_slot())
                {
                    self.emit(GameEvent::Clip(v.audio_slot()));
                }
                s.judgement = Some(report.judgement);
                s.phase = BPhase::Feedback { t: 0 };
            }
        }
    }

    fn begin_outro(&mut self, s: &mut BattleScene) {
        let mut lines: Vec<String> = vec![];
        let name = s.battle.name.clone();
        match (&s.battle.kind, s.battle.outcome) {
            (BattleKind::Wild { .. }, Some(Outcome::Caught)) => {
                lines.push(self.tf("battle.caught", &[("name", &name)]));
                self.set_music(None);
                self.sfx("caught");
                let cry = match &s.battle.kind {
                    BattleKind::Wild { species } => self.content.species[species].cry_hz,
                    _ => 440,
                };
                self.emit(GameEvent::Cry(cry));
            }
            (BattleKind::Wild { .. }, Some(Outcome::Lost)) => {
                lines.push(self.tf("battle.fled_shy", &[("name", &name)]));
                self.sfx("sfx_wrong");
            }
            (BattleKind::Wild { .. }, _) => lines.push(self.t("battle.got_away")),
            (BattleKind::Challenge { .. }, Some(Outcome::Won)) => {
                lines.push(self.tf("battle.won", &[("name", &name)]));
                self.set_music(None);
                self.sfx("victory");
            }
            (BattleKind::Challenge { .. }, Some(Outcome::Lost)) => {
                lines.push(self.t("battle.lost"));
                self.sfx("sfx_wrong");
            }
            (BattleKind::Practice, Some(Outcome::Completed)) => {
                lines.push(self.tf(
                    "battle.practice_done",
                    &[
                        ("right", &s.battle.correct.to_string()),
                        ("total", &s.battle.asked.to_string()),
                    ],
                ));
                self.sfx("sfx_levelup");
            }
            _ => lines.push(self.t("battle.left")),
        }
        lines.push(self.tf(
            "battle.score",
            &[
                ("right", &s.battle.correct.to_string()),
                ("wrong", &s.battle.wrong.to_string()),
            ],
        ));
        // level-ups
        let after = self.party_levels();
        for (sp, lv) in &after {
            if let Some((_, before)) = s.levels_before.iter().find(|(b, _)| b == sp)
                && lv > before
            {
                let nm = self.content.species[sp].name.clone();
                lines.push(self.tf("battle.levelup", &[("name", &nm), ("level", &lv.to_string())]));
                self.sfx("sfx_levelup");
            }
        }
        if s.battle.outcome == Some(Outcome::Caught) {
            // the new Bicho is added in finish_battle; show its first level here
        }
        s.outro = lines;
        s.phase = BPhase::Outro { t: 0 };
    }

    fn finish_battle(&mut self, s: Box<BattleScene>) {
        let now = self.now();
        let content = self.content.clone();
        let events = settle(&s.battle, &content, &mut self.save.progress, now);
        let outcome = s.battle.outcome;
        let mut after_dialogue: Option<String> = None;
        let mut blackout = false;
        match &s.battle.kind {
            BattleKind::Wild { .. } => {
                self.save.confidence = Some(s.battle.confidence).filter(|c| *c < s.battle.max_confidence);
                if outcome == Some(Outcome::Lost) && s.battle.confidence == 0 {
                    blackout = true;
                }
            }
            BattleKind::Challenge { id } => {
                if let Some(ch) = content.challenges.get(id) {
                    after_dialogue = match outcome {
                        Some(Outcome::Won) => ch.win_dialogue.clone(),
                        _ => ch.lose_dialogue.clone(),
                    };
                }
            }
            BattleKind::Practice => {}
        }
        let mut saved = false;
        for e in &events {
            match e {
                SettleEvent::Caught(sp) => {
                    let nm = content.species[sp].name.clone();
                    let m = self.tf("toast.new_bicho", &[("name", &nm)]);
                    self.toast(m);
                    saved = true;
                }
                SettleEvent::Seal(sl) => {
                    let nm = content.seals[sl].name.clone();
                    let m = self.tf("toast.new_seal", &[("name", &nm)]);
                    self.toast(m);
                    saved = true;
                }
            }
        }
        self.scene = Scene::World;
        let m = self.map().music.clone();
        self.set_music(m.as_deref());
        if blackout {
            self.save.confidence = None;
            let st = content.story.clone().expect("story");
            let msg = self.t("battle.blackout");
            self.toast(msg);
            self.begin_warp(
                st.start_map,
                st.start_x,
                st.start_y,
                falabichos_core::model::Dir::Up,
            );
        }
        if saved {
            self.save_game();
        }
        if let Some(d) = after_dialogue {
            self.start_dialogue(&d, None);
        }
        // story ending
        if let Some(st) = &content.story
            && self.save.progress.has_flag(&st.finale_flag)
            && !self.save.progress.has_flag("finale_seen")
        {
            self.save.progress.flags.insert("finale_seen".to_string());
            if self.world.dialogue.is_none() {
                self.start_dialogue(&st.finale_dialogue, None);
            } else if let Some(d) = self.world.dialogue.as_mut() {
                // queue the finale after the win dialogue
                if let Some(fin) = content
                    .dialogues
                    .get(&st.finale_dialogue)
                    .and_then(|d| d.branches.first())
                {
                    d.lines.extend(fin.lines.clone());
                }
            }
        }
    }

    // ------------------------------------------------------------------ drawing

    pub(crate) fn draw_battle(&self, c: &mut Canvas, s: &BattleScene) {
        let ui = Ui::new(
            &self.bank,
            self.settings.high_contrast,
            self.settings.text_size == falabichos_core::save::TextSize::Large,
        );
        let b = &self.bank;
        let hc = self.settings.high_contrast;
        // backdrop
        c.clear(b.pal(if hc { 'K' } else { 'B' }));
        if !hc {
            c.rect_fill(0, 0, 240, 26, b.pal('c'));
            c.rect_fill(0, 44, 240, 24, b.pal('E'));
            c.rect_fill(0, 44, 240, 2, b.pal('l'));
            c.rect_fill(150, 52, 80, 6, b.pal('e'));
        }
        // opponent
        let shake = if s.shake > 0 && !self.settings.reduce_motion {
            (s.shake % 4 - 2) * 2
        } else {
            0
        };
        let bob = if self.settings.reduce_motion {
            0
        } else {
            ((self.tick / 20) % 2) as usize
        };
        let (ox, oy) = (178 + shake, 6);
        if s.opp_is_char {
            if let Some(sp) = b.character(&s.opp_art) {
                c.blit(sp, 0, ox, oy, 3, false);
            }
        } else if let Some(sp) = b.bicho(&s.opp_art) {
            let intro_slide = if let BPhase::Intro { t } = s.phase {
                (30 - t).max(0) * 4
            } else {
                0
            };
            c.blit(sp, bob, ox + intro_slide, oy, 3, true);
        }
        // HUD
        self.draw_battle_hud(c, &ui, s);
        // bottom panel
        ui.panel(c, 2, 64, 236, 95);
        match &s.phase {
            BPhase::Intro { .. } => {
                let msg = match &s.battle.kind {
                    BattleKind::Wild { .. } => self.tf("battle.appears", &[("name", &s.battle.name)]),
                    BattleKind::Challenge { id } => {
                        let theme = self
                            .content
                            .challenges
                            .get(id)
                            .map(|c| c.theme.clone())
                            .unwrap_or_default();
                        format!(
                            "{}\n{}",
                            self.tf("battle.challenge_intro", &[("name", &s.battle.name)]),
                            theme
                        )
                    }
                    BattleKind::Practice => self.t("battle.practice_intro"),
                };
                ui.text_wrap(c, 10, 76, 36, &msg, ui.col.text, 3);
                ui.text_wrap(c, 10, 134, 37, &self.t("battle.keys_intro"), ui.col.dim, 2);
            }
            BPhase::Ask => self.draw_ask(c, &ui, s),
            BPhase::Feedback { .. } => self.draw_feedback(c, &ui, s),
            BPhase::Outro { .. } => {
                for (i, l) in s.outro.iter().enumerate() {
                    ui.text_wrap(
                        c,
                        10,
                        74 + i as i32 * 24,
                        36,
                        l,
                        if i == 0 { ui.col.text } else { ui.col.dim },
                        2,
                    );
                }
                if (self.tick / 20).is_multiple_of(2) {
                    ui.text(c, 224, 146, "\u{25BC}", ui.col.accent);
                }
            }
        }
        if s.flee_prompt {
            ui.panel(c, 40, 78, 160, 42);
            ui.text_center(c, 120, 84, &self.t("battle.leave_q"), ui.col.text, 1);
            ui.text_center(c, 120, 100, &self.t("battle.leave_keys"), ui.col.dim, 1);
        }
    }

    fn draw_battle_hud(&self, c: &mut Canvas, ui: &Ui, s: &BattleScene) {
        let bt = &s.battle;
        ui.panel(c, 4, 3, 160, 29);
        ui.text(c, 10, 5, &clip(&bt.name, 20), ui.col.text);
        match &bt.kind {
            BattleKind::Wild { species } => {
                let lv = self.content.species.get(species).map(|_| "").unwrap_or("");
                let _ = lv;
                ui.text(c, 10, 17, &self.t("battle.trust"), ui.col.dim);
                let n = bt.trust_needed as i32;
                let w = (n * 9).min(104);
                for i in 0..n {
                    let x = 56 + i * (w / n.max(1));
                    let filled = i < bt.trust as i32;
                    ui.text(
                        c,
                        x,
                        17,
                        "\u{2665}",
                        if filled { ui.bank.pal('R') } else { ui.col.dim },
                    );
                }
            }
            BattleKind::Challenge { .. } => {
                let total = bt.total.unwrap_or(1);
                ui.text(
                    c,
                    10,
                    17,
                    &self.tf(
                        "battle.progress",
                        &[
                            (
                                "n",
                                &(bt.asked
                                    + u32::from(
                                        !matches!(s.phase, BPhase::Outro { .. }) && bt.asked < total as u32,
                                    ))
                                .min(total as u32)
                                .to_string(),
                            ),
                            ("total", &total.to_string()),
                            ("need", &bt.pass_correct.to_string()),
                        ],
                    ),
                    ui.col.dim,
                );
            }
            BattleKind::Practice => {
                let total = bt.total.unwrap_or(1);
                ui.text(
                    c,
                    10,
                    17,
                    &self.tf(
                        "battle.progress_practice",
                        &[
                            ("n", &(bt.asked + 1).min(total as u32).to_string()),
                            ("total", &total.to_string()),
                        ],
                    ),
                    ui.col.dim,
                );
            }
        }
        // confidence
        if !matches!(bt.kind, BattleKind::Practice) {
            ui.panel(c, 4, 34, 160, 15);
            ui.text(c, 10, 37, &self.t("battle.confidence"), ui.col.dim);
            let max = bt.max_confidence as i32;
            let step = (84 / max.max(1)).min(9);
            for i in 0..max {
                let filled = i < bt.confidence as i32;
                ui.text(
                    c,
                    74 + i * step,
                    37,
                    "\u{2665}",
                    if filled { ui.col.good } else { ui.col.dim },
                );
            }
        }
        if bt.hints_left > 0 && !matches!(bt.kind, BattleKind::Practice) {
            ui.text(c, 130, 5, &format!("\u{2605}{}", bt.hints_left), ui.col.accent);
        }
    }

    fn prompt_scale(&self, ui: &Ui, text: &str) -> i32 {
        if ui.scale > 1 && self.bank.font.wrap(text, Ui::cols(224, 2)).len() <= 2 {
            2
        } else {
            1
        }
    }

    fn flavor_caption(&self, ex: &Exercise) -> String {
        self.t(match ex.flavor {
            Flavor::PtToEn => "battle.q.pt_to_en",
            Flavor::EnToPt => "battle.q.en_to_pt",
            Flavor::Article => "battle.q.article",
            Flavor::Listen => "battle.q.listen",
            Flavor::ListenIpa => "battle.q.listen_ipa",
            Flavor::Sentence => "battle.q.order",
            Flavor::Grammar => match ex.kind {
                ExKind::Order => "battle.q.order",
                ExKind::Type => "battle.q.type",
                ExKind::Choice => "battle.q.grammar",
            },
        })
    }

    fn draw_ask(&self, c: &mut Canvas, ui: &Ui, s: &BattleScene) {
        let Some(ex) = &s.battle.current else { return };
        ui.text(c, 8, 67, &clip(&self.flavor_caption(ex), 38), ui.col.dim);
        let mut prompt = if ex.prompt.is_empty() {
            self.t("battle.listen_prompt")
        } else {
            ex.prompt.clone()
        };
        // Type/Order screens need the room below for the keyboard / tiles: one-line prompts only.
        let compact = ex.kind != ExKind::Choice;
        if compact {
            prompt = clip(&prompt, 36);
        }
        let sc = if compact {
            1
        } else {
            self.prompt_scale(ui, &prompt)
        };
        let pcol = if ex.prompt_lang == Lang::Pt {
            ui.col.accent
        } else {
            ui.col.text
        };
        let cols = Ui::cols(224, sc);
        let lines = self.bank.font.wrap(&prompt, cols);
        let mut y = 78;
        for l in lines.iter().take(if compact { 1 } else { 2 }) {
            ui.text_scaled(c, 10, y, l, pcol, sc);
            y += LINE_HEIGHT * sc;
        }
        if let Some(h) = &s.hint {
            ui.text(
                c,
                10,
                y,
                &clip(&format!("{} {}", self.t("battle.hint"), h), 37),
                ui.col.good,
            );
            y += 12;
        } else if let Some(ctx) = &ex.context {
            ui.text(c, 10, y, &clip(ctx, 37), ui.col.dim);
            y += 12;
        }
        match ex.kind {
            ExKind::Choice => {
                let top = y + 1;
                for (i, o) in ex.options.iter().enumerate() {
                    let oy = top + i as i32 * 11;
                    if s.eliminated.contains(&i) {
                        ui.text(c, 24, oy, "-", ui.col.dim);
                        continue;
                    }
                    if i == s.cursor {
                        c.rect_fill(8, oy - 1, 224, 11, ui.col.highlight);
                        ui.glyph_cursor(c, 10, oy);
                    }
                    ui.text(c, 24, oy, &clip(o, 34), ui.col.text);
                }
            }
            ExKind::Type => self.draw_keyboard(c, ui, s),
            ExKind::Order => self.draw_order(c, ui, s, ex),
        }
    }

    fn draw_keyboard(&self, c: &mut Canvas, ui: &Ui, s: &BattleScene) {
        // text field
        c.rect_fill(10, 100, 220, 12, ui.col.highlight);
        c.rect_outline(10, 100, 220, 12, ui.col.border);
        let cursor = if (self.tick / 20).is_multiple_of(2) {
            "_"
        } else {
            " "
        };
        if s.input.is_empty() {
            ui.text(
                c,
                14,
                101,
                &format!("{cursor}{}", self.t("battle.type_here")),
                ui.col.dim,
            );
        } else {
            ui.text(c, 14, 101, &format!("{}{}", s.input, cursor), ui.col.text);
        }
        let (x0, y0) = (16, 114);
        for (r, row) in KB_ROWS.iter().enumerate() {
            for (k, ch) in row.chars().enumerate() {
                let (x, y) = (x0 + k as i32 * 16, y0 + r as i32 * 11);
                let sel = s.kb == (r, k);
                if sel {
                    c.rect_fill(x - 1, y - 1, 15, 11, ui.col.accent);
                }
                ui.text(
                    c,
                    x + 4,
                    y - 1,
                    &ch.to_string(),
                    if sel { ui.col.panel } else { ui.col.text },
                );
            }
        }
        for (k, label) in KB_BUTTONS.iter().enumerate() {
            let (x, y) = (x0 + k as i32 * 52, y0 + 33);
            let sel = s.kb == (3, k);
            c.rect_fill(
                x,
                y - 1,
                50,
                11,
                if sel { ui.col.accent } else { ui.col.highlight },
            );
            ui.text_center(
                c,
                x + 25,
                y - 1,
                label,
                if sel { ui.col.panel } else { ui.col.text },
                1,
            );
        }
    }

    fn draw_order(&self, c: &mut Canvas, ui: &Ui, s: &BattleScene, ex: &Exercise) {
        // chosen words line
        let chosen: Vec<&str> = s
            .chosen
            .iter()
            .filter_map(|&i| ex.options.get(i))
            .map(String::as_str)
            .collect();
        c.rect_fill(10, 100, 220, 21, ui.col.highlight);
        c.rect_outline(10, 100, 220, 21, ui.col.border);
        ui.text_wrap(c, 14, 99, 35, &chosen.join(" "), ui.col.text, 2);
        // tiles
        let (mut x, mut y) = (12, 124);
        for (i, w) in ex.options.iter().enumerate() {
            let wpx = w.chars().count() as i32 * 6 + 8;
            if x + wpx > 230 {
                x = 12;
                y += 12;
            }
            let used = s.chosen.contains(&i);
            let sel = s.cursor == i;
            let bg = if sel {
                ui.col.accent
            } else if used {
                ui.col.panel
            } else {
                ui.col.highlight
            };
            c.rect_fill(x, y, wpx, 11, bg);
            c.rect_outline(x, y, wpx, 11, if used { ui.col.dim } else { ui.col.border });
            let fg = if sel {
                ui.col.panel
            } else if used {
                ui.col.dim
            } else {
                ui.col.text
            };
            ui.text(c, x + 4, y - 1, w, fg);
            x += wpx + 4;
        }
        // check button
        let sel = s.cursor == ex.options.len();
        let label = self.t("battle.check");
        let wpx = label.chars().count() as i32 * 6 + 10;
        if x + wpx > 230 {
            x = 12;
            y += 12;
        }
        c.rect_fill(x, y, wpx, 11, if sel { ui.col.good } else { ui.col.highlight });
        c.rect_outline(x, y, wpx, 11, ui.col.border);
        ui.text(
            c,
            x + 5,
            y - 1,
            &label,
            if sel { ui.col.panel } else { ui.col.text },
        );
    }

    fn draw_feedback(&self, c: &mut Canvas, ui: &Ui, s: &BattleScene) {
        let (Some(j), Some(ex)) = (&s.judgement, &s.battle.current) else {
            return;
        };
        let (head, col) = if j.correct {
            (self.t("battle.correct"), ui.col.good)
        } else {
            (self.t("battle.wrong"), ui.col.bad)
        };
        ui.text(c, 10, 68, if j.correct { "\u{2713}" } else { "\u{2717}" }, col);
        ui.text(c, 22, 68, &head, col);
        if s.trust_gain > 0 {
            ui.text(c, 150, 68, &format!("+{} \u{2665}", s.trust_gain), ui.col.good);
        }
        // verdict notes
        let mut y = 80;
        match &j.verdict {
            Some(Verdict::AccentSlip { expected }) => {
                ui.text(
                    c,
                    10,
                    y,
                    &clip(&self.tf("battle.accent_note", &[("word", expected)]), 38),
                    ui.col.accent,
                );
                y += 12;
            }
            Some(Verdict::Typo { expected }) => {
                ui.text(
                    c,
                    10,
                    y,
                    &clip(&self.tf("battle.typo_note", &[("word", expected)]), 38),
                    ui.col.accent,
                );
                y += 12;
            }
            _ => {}
        }
        ui.text(
            c,
            10,
            y,
            &clip(&format!("{} {}", self.t("battle.answer"), j.expected), 38),
            ui.col.text,
        );
        y += 12;
        if let Some(e) = &j.explanation {
            let n = ui.text_wrap(c, 10, y, 37, e, ui.col.dim, 3);
            y += n as i32 * LINE_HEIGHT;
        }
        if let Some(ipa) = &ex.ipa
            && y <= 134
        {
            ui.text(c, 10, y, &format!("[{ipa}]"), ui.col.accent);
            y += 12;
        }
        if !j.correct
            && let Some(tip) = ex.pron_tags.first().and_then(|t| self.content.pron_tip(*t))
            && y <= 136
        {
            ui.text(
                c,
                10,
                y,
                &clip(&format!("{}: {}", self.t("battle.tip"), tip.title), 38),
                ui.col.dim,
            );
        }
        if (self.tick / 20).is_multiple_of(2) {
            ui.text(c, 224, 146, "\u{25BC}", ui.col.accent);
        }
    }
}

trait PadExt {
    fn enter_pressed(&self) -> bool;
}
impl PadExt for Pad {
    fn enter_pressed(&self) -> bool {
        self.enter
    }
}

impl Game {
    /// True while the player is typing an answer: the shell should treat letter keys as text.
    pub fn wants_text_input(&self) -> bool {
        match &self.scene {
            Scene::Battle(b) => {
                matches!(b.phase, BPhase::Ask)
                    && !b.flee_prompt
                    && b.battle
                        .current
                        .as_ref()
                        .map(|e| e.kind == ExKind::Type)
                        .unwrap_or(false)
            }
            _ => false,
        }
    }
}
