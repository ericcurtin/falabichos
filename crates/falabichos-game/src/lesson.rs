// SPDX-License-Identifier: Apache-2.0
//! Mini-lessons: before the first conversation with a grammar Bicho, the player is shown a short
//! page for each grammar concept that Bicho teaches (the same page as in the Notebook).
//!
//! Which pages were already shown is stored as the story flags `lesson_<concept id>`, so the
//! save format does not change and the lesson never repeats.

use crate::input::Pad;
use crate::ui::{Ui, clip};
use crate::world::PendingBattle;
use crate::{Game, Scene};
use falabichos_core::model::SpeciesKind;
use falabichos_media::Canvas;

/// Visible text lines on a lesson page.
const PAGE_LINES: usize = 9;

pub(crate) struct LessonState {
    /// Grammar concept ids to show, in order.
    pub pages: Vec<String>,
    pub page: usize,
    pub scroll: usize,
    /// The battle that starts when the lessons are over.
    pub then: PendingBattle,
}

/// The story flag that records that a concept's mini-lesson was shown.
pub fn lesson_flag(concept: &str) -> String {
    format!("lesson_{concept}")
}

impl Game {
    /// Concepts of a grammar Bicho whose mini-lesson has not been shown yet.
    pub(crate) fn unseen_lessons(&self, species: &str) -> Vec<String> {
        let Some(sp) = self.content.species.get(species) else {
            return vec![];
        };
        if sp.kind != SpeciesKind::Grammar {
            return vec![];
        }
        sp.grammar
            .iter()
            .filter(|g| self.content.grammar.contains_key(*g))
            .filter(|g| !self.save.progress.has_flag(&lesson_flag(g)))
            .cloned()
            .collect()
    }

    /// Show the mini-lessons that are due before `pb`; returns false if there are none.
    pub(crate) fn open_lessons_before(&mut self, pb: &PendingBattle) -> bool {
        let PendingBattle::Wild(species) = pb else {
            return false;
        };
        let pages = self.unseen_lessons(species);
        if pages.is_empty() {
            return false;
        }
        // Mark them all as shown right away: a lesson never nags twice (the Notebook keeps the pages).
        for g in &pages {
            self.save.progress.flags.insert(lesson_flag(g));
        }
        self.toast = None;
        self.sfx("sfx_confirm");
        self.scene = Scene::Lesson(Box::new(LessonState {
            pages,
            page: 0,
            scroll: 0,
            then: pb.clone(),
        }));
        true
    }

    pub(crate) fn tick_lesson(&mut self, mut s: Box<LessonState>, pad: &Pad) {
        if pad.b || pad.start {
            // skip the rest of the lessons
            self.sfx("sfx_cancel");
            self.start_battle_now(s.then);
            return;
        }
        let concept = &self.content.grammar[&s.pages[s.page]];
        let total = self.concept_lines(concept).len();
        if pad.down && s.scroll + PAGE_LINES < total {
            s.scroll += 1;
        }
        if pad.up && s.scroll > 0 {
            s.scroll -= 1;
        }
        if pad.a {
            if s.page + 1 < s.pages.len() {
                s.page += 1;
                s.scroll = 0;
                self.sfx("sfx_confirm");
            } else {
                self.sfx("sfx_confirm");
                self.start_battle_now(s.then);
                return;
            }
        }
        self.scene = Scene::Lesson(s);
    }

    pub(crate) fn draw_lesson(&self, c: &mut Canvas, ui: &Ui, s: &LessonState) {
        c.clear(self.bank.pal(if self.settings.high_contrast { 'K' } else { 'n' }));
        let head = self.tf(
            "lesson.title",
            &[
                ("n", &(s.page + 1).to_string()),
                ("total", &s.pages.len().to_string()),
            ],
        );
        ui.text(c, 8, 6, &head, self.bank.pal('W'));
        let concept = &self.content.grammar[&s.pages[s.page]];
        ui.panel(c, 4, 18, 232, 126);
        ui.text(c, 10, 21, &clip(&concept.title, 36), ui.col.accent);
        let lines = self.concept_lines(concept);
        for (i, (l, style)) in lines.iter().skip(s.scroll).take(PAGE_LINES).enumerate() {
            let col = match style {
                1 => ui.col.accent,
                2 => ui.col.dim,
                3 => ui.col.good,
                _ => ui.col.text,
            };
            ui.text(c, 10, 34 + i as i32 * 11, l, col);
        }
        let more = s.scroll + PAGE_LINES < lines.len();
        if more {
            ui.text(c, 224, 133, "\u{25BC}", ui.col.dim);
        }
        let last = s.page + 1 == s.pages.len();
        let hint = self.t(if last { "lesson.hint_last" } else { "lesson.hint" });
        ui.text(c, 8, 148, &clip(&hint, 38), self.bank.pal('W'));
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

    #[test]
    fn only_grammar_bichos_have_lessons_and_each_is_shown_once() {
        let mut g = game();
        assert!(
            g.unseen_lessons("galo").is_empty(),
            "vocabulary Bichos have no lessons"
        );
        assert!(g.unseen_lessons("no_such_bicho").is_empty());
        let all = g.content.species["coruja"].grammar.clone();
        assert_eq!(g.unseen_lessons("coruja"), all);
        g.save.progress.flags.insert(lesson_flag(&all[0]));
        assert_eq!(g.unseen_lessons("coruja"), all[1..].to_vec());
    }

    #[test]
    fn first_wild_battle_with_a_grammar_bicho_shows_lessons_then_battles() {
        let mut g = game();
        let pages = g.content.species["camaleao"].grammar.len();
        assert!(pages >= 2);
        g.start_battle(PendingBattle::Wild("camaleao".into()));
        assert_eq!(g.scene_name(), "lesson");
        assert!(!g.in_battle());
        // a frame is drawn for every page (a layout panic would fail the test)
        for page in 0..pages {
            let mut c = Canvas::new(crate::LOGICAL_W, crate::LOGICAL_H);
            g.draw(&mut c);
            if page == 0
                && let Ok(dir) = std::env::var("FALABICHOS_TEST_PNG_DIR")
            {
                std::fs::write(format!("{dir}/lesson.png"), c.upscaled(3).to_png()).unwrap();
            }
            assert_eq!(g.scene_name(), "lesson", "page {page}");
            // scrolling never goes out of range
            for _ in 0..40 {
                tap(&mut g, |i| i.down = true);
            }
            for _ in 0..40 {
                tap(&mut g, |i| i.up = true);
            }
            tap(&mut g, |i| i.a = true);
        }
        assert!(g.in_battle(), "the battle starts after the last page");
        for id in &g.content.species["camaleao"].grammar {
            assert!(g.progress().has_flag(&lesson_flag(id)));
        }
        // second meeting: straight into the battle
        g.scene = Scene::World;
        g.start_battle(PendingBattle::Wild("camaleao".into()));
        assert!(g.in_battle());
    }

    #[test]
    fn b_skips_all_lessons_and_never_nags_again() {
        let mut g = game();
        g.start_battle(PendingBattle::Wild("coruja".into()));
        assert_eq!(g.scene_name(), "lesson");
        tap(&mut g, |i| i.b = true);
        assert!(g.in_battle());
        assert!(g.unseen_lessons("coruja").is_empty());
    }

    #[test]
    fn vocabulary_bichos_and_challenges_skip_lessons() {
        let mut g = game();
        g.start_battle(PendingBattle::Wild("galo".into()));
        assert!(g.in_battle());
        g.scene = Scene::World;
        g.start_battle(PendingBattle::Challenge("mestre_joaquim".into()));
        assert!(g.in_battle());
        assert!(
            g.unseen_lessons("camaleao").len() == g.content.species["camaleao"].grammar.len(),
            "a Mestre test must not consume the lessons"
        );
    }
}
