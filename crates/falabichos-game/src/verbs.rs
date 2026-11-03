// SPDX-License-Identifier: Apache-2.0
//! The verb tables screen: browse the conjugation tables (pretérito perfeito, imperfeito, futuro) and
//! start a typed drill for one verb or for all of them.

use crate::input::Pad;
use crate::ui::{Ui, clip};
use crate::{Game, Scene};
use falabichos_core::model::{PERSONS, Tense};
use falabichos_media::Canvas;

/// Questions in one conjugation drill.
pub const DRILL_QUESTIONS: u8 = 8;
/// A form counts as learned (drawn green) from this mastery on.
const LEARNED: f32 = 0.5;

pub(crate) struct VerbsState {
    /// Index into the verbs (in id order).
    pub cursor: usize,
    /// Index into [`Tense::ALL`].
    pub tense: usize,
}

impl Game {
    fn verb_ids(&self) -> Vec<String> {
        self.content.verbs.keys().cloned().collect()
    }

    /// Mean mastery of the 15 forms of a verb.
    fn verb_mastery(&self, id: &str) -> f32 {
        let skills = self.content.conjugation_skills(Some(id));
        if skills.is_empty() {
            return 0.0;
        }
        skills.iter().map(|s| self.save.srs.mastery(s)).sum::<f32>() / skills.len() as f32
    }

    pub(crate) fn tick_verbs(&mut self, mut s: VerbsState, pad: &Pad) {
        let ids = self.verb_ids();
        if pad.b || pad.start {
            self.sfx("sfx_cancel");
            self.scene = Scene::Menu(crate::menus::MenuState::new(""));
            return;
        }
        let n = ids.len().max(1);
        if pad.down {
            s.cursor = (s.cursor + 1) % n;
            self.sfx("sfx_select");
        }
        if pad.up {
            s.cursor = (s.cursor + n - 1) % n;
            self.sfx("sfx_select");
        }
        let t = Tense::ALL.len();
        if pad.right {
            s.tense = (s.tense + 1) % t;
            self.sfx("sfx_blip");
        }
        if pad.left {
            s.tense = (s.tense + t - 1) % t;
            self.sfx("sfx_blip");
        }
        if pad.a || pad.select {
            let only = if pad.select {
                None
            } else {
                ids.get(s.cursor).cloned()
            };
            self.sfx("sfx_confirm");
            self.start_conjugation(only.as_deref(), DRILL_QUESTIONS);
            return;
        }
        self.scene = Scene::Verbs(s);
    }

    pub(crate) fn draw_verbs(&self, c: &mut Canvas, ui: &Ui, s: &VerbsState) {
        c.clear(self.bank.pal(if self.settings.high_contrast { 'K' } else { 'n' }));
        ui.text(c, 8, 6, &self.t("verbs.title"), self.bank.pal('W'));
        let ids = self.verb_ids();
        // list
        ui.panel(c, 4, 18, 84, 126);
        for (i, id) in ids.iter().enumerate().take(10) {
            let y = 24 + i as i32 * 11;
            let v = &self.content.verbs[id];
            if i == s.cursor {
                c.rect_fill(7, y - 2, 78, 11, ui.col.highlight);
                ui.glyph_cursor(c, 8, y);
            }
            ui.text(c, 18, y, &clip(&v.inf, 7), ui.col.text);
            ui.bar(c, 62, y + 1, 22, 6, self.verb_mastery(id), ui.col.good);
        }
        // table
        ui.panel(c, 92, 18, 144, 126);
        let Some(v) = ids.get(s.cursor).and_then(|id| self.content.verbs.get(id)) else {
            return;
        };
        let tense = Tense::ALL[s.tense % Tense::ALL.len()];
        ui.text(c, 98, 22, &clip(&v.inf, 22), ui.col.accent);
        ui.text(c, 98, 33, &clip(&v.en, 22), ui.col.dim);
        ui.text(
            c,
            98,
            46,
            &format!("\u{25C4} {} \u{25BA}", clip(tense.pt(), 20)),
            ui.col.text,
        );
        for (p, form) in v.forms(tense).iter().enumerate() {
            let y = 59 + p as i32 * 11;
            let skill = falabichos_core::Content::conjugation_skill(&v.id, tense, p);
            let learned = self.save.srs.mastery(&skill) >= LEARNED;
            ui.text(c, 98, y, PERSONS[p], ui.col.dim);
            ui.text(
                c,
                98 + 10 * 6,
                y,
                &clip(form, 12),
                if learned { ui.col.good } else { ui.col.text },
            );
        }
        if let Some(note) = &v.note {
            ui.text_wrap(c, 98, 117, 23, note, ui.col.dim, 3);
        }
        ui.text(c, 8, 148, &clip(&self.t("verbs.hint"), 38), self.bank.pal('W'));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GameConfig, Input, MemoryStorage};
    use falabichos_assets::Assets;
    use std::sync::Arc;

    fn game() -> Game {
        let mut g = Game::new(
            Arc::new(Assets::embedded()),
            Box::new(MemoryStorage::default()),
            GameConfig::default(),
        )
        .unwrap();
        g.set_clock(|| 1_000_000);
        g.debug_teleport("vila_azul");
        g
    }

    fn tap(g: &mut Game, f: impl Fn(&mut Input)) {
        let mut i = Input::default();
        f(&mut i);
        g.tick(&i);
        g.tick(&Input::default());
    }

    fn open(g: &mut Game) {
        tap(g, |i| i.start = true);
        let at = crate::menus::MenuState::new("")
            .items
            .iter()
            .position(|m| *m == crate::menus::MenuItem::Verbs)
            .unwrap();
        for _ in 0..at {
            tap(g, |i| i.down = true);
        }
        tap(g, |i| i.a = true);
    }

    #[test]
    fn verb_tables_open_from_the_menu_and_every_table_draws() {
        let mut g = game();
        open(&mut g);
        assert_eq!(g.scene_name(), "verbs");
        let n = g.content.verbs.len();
        for v in 0..n {
            for t in 0..3 {
                let mut c = Canvas::new(crate::LOGICAL_W, crate::LOGICAL_H);
                g.draw(&mut c);
                if v == 0
                    && t == 0
                    && let Ok(dir) = std::env::var("FALABICHOS_TEST_PNG_DIR")
                {
                    std::fs::write(format!("{dir}/verbs.png"), c.upscaled(3).to_png()).unwrap();
                }
                tap(&mut g, |i| i.right = true);
            }
            tap(&mut g, |i| i.down = true);
        }
        // wrapped all the way around
        let Scene::Verbs(s) = &g.scene else { panic!() };
        assert_eq!((s.cursor, s.tense), (0, 0));
        tap(&mut g, |i| i.b = true);
        assert_eq!(g.scene_name(), "menu");
    }

    #[test]
    fn a_starts_a_drill_for_the_selected_verb_and_select_for_all() {
        let mut g = game();
        open(&mut g);
        tap(&mut g, |i| i.down = true); // second verb
        let second = g.content.verbs.keys().nth(1).unwrap().clone();
        tap(&mut g, |i| i.a = true);
        assert!(g.in_battle());
        let ex = g.battle_exercise().unwrap().clone();
        assert!(ex.skill.starts_with(&format!("conj_{second}_")), "{}", ex.skill);
        // Select drills every verb
        g.scene = Scene::World;
        open(&mut g);
        tap(&mut g, |i| i.select = true);
        assert!(g.in_battle());
        assert!(g.battle_exercise().unwrap().skill.starts_with("conj_"));
    }

    #[test]
    fn learned_forms_show_in_the_table() {
        let mut g = game();
        g.scene = Scene::Verbs(VerbsState { cursor: 0, tense: 0 });
        let before = {
            let mut c = Canvas::new(crate::LOGICAL_W, crate::LOGICAL_H);
            g.draw(&mut c);
            (0..c.h)
                .flat_map(|y| (0..c.w).map(move |x| (x, y)))
                .map(|(x, y)| c.get(x, y))
                .collect::<Vec<_>>()
        };
        let id = g.content.verbs.keys().next().unwrap().clone();
        for p in 0..PERSONS.len() {
            let skill = falabichos_core::Content::conjugation_skill(&id, Tense::Perfeito, p);
            for k in 0..4 {
                g.save
                    .srs
                    .review(&skill, falabichos_core::srs::Grade::Good, 1000 + k * 1000);
            }
        }
        assert!(g.verb_mastery(&id) > 0.0);
        let mut c = Canvas::new(crate::LOGICAL_W, crate::LOGICAL_H);
        g.draw(&mut c);
        let after: Vec<_> = (0..c.h)
            .flat_map(|y| (0..c.w).map(move |x| (x, y)))
            .map(|(x, y)| c.get(x, y))
            .collect();
        assert_ne!(before, after, "mastery bars and green forms appear");
    }
}
