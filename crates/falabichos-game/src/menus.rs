// SPDX-License-Identifier: Apache-2.0
//! Title screen, pause menu, Bichodex / party, grammar notebook, seals, settings.

use crate::input::Pad;
use crate::ui::{Ui, clip};
use crate::{Game, GameEvent, Scene};
use falabichos_core::model::SpeciesKind;
use falabichos_core::progress::{bicho_level, species_learned, species_mastery};
use falabichos_core::save::TextSize;
use falabichos_media::Canvas;

// ---------------------------------------------------------------------------------- title

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TitleItem {
    Continue,
    NewGame,
    Settings,
    Quit,
}

pub(crate) struct TitleState {
    pub items: Vec<TitleItem>,
    pub cursor: usize,
}

impl TitleState {
    pub fn new(has_save: bool, can_quit: bool) -> TitleState {
        let mut items = vec![];
        if has_save {
            items.push(TitleItem::Continue);
        }
        items.push(TitleItem::NewGame);
        items.push(TitleItem::Settings);
        if can_quit {
            items.push(TitleItem::Quit);
        }
        TitleState { items, cursor: 0 }
    }
}

fn move_cursor(cursor: &mut usize, n: usize, pad: &Pad) -> bool {
    if n == 0 {
        return false;
    }
    if pad.down {
        *cursor = (*cursor + 1) % n;
        return true;
    }
    if pad.up {
        *cursor = (*cursor + n - 1) % n;
        return true;
    }
    false
}

impl Game {
    pub(crate) fn tick_title(&mut self, mut s: TitleState, pad: &Pad) {
        if move_cursor(&mut s.cursor, s.items.len(), pad) {
            self.sfx("sfx_select");
        }
        if pad.a || pad.start {
            self.sfx("sfx_confirm");
            match s.items[s.cursor] {
                TitleItem::Continue => {
                    if !self.continue_game() {
                        self.scene = Scene::Title(s);
                    }
                    return;
                }
                TitleItem::NewGame => {
                    self.new_game();
                    return;
                }
                TitleItem::Settings => {
                    self.scene = Scene::Settings(SettingsState {
                        cursor: 0,
                        from_title: true,
                    });
                    return;
                }
                TitleItem::Quit => {
                    self.emit(GameEvent::Quit);
                }
            }
        }
        self.scene = Scene::Title(s);
    }

    pub(crate) fn draw_title(&self, c: &mut Canvas, ui: &Ui, s: &TitleState) {
        let b = &self.bank;
        // sky gradient bands
        let bands = ['n', 'n', 'b', 'b', 'B', 'B', 'c'];
        for (i, ch) in bands.iter().enumerate() {
            let y0 = i as i32 * 12;
            c.rect_fill(0, y0, 240, 12, b.pal(*ch));
        }
        if self.settings.high_contrast {
            c.rect_fill(0, 0, 240, 160, b.pal('K'));
        }
        // sea
        let wave = if self.settings.reduce_motion {
            0
        } else {
            (self.tick / 20) as i32
        };
        if !self.settings.high_contrast {
            c.rect_fill(0, 84, 240, 76, b.pal('s'));
            for row in 0..5 {
                for k in 0..16 {
                    let x = k * 16 + (row * 7 + wave * (1 + row % 2)) % 16;
                    c.rect_fill(x, 90 + row * 14, 6, 1, b.pal('c'));
                }
            }
            // sun
            c.rect_fill(206, 62, 14, 14, b.pal('y'));
            c.rect_fill(208, 60, 10, 18, b.pal('y'));
            c.rect_fill(204, 64, 18, 10, b.pal('y'));
        }
        // bouncing bichos
        let bob = if self.settings.reduce_motion {
            0
        } else {
            ((self.tick / 15) % 2) as usize
        };
        for (id, x, y) in [("galo", 14, 56), ("sardinha", 178, 62), ("pastel", 150, 40)] {
            if let Some(sp) = b.bicho(id) {
                c.blit(sp, bob, x, y, 3, id == "galo");
            }
        }
        // logo
        let title = "FALABICHOS";
        let tw = b.font.width(title, 3);
        let x = (240 - tw) / 2;
        ui.text_shadow(c, x + 2, 10, title, b.pal('Y'), 3);
        ui.text_center(c, 120, 46, &self.t("title.tagline"), b.pal('W'), 1);
        // menu
        ui.panel(c, 70, 92, 100, (s.items.len() as i32) * 14 + 10);
        for (i, it) in s.items.iter().enumerate() {
            let label = match it {
                TitleItem::Continue => self.t("title.continue"),
                TitleItem::NewGame => self.t("title.new"),
                TitleItem::Settings => self.t("title.settings"),
                TitleItem::Quit => self.t("title.quit"),
            };
            let y = 98 + i as i32 * 14;
            if i == s.cursor {
                ui.glyph_cursor(c, 78, y);
            }
            ui.text(c, 90, y, &label, ui.col.text);
        }
        ui.text(c, 4, 150, &format!("v{}", crate::VERSION), b.pal('W'));
        let hint = self.t(if self.config.touch {
            "title.hint_touch"
        } else {
            "title.hint"
        });
        ui.text(c, 236 - b.font.width(&hint, 1), 150, &hint, b.pal('W'));
    }
}

// ---------------------------------------------------------------------------------- pause menu

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuItem {
    Dex,
    Party,
    Practice,
    Notebook,
    Seals,
    Save,
    Settings,
    Title,
    Close,
}

pub(crate) struct MenuState {
    pub items: Vec<MenuItem>,
    pub cursor: usize,
}

impl MenuState {
    pub fn new(_title: &str) -> MenuState {
        MenuState {
            items: vec![
                MenuItem::Dex,
                MenuItem::Party,
                MenuItem::Practice,
                MenuItem::Notebook,
                MenuItem::Seals,
                MenuItem::Save,
                MenuItem::Settings,
                MenuItem::Title,
                MenuItem::Close,
            ],
            cursor: 0,
        }
    }
}

impl Game {
    fn menu_label(&self, it: MenuItem) -> String {
        self.t(match it {
            MenuItem::Dex => "menu.dex",
            MenuItem::Party => "menu.party",
            MenuItem::Practice => "menu.practice",
            MenuItem::Notebook => "menu.notebook",
            MenuItem::Seals => "menu.seals",
            MenuItem::Save => "menu.save",
            MenuItem::Settings => "menu.settings",
            MenuItem::Title => "menu.title_screen",
            MenuItem::Close => "menu.close",
        })
    }

    pub(crate) fn tick_menu(&mut self, mut s: MenuState, pad: &Pad) {
        if pad.b || pad.start {
            self.sfx("sfx_cancel");
            self.scene = Scene::World;
            return;
        }
        if move_cursor(&mut s.cursor, s.items.len(), pad) {
            self.sfx("sfx_select");
        }
        if pad.a {
            self.sfx("sfx_confirm");
            match s.items[s.cursor] {
                MenuItem::Dex => self.scene = Scene::Dex(self.dex_state(DexMode::Dex)),
                MenuItem::Party => self.scene = Scene::Dex(self.dex_state(DexMode::Party)),
                MenuItem::Practice => {
                    if self.save.progress.party.is_empty() {
                        let m = self.t("practice.need_bicho");
                        self.toast(m);
                        self.scene = Scene::Menu(s);
                    } else {
                        self.start_practice();
                    }
                }
                MenuItem::Notebook => {
                    self.scene = Scene::Notebook(NotebookState {
                        cursor: 0,
                        top: 0,
                        detail: None,
                    })
                }
                MenuItem::Seals => self.scene = Scene::Seals(SealsState { cursor: 0 }),
                MenuItem::Save => {
                    let ok = self.save_game();
                    let m = self.t(if ok { "toast.saved" } else { "toast.save_failed" });
                    self.sfx(if ok { "sfx_save" } else { "sfx_wrong" });
                    self.toast(m);
                    self.scene = Scene::Menu(s);
                }
                MenuItem::Settings => {
                    self.scene = Scene::Settings(SettingsState {
                        cursor: 0,
                        from_title: false,
                    })
                }
                MenuItem::Title => {
                    self.save_game();
                    self.open_title();
                }
                MenuItem::Close => self.scene = Scene::World,
            }
            return;
        }
        self.scene = Scene::Menu(s);
    }

    pub(crate) fn draw_menu(&self, c: &mut Canvas, ui: &Ui, s: &MenuState) {
        let h = s.items.len() as i32 * 13 + 10;
        ui.panel(c, 150, 4, 86, h);
        for (i, it) in s.items.iter().enumerate() {
            let y = 9 + i as i32 * 13;
            if i == s.cursor {
                ui.glyph_cursor(c, 155, y);
            }
            ui.text(c, 166, y, &self.menu_label(*it), ui.col.text);
        }
        // status
        let (seen, caught) = self.save.progress.dex_counts();
        ui.panel(c, 4, 4, 100, 42);
        ui.text(
            c,
            10,
            8,
            &format!("{} {}/{}", self.t("menu.dex"), caught, self.content.species.len()),
            ui.col.text,
        );
        ui.text(
            c,
            10,
            20,
            &format!("{} {}", self.t("menu.seals"), self.save.progress.seals.len()),
            ui.col.text,
        );
        let mins = self.save.play_secs / 60;
        ui.text(
            c,
            10,
            32,
            &format!("{} {}:{:02}", self.t("menu.time"), mins / 60, mins % 60),
            ui.col.dim,
        );
        let _ = seen;
    }
}

// ---------------------------------------------------------------------------------- dex / party

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DexMode {
    Dex,
    Party,
}

pub(crate) struct DexState {
    pub mode: DexMode,
    pub cursor: usize,
    pub top: usize,
    /// (page, word cursor)
    pub detail: Option<(u8, usize)>,
}

impl DexState {
    pub fn new(mode: DexMode) -> DexState {
        DexState {
            mode,
            cursor: 0,
            top: 0,
            detail: None,
        }
    }
}

impl Game {
    /// A fresh Bichodex view whose cursor starts on the first Bicho you have met (not on a `???` row).
    fn dex_state(&self, mode: DexMode) -> DexState {
        let mut s = DexState::new(mode);
        let p = &self.save.progress;
        if let Some(i) = self
            .dex_entries(mode)
            .iter()
            .position(|id| p.dex_seen.contains(id) || p.has_bicho(id))
        {
            s.cursor = i;
        }
        s
    }

    fn dex_entries(&self, mode: DexMode) -> Vec<String> {
        let mut v: Vec<(u16, String)> = self
            .content
            .species
            .values()
            .map(|s| (s.dex_no, s.id.clone()))
            .collect();
        v.sort();
        let p = &self.save.progress;
        v.into_iter()
            .map(|(_, id)| id)
            .filter(|id| match mode {
                DexMode::Dex => true,
                DexMode::Party => p.has_bicho(id),
            })
            .collect()
    }

    pub(crate) fn tick_dex(&mut self, mut s: DexState, pad: &Pad) {
        let entries = self.dex_entries(s.mode);
        if let Some((page, mut wc)) = s.detail {
            let id = entries.get(s.cursor).cloned().unwrap_or_default();
            let skills = self.content.species_skills(&id);
            if pad.b {
                s.detail = None;
                self.sfx("sfx_cancel");
            } else {
                let mut page = page;
                if pad.left || pad.right {
                    page = 1 - page;
                    self.sfx("sfx_select");
                }
                if page == 1 {
                    if pad.down && !skills.is_empty() {
                        wc = (wc + 1) % skills.len();
                        self.sfx("sfx_blip");
                    }
                    if pad.up && !skills.is_empty() {
                        wc = (wc + skills.len() - 1) % skills.len();
                        self.sfx("sfx_blip");
                    }
                }
                s.detail = Some((page, wc));
            }
            self.scene = Scene::Dex(s);
            return;
        }
        if pad.b || pad.start {
            self.sfx("sfx_cancel");
            self.scene = Scene::Menu(MenuState::new(""));
            return;
        }
        if move_cursor(&mut s.cursor, entries.len(), pad) {
            self.sfx("sfx_select");
        }
        if s.cursor < s.top {
            s.top = s.cursor;
        }
        if s.cursor >= s.top + 7 {
            s.top = s.cursor - 6;
        }
        if pad.a
            && let Some(id) = entries.get(s.cursor)
        {
            let p = &self.save.progress;
            if p.dex_seen.contains(id) || p.has_bicho(id) {
                s.detail = Some((0, 0));
                self.sfx("sfx_confirm");
                if let Some(sp) = self.content.species.get(id) {
                    self.emit(GameEvent::Cry(sp.cry_hz));
                }
            } else {
                self.sfx("sfx_bump");
            }
        }
        self.scene = Scene::Dex(s);
    }

    pub(crate) fn draw_dex(&self, c: &mut Canvas, ui: &Ui, s: &DexState) {
        c.clear(self.bank.pal(if self.settings.high_contrast { 'K' } else { 'n' }));
        let entries = self.dex_entries(s.mode);
        let (_, caught) = self.save.progress.dex_counts();
        let title = match s.mode {
            DexMode::Dex => format!(
                "{}  {}/{}",
                self.t("menu.dex"),
                caught,
                self.content.species.len()
            ),
            DexMode::Party => format!("{}  {}", self.t("menu.party"), self.save.progress.party.len()),
        };
        ui.text(c, 8, 6, &title, self.bank.pal('W'));
        if let Some((page, wc)) = s.detail {
            if let Some(id) = entries.get(s.cursor) {
                self.draw_dex_detail(c, ui, id, page, wc);
            }
            return;
        }
        ui.panel(c, 4, 20, 232, 120);
        if entries.is_empty() {
            ui.text_wrap(c, 12, 30, 36, &self.t("party.empty"), ui.col.text, 6);
        }
        for (row, id) in entries.iter().enumerate().skip(s.top).take(7) {
            let y = 26 + (row - s.top) as i32 * 16;
            let sp = &self.content.species[id];
            let p = &self.save.progress;
            let caught = p.has_bicho(id);
            let seen = p.dex_seen.contains(id) || caught;
            if row == s.cursor {
                c.rect_fill(8, y - 2, 224, 15, ui.col.highlight);
                ui.glyph_cursor(c, 10, y);
            }
            if seen && let Some(spr) = self.bank.bicho(&sp.art) {
                c.blit(spr, 0, 22, y - 2, 1, false);
            }
            let name = if seen { sp.name.clone() } else { "???".to_string() };
            ui.text(
                c,
                42,
                y,
                &format!("#{:02} {}", sp.dex_no, name),
                if caught { ui.col.text } else { ui.col.dim },
            );
            if caught {
                let lv = bicho_level(&self.content, &self.save.srs, id);
                ui.text(c, 170, y, &format!("Lv{lv}"), ui.col.accent);
                let m = species_mastery(&self.content, &self.save.srs, id);
                ui.bar(c, 196, y + 1, 32, 7, m, ui.col.good);
            } else if seen {
                ui.text(c, 170, y, &self.t("dex.seen"), ui.col.dim);
            }
        }
        ui.text(c, 8, 146, &self.t("dex.hint"), self.bank.pal('W'));
    }

    fn draw_dex_detail(&self, c: &mut Canvas, ui: &Ui, id: &str, page: u8, wc: usize) {
        let sp = &self.content.species[id];
        let caught = self.save.progress.has_bicho(id);
        ui.panel(c, 4, 20, 232, 126);
        if let Some(spr) = self.bank.bicho(&sp.art) {
            c.rect_fill(10, 26, 52, 52, ui.col.highlight);
            c.rect_outline(10, 26, 52, 52, ui.col.border);
            c.blit(spr, ((self.tick / 20) % 2) as usize, 12, 28, 3, false);
        }
        ui.text(c, 70, 26, &format!("#{:02} {}", sp.dex_no, sp.name), ui.col.text);
        let kind = match sp.kind {
            SpeciesKind::Vocab => self.t("dex.kind_vocab"),
            SpeciesKind::Grammar => self.t("dex.kind_grammar"),
        };
        ui.text(c, 70, 38, &kind, ui.col.accent);
        if caught {
            let lv = bicho_level(&self.content, &self.save.srs, id);
            let (learned, total) = species_learned(&self.content, &self.save.srs, id);
            let m = species_mastery(&self.content, &self.save.srs, id);
            ui.text(c, 70, 50, &format!("Lv {lv}"), ui.col.text);
            ui.bar(c, 108, 52, 100, 8, m, ui.col.good);
            ui.text(
                c,
                70,
                64,
                &self.tf(
                    "dex.learned",
                    &[("n", &learned.to_string()), ("total", &total.to_string())],
                ),
                ui.col.dim,
            );
        } else {
            ui.text(c, 70, 50, &self.t("dex.not_caught"), ui.col.dim);
        }
        if page == 0 {
            // Tight 10 px line pitch: a glyph cell is 10 px tall, so lines never collide.
            let wrap = |t: &str| ui.bank.font.wrap(t, 37);
            let mut y = 79;
            for (text, col) in [
                (&sp.theme, ui.col.text),
                (&sp.flavor_pt, ui.col.accent),
                (&sp.flavor_en, ui.col.dim),
            ] {
                for l in wrap(text).iter().take(2) {
                    ui.text(c, 10, y, l, col);
                    y += 10;
                }
                y += 2;
            }
        } else {
            self.draw_dex_words(c, ui, id, wc);
        }
        ui.text(
            c,
            8,
            149,
            &clip(
                &format!(
                    "{}  {}",
                    if page == 0 { "1/2" } else { "2/2" },
                    self.t("dex.detail_hint")
                ),
                38,
            ),
            self.bank.pal('W'),
        );
    }

    fn draw_dex_words(&self, c: &mut Canvas, ui: &Ui, id: &str, wc: usize) {
        let skills = self.content.species_skills(id);
        let top = wc.saturating_sub(3).min(skills.len().saturating_sub(5));
        for (row, sk) in skills.iter().enumerate().skip(top).take(5) {
            let y = 80 + (row - top) as i32 * 10;
            let label = if let Some(v) = self.content.vocab.get(sk) {
                format!("{} - {}", v.with_article(), v.primary_en())
            } else if let Some(e) = self.content.grammar_exercise(sk) {
                clip(&e.prompt, 34)
            } else {
                sk.clone()
            };
            let mastery = self.save.srs.mastery(sk);
            if row == wc {
                c.rect_fill(8, y - 1, 224, 10, ui.col.highlight);
            }
            let col = if self.save.srs.card(sk).is_some() {
                ui.col.text
            } else {
                ui.col.dim
            };
            ui.text(c, 12, y - 1, &clip(&label, 31), col);
            ui.bar(c, 212, y + 1, 18, 6, mastery, ui.col.good);
        }
        // details for the selected word
        if let Some(sk) = skills.get(wc) {
            if let Some(v) = self.content.vocab.get(sk) {
                let mut y = 133;
                if let Some(ipa) = &v.ipa {
                    ui.text(c, 12, y, &format!("[{ipa}]"), ui.col.accent);
                    y += 0;
                }
                if let Some(tag) = v.pron_tags.first().and_then(|t| self.content.pron_tip(*t)) {
                    let msg = clip(&tag.title, 22);
                    ui.text(c, 120, y, &msg, ui.col.dim);
                }
            } else if let Some(g) = self
                .content
                .grammar
                .values()
                .find(|g| g.exercises.iter().any(|e| &e.id == sk))
            {
                ui.text(c, 12, 133, &clip(&g.title, 36), ui.col.accent);
            }
        }
    }
}

// ---------------------------------------------------------------------------------- notebook

pub(crate) struct NotebookState {
    pub cursor: usize,
    pub top: usize,
    /// (concept index, scroll)
    pub detail: Option<(usize, usize)>,
}

impl Game {
    fn concepts(&self) -> Vec<&falabichos_core::model::GrammarConcept> {
        self.content.grammar.values().collect()
    }

    fn notebook_lines(&self, idx: usize) -> Vec<(String, u8)> {
        // (text, style) style: 0 normal, 1 heading, 2 dim, 3 accent
        let g = self.concepts()[idx];
        let f = &self.bank.font;
        let mut out: Vec<(String, u8)> = vec![];
        for l in f.wrap(&g.summary, 37) {
            out.push((l, 0));
        }
        out.push((String::new(), 0));
        for r in &g.rules {
            let w = f.wrap(r, 35);
            for (i, l) in w.iter().enumerate() {
                out.push((
                    if i == 0 {
                        format!("\u{25BA} {l}")
                    } else {
                        format!("  {l}")
                    },
                    0,
                ));
            }
        }
        out.push((String::new(), 0));
        out.push((self.t("notebook.examples"), 1));
        for e in &g.examples {
            for l in f.wrap(&e.pt, 37) {
                out.push((l, 3));
            }
            for l in f.wrap(&e.en, 37) {
                out.push((l, 2));
            }
        }
        if let Some(n) = &g.pt_pt_note {
            out.push((String::new(), 0));
            out.push((self.t("notebook.ptpt"), 1));
            for l in f.wrap(n, 37) {
                out.push((l, 0));
            }
        }
        out
    }

    pub(crate) fn tick_notebook(&mut self, mut s: NotebookState, pad: &Pad) {
        let n = self.concepts().len();
        if let Some((idx, mut scroll)) = s.detail {
            if pad.b {
                s.detail = None;
                self.sfx("sfx_cancel");
            } else {
                let total = self.notebook_lines(idx).len();
                if pad.down && scroll + 9 < total {
                    scroll += 1;
                }
                if pad.up && scroll > 0 {
                    scroll -= 1;
                }
                s.detail = Some((idx, scroll));
            }
            self.scene = Scene::Notebook(s);
            return;
        }
        if pad.b || pad.start {
            self.sfx("sfx_cancel");
            self.scene = Scene::Menu(MenuState::new(""));
            return;
        }
        if move_cursor(&mut s.cursor, n, pad) {
            self.sfx("sfx_select");
        }
        if s.cursor < s.top {
            s.top = s.cursor;
        }
        if s.cursor >= s.top + 8 {
            s.top = s.cursor - 7;
        }
        if pad.a {
            s.detail = Some((s.cursor, 0));
            self.sfx("sfx_confirm");
        }
        self.scene = Scene::Notebook(s);
    }

    pub(crate) fn draw_notebook(&self, c: &mut Canvas, ui: &Ui, s: &NotebookState) {
        c.clear(self.bank.pal(if self.settings.high_contrast { 'K' } else { 'n' }));
        ui.text(c, 8, 6, &self.t("menu.notebook"), self.bank.pal('W'));
        let concepts = self.concepts();
        if let Some((idx, scroll)) = s.detail {
            ui.panel(c, 4, 18, 232, 126);
            ui.text(c, 10, 21, &clip(&concepts[idx].title, 36), ui.col.accent);
            for (i, (l, style)) in self.notebook_lines(idx).iter().skip(scroll).take(9).enumerate() {
                let col = match style {
                    1 => ui.col.accent,
                    2 => ui.col.dim,
                    3 => ui.col.good,
                    _ => ui.col.text,
                };
                ui.text(c, 10, 34 + i as i32 * 11, l, col);
            }
            ui.text(c, 8, 148, &self.t("notebook.hint_detail"), self.bank.pal('W'));
            return;
        }
        ui.panel(c, 4, 20, 232, 122);
        for (row, g) in concepts.iter().enumerate().skip(s.top).take(8) {
            let y = 26 + (row - s.top) as i32 * 14;
            if row == s.cursor {
                c.rect_fill(8, y - 2, 224, 13, ui.col.highlight);
                ui.glyph_cursor(c, 10, y);
            }
            ui.text(c, 22, y, &clip(&g.title, 30), ui.col.text);
            let done: f32 = g
                .exercises
                .iter()
                .map(|e| self.save.srs.mastery(&e.id))
                .sum::<f32>()
                / g.exercises.len().max(1) as f32;
            ui.bar(c, 200, y + 1, 28, 7, done, ui.col.good);
        }
        ui.text(c, 8, 146, &self.t("notebook.hint"), self.bank.pal('W'));
    }
}

// ---------------------------------------------------------------------------------- seals

pub(crate) struct SealsState {
    pub cursor: usize,
}

const SEAL_SLOTS: usize = 8;

impl Game {
    pub(crate) fn tick_seals(&mut self, mut s: SealsState, pad: &Pad) {
        if pad.b || pad.start || pad.a {
            self.sfx("sfx_cancel");
            self.scene = Scene::Menu(MenuState::new(""));
            return;
        }
        if pad.right {
            s.cursor = (s.cursor + 1) % SEAL_SLOTS;
        }
        if pad.left {
            s.cursor = (s.cursor + SEAL_SLOTS - 1) % SEAL_SLOTS;
        }
        if pad.down {
            s.cursor = (s.cursor + 4) % SEAL_SLOTS;
        }
        if pad.up {
            s.cursor = (s.cursor + 4) % SEAL_SLOTS;
        }
        self.scene = Scene::Seals(s);
    }

    pub(crate) fn draw_seals(&self, c: &mut Canvas, ui: &Ui, s: &SealsState) {
        c.clear(self.bank.pal(if self.settings.high_contrast { 'K' } else { 'n' }));
        ui.text(c, 8, 6, &self.t("menu.seals"), self.bank.pal('W'));
        ui.panel(c, 4, 20, 232, 134);
        let seals: Vec<_> = self.content.seals.values().collect();
        for slot in 0..SEAL_SLOTS {
            let (x, y) = (14 + (slot % 4) as i32 * 56, 28 + (slot / 4) as i32 * 44);
            let seal = seals.get(slot);
            let earned = seal
                .map(|sl| self.save.progress.seals.contains(&sl.id))
                .unwrap_or(false);
            c.rect_fill(
                x,
                y,
                40,
                40,
                if slot == s.cursor {
                    ui.col.highlight
                } else {
                    ui.col.panel
                },
            );
            c.rect_outline(
                x,
                y,
                40,
                40,
                if slot == s.cursor {
                    ui.col.accent
                } else {
                    ui.col.dim
                },
            );
            match (seal, earned) {
                (Some(sl), true) => {
                    if let Some(spr) = self.bank.seal(&sl.art) {
                        c.blit(spr, 0, x + 4, y + 4, 2, false);
                    }
                }
                _ => ui.text_center(c, x + 20, y + 14, "?", ui.col.dim, 2),
            }
        }
        let info = match seals.get(s.cursor) {
            Some(sl) if self.save.progress.seals.contains(&sl.id) => {
                format!("{}: {}", sl.name, sl.description)
            }
            Some(sl) => format!("{}: {}", sl.name, self.t("seals.locked")),
            None => self.t("seals.coming"),
        };
        ui.text_wrap(c, 10, 118, 37, &info, ui.col.text, 3);
    }
}

// ---------------------------------------------------------------------------------- settings

pub(crate) struct SettingsState {
    pub cursor: usize,
    pub from_title: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingItem {
    TextSize,
    HighContrast,
    ReduceMotion,
    Music,
    Sfx,
    Gloss,
    Accents,
    Typos,
    Fullscreen,
    Back,
}

const SETTING_ITEMS: [SettingItem; 10] = [
    SettingItem::TextSize,
    SettingItem::HighContrast,
    SettingItem::ReduceMotion,
    SettingItem::Music,
    SettingItem::Sfx,
    SettingItem::Gloss,
    SettingItem::Accents,
    SettingItem::Typos,
    SettingItem::Fullscreen,
    SettingItem::Back,
];

impl Game {
    fn setting_label(&self, it: SettingItem) -> (String, String) {
        let onoff = |b: bool| self.t(if b { "settings.on" } else { "settings.off" });
        let s = &self.settings;
        match it {
            SettingItem::TextSize => (
                self.t("settings.text_size"),
                self.t(if s.text_size == TextSize::Large {
                    "settings.large"
                } else {
                    "settings.normal"
                }),
            ),
            SettingItem::HighContrast => (self.t("settings.high_contrast"), onoff(s.high_contrast)),
            SettingItem::ReduceMotion => (self.t("settings.reduce_motion"), onoff(s.reduce_motion)),
            SettingItem::Music => (self.t("settings.music"), format!("{}/10", s.music_volume)),
            SettingItem::Sfx => (self.t("settings.sfx"), format!("{}/10", s.sfx_volume)),
            SettingItem::Gloss => (self.t("settings.gloss"), onoff(s.gloss_default)),
            SettingItem::Accents => (self.t("settings.accents"), onoff(s.accent_insensitive)),
            SettingItem::Typos => (self.t("settings.typos"), onoff(s.typo_tolerance)),
            SettingItem::Fullscreen => (self.t("settings.fullscreen"), onoff(s.fullscreen)),
            SettingItem::Back => (self.t("settings.back"), String::new()),
        }
    }

    pub(crate) fn tick_settings(&mut self, mut s: SettingsState, pad: &Pad) {
        let leave = |g: &mut Game, s: &SettingsState| {
            g.save_settings();
            if s.from_title {
                g.open_title();
            } else {
                g.scene = Scene::Menu(MenuState::new(""));
            }
        };
        if pad.b || pad.start {
            self.sfx("sfx_cancel");
            leave(self, &s);
            return;
        }
        if move_cursor(&mut s.cursor, SETTING_ITEMS.len(), pad) {
            self.sfx("sfx_select");
        }
        let delta: i32 = if pad.right || pad.a {
            1
        } else if pad.left {
            -1
        } else {
            0
        };
        if delta != 0 {
            let it = SETTING_ITEMS[s.cursor];
            let st = &mut self.settings;
            let mut changed = true;
            match it {
                SettingItem::TextSize => {
                    st.text_size = if st.text_size == TextSize::Normal {
                        TextSize::Large
                    } else {
                        TextSize::Normal
                    }
                }
                SettingItem::HighContrast => st.high_contrast = !st.high_contrast,
                SettingItem::ReduceMotion => st.reduce_motion = !st.reduce_motion,
                SettingItem::Music => st.music_volume = (st.music_volume as i32 + delta).clamp(0, 10) as u8,
                SettingItem::Sfx => st.sfx_volume = (st.sfx_volume as i32 + delta).clamp(0, 10) as u8,
                SettingItem::Gloss => st.gloss_default = !st.gloss_default,
                SettingItem::Accents => st.accent_insensitive = !st.accent_insensitive,
                SettingItem::Typos => st.typo_tolerance = !st.typo_tolerance,
                SettingItem::Fullscreen => {
                    st.fullscreen = !st.fullscreen;
                    let f = st.fullscreen;
                    self.emit(GameEvent::Fullscreen(f));
                }
                SettingItem::Back => {
                    changed = false;
                    if pad.a {
                        self.sfx("sfx_cancel");
                        leave(self, &s);
                        return;
                    }
                }
            }
            if changed {
                self.sfx("sfx_select");
                self.events.push(GameEvent::SettingsChanged);
            }
        }
        self.scene = Scene::Settings(s);
    }

    pub(crate) fn draw_settings(&self, c: &mut Canvas, ui: &Ui, s: &SettingsState) {
        c.clear(self.bank.pal(if self.settings.high_contrast { 'K' } else { 'n' }));
        ui.text(c, 8, 6, &self.t("menu.settings"), self.bank.pal('W'));
        ui.panel(c, 4, 20, 232, 134);
        for (i, it) in SETTING_ITEMS.iter().enumerate() {
            let y = 25 + i as i32 * 12;
            let (label, value) = self.setting_label(*it);
            if i == s.cursor {
                c.rect_fill(8, y - 2, 224, 12, ui.col.highlight);
                ui.glyph_cursor(c, 10, y);
            }
            ui.text(c, 22, y, &label, ui.col.text);
            ui.text(c, 224 - ui.bank.font.width(&value, 1), y, &value, ui.col.accent);
        }
    }
}
