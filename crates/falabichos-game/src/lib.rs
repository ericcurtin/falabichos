// SPDX-License-Identifier: Apache-2.0
//! The Falabichos game, independent of any engine.
//!
//! The platform shell (the Bevy client, a test, or the headless simulator) does four things:
//! 1. build a [`Game`] from [`Assets`] and a [`Storage`];
//! 2. call [`Game::tick`] 60 times a second with the current [`Input`];
//! 3. call [`Game::draw`] to render the 240x160 frame into a [`Canvas`];
//! 4. drain [`Game::take_events`] to play sounds/music, save, toggle fullscreen or quit.
//!
//! Because rendering is a software canvas, the whole game can run (and be screenshotted) with no
//! window and no GPU. See `docs/ARCHITECTURE.md`.

mod battle_ui;
mod dialogue;
pub mod input;
mod lesson;
mod menus;
pub mod sim;
pub mod storage;
pub mod touch_art;
pub mod ui;
mod world;

pub use input::{Input, Pad};
pub use storage::{FileStorage, MemoryStorage, Storage, default_data_dir};

use battle_ui::BattleScene;
use falabichos_assets::Assets;
use falabichos_core::i18n::Strings;
use falabichos_core::model::Dir;
use falabichos_core::save::{PlayerState, SaveGame, Settings, load as load_save_text};
use falabichos_core::{Content, Rng};
use falabichos_media::{ArtBank, Canvas};
use input::PadTracker;
use lesson::LessonState;
use menus::{CalendarState, DexState, MenuState, NotebookState, SealsState, SettingsState, TitleState};
use std::sync::Arc;
use world::World;

/// Logical resolution.
pub const LOGICAL_W: usize = 240;
pub const LOGICAL_H: usize = 160;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const SAVE_FILE: &str = "save.ron";
pub const SETTINGS_FILE: &str = "settings.ron";

/// Things the shell must react to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameEvent {
    /// Play a sound effect (`assets/music/<name>.song`).
    Sfx(String),
    /// Switch music (`None` = silence). Looping unless the song says `loop: no`.
    Music(Option<String>),
    /// Play a recorded pronunciation clip (`audio/pt-PT/<name>.ogg`) if the shell can.
    Clip(String),
    /// Play a Bicho cry at this pitch.
    Cry(u16),
    SettingsChanged,
    Fullscreen(bool),
    Saved,
    Quit,
}

#[derive(Debug, Clone)]
pub struct GameConfig {
    /// Show a "Quit" entry (desktop). Mobile platforms hide it.
    pub can_quit: bool,
    /// Show on-screen-control hints in text (mobile).
    pub touch: bool,
    /// RNG seed for new games (None = clock).
    pub seed: Option<u64>,
}

impl Default for GameConfig {
    fn default() -> Self {
        GameConfig {
            can_quit: true,
            touch: false,
            seed: None,
        }
    }
}

pub(crate) enum Scene {
    Title(TitleState),
    World,
    Menu(MenuState),
    Battle(Box<BattleScene>),
    Dex(DexState),
    Notebook(NotebookState),
    Seals(SealsState),
    Calendar(CalendarState),
    Lesson(Box<LessonState>),
    Settings(SettingsState),
}

pub struct Game {
    pub(crate) content: Arc<Content>,
    pub(crate) bank: Arc<ArtBank>,
    pub(crate) assets: Arc<Assets>,
    pub(crate) strings: Strings,
    pub settings: Settings,
    pub(crate) storage: Box<dyn Storage>,
    clock: Box<dyn Fn() -> i64 + Send + Sync>,
    pub(crate) scene: Scene,
    pub(crate) save: SaveGame,
    pub(crate) rng: Rng,
    pub(crate) world: World,
    events: Vec<GameEvent>,
    tracker: PadTracker,
    pub tick: u64,
    pub(crate) toast: Option<(String, i32)>,
    pub config: GameConfig,
    music_now: Option<String>,
}

fn system_clock() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl Game {
    /// Build the game. Fails with a readable message if content or art is invalid.
    pub fn new(assets: Arc<Assets>, storage: Box<dyn Storage>, config: GameConfig) -> Result<Game, String> {
        let content = falabichos_content::load_strict(&assets)
            .map_err(|errs| errs.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n"))?;
        let bank = ArtBank::load(&assets).map_err(|e| e.join("\n"))?;
        let settings = match storage.read(SETTINGS_FILE) {
            Some(t) => Settings::load(&t).unwrap_or_default(),
            None => Settings::default(),
        };
        let strings = Self::load_strings(&assets, &settings.ui_lang);
        let story = content.story.clone().ok_or("content has no story")?;
        let save = SaveGame::new(
            PlayerState {
                map: story.start_map.clone(),
                x: story.start_x,
                y: story.start_y,
                facing: Dir::Up,
            },
            system_clock(),
        );
        let content = Arc::new(content);
        let world = World::new(&content, &story.start_map, story.start_x, story.start_y, Dir::Up);
        let mut g = Game {
            content,
            bank: Arc::new(bank),
            assets,
            strings,
            settings,
            storage,
            clock: Box::new(system_clock),
            scene: Scene::World,
            rng: Rng::new(save.rng_state),
            save,
            world,
            events: vec![],
            tracker: PadTracker::default(),
            tick: 0,
            toast: None,
            config,
            music_now: None,
        };
        g.open_title();
        Ok(g)
    }

    fn load_strings(assets: &Assets, lang: &str) -> Strings {
        let en = assets.text("i18n/en/ui.lang").unwrap_or("");
        let l = assets.text(&format!("i18n/{lang}/ui.lang")).unwrap_or(en);
        Strings::new(l, en)
    }

    /// Replace the wall clock (tests and the simulator use a fake clock).
    pub fn set_clock(&mut self, f: impl Fn() -> i64 + Send + Sync + 'static) {
        self.clock = Box::new(f);
    }

    pub fn now(&self) -> i64 {
        (self.clock)()
    }

    pub fn content(&self) -> &Content {
        &self.content
    }

    pub fn art(&self) -> &ArtBank {
        &self.bank
    }

    /// Translate a UI string key.
    pub(crate) fn t(&self, key: &str) -> String {
        self.strings.get(key).to_string()
    }

    pub(crate) fn tf(&self, key: &str, args: &[(&str, &str)]) -> String {
        self.strings.fmt(key, args)
    }

    pub(crate) fn emit(&mut self, e: GameEvent) {
        self.events.push(e);
    }

    pub(crate) fn sfx(&mut self, name: &str) {
        self.events.push(GameEvent::Sfx(name.to_string()));
    }

    pub(crate) fn set_music(&mut self, name: Option<&str>) {
        let new = name.map(String::from);
        if new != self.music_now {
            self.music_now = new.clone();
            self.events.push(GameEvent::Music(new));
        }
    }

    pub(crate) fn toast(&mut self, text: String) {
        self.toast = Some((text, 150));
    }

    /// Drain pending events.
    pub fn take_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn has_save(&self) -> bool {
        self.storage.read(SAVE_FILE).is_some()
    }

    // ------------------------------------------------------------------ lifecycle

    pub(crate) fn open_title(&mut self) {
        let has_save = self.has_save();
        self.scene = Scene::Title(TitleState::new(has_save, self.config.can_quit));
        self.set_music(Some("title"));
    }

    /// Start a fresh game (overwrites nothing until the player saves).
    pub fn new_game(&mut self) {
        let story = self.content.story.clone().expect("story validated at load");
        let now = self.now();
        let seed = self.config.seed.unwrap_or(now as u64 ^ 0xFA1A_B1C0);
        self.save = SaveGame::new(
            PlayerState {
                map: story.start_map.clone(),
                x: story.start_x,
                y: story.start_y,
                facing: Dir::Up,
            },
            now,
        );
        self.save.rng_state = seed;
        self.rng = Rng::new(seed);
        self.world = World::new(
            &self.content,
            &story.start_map,
            story.start_x,
            story.start_y,
            Dir::Up,
        );
        self.scene = Scene::World;
        self.enter_current_map(true);
        self.start_dialogue(&story.intro_dialogue, None);
    }

    /// Load the saved game. Returns false (and keeps the title) if there is no usable save.
    pub fn continue_game(&mut self) -> bool {
        let text = self
            .storage
            .read(SAVE_FILE)
            .or_else(|| self.storage.read(&format!("{SAVE_FILE}.bak")));
        let Some(text) = text else { return false };
        match load_save_text(&text) {
            Ok(s) => {
                self.rng = Rng::new(s.rng_state);
                let p = s.player.clone();
                self.save = s;
                // Maps removed by a content update: fall back to the start of the story.
                let (map, x, y) = if self.content.maps.contains_key(&p.map) {
                    (p.map, p.x, p.y)
                } else {
                    let st = self.content.story.clone().expect("story");
                    (st.start_map, st.start_x, st.start_y)
                };
                self.world = World::new(&self.content, &map, x, y, p.facing);
                self.scene = Scene::World;
                self.enter_current_map(false);
                true
            }
            Err(e) => {
                self.toast(self.tf("save.corrupt", &[("error", &e.to_string())]));
                false
            }
        }
    }

    /// Write the save file. Returns whether it succeeded.
    pub fn save_game(&mut self) -> bool {
        self.save.version = falabichos_core::save::SAVE_VERSION;
        self.save.saved_at = self.now();
        self.save.rng_state = self.rng.state();
        self.save.player = PlayerState {
            map: self.world.map_id.clone(),
            x: self.world.x,
            y: self.world.y,
            facing: self.world.facing,
        };
        let ok = match self.save.to_ron() {
            Ok(text) => self.storage.write(SAVE_FILE, &text).is_ok(),
            Err(_) => false,
        };
        if ok {
            self.events.push(GameEvent::Saved);
        }
        ok
    }

    pub fn save_settings(&mut self) {
        if let Ok(t) = self.settings.to_ron() {
            let _ = self.storage.write(SETTINGS_FILE, &t);
        }
        self.strings = Self::load_strings(&self.assets, &self.settings.ui_lang);
        self.events.push(GameEvent::SettingsChanged);
    }

    /// The app is going to the background (mobile) or closing: persist progress if it is safe.
    pub fn on_suspend(&mut self) {
        self.save_settings();
        if matches!(self.scene, Scene::World | Scene::Menu(_)) && self.world.dialogue.is_none() {
            self.save_game();
        }
    }

    // ------------------------------------------------------------------ frame

    /// Advance the game by one 60 Hz tick.
    pub fn tick(&mut self, input: &Input) {
        let pad = self.tracker.update(input);
        self.tick += 1;
        if let Some((_, t)) = &mut self.toast {
            *t -= 1;
            if *t <= 0 {
                self.toast = None;
            }
        }
        if !matches!(self.scene, Scene::Title(_)) && self.tick.is_multiple_of(60) {
            self.save.play_secs += 1;
        }
        let scene = std::mem::replace(&mut self.scene, Scene::World);
        match scene {
            Scene::Title(s) => self.tick_title(s, &pad),
            Scene::World => self.tick_world(&pad),
            Scene::Menu(s) => self.tick_menu(s, &pad),
            Scene::Battle(b) => self.tick_battle(b, &pad),
            Scene::Dex(s) => self.tick_dex(s, &pad),
            Scene::Notebook(s) => self.tick_notebook(s, &pad),
            Scene::Seals(s) => self.tick_seals(s, &pad),
            Scene::Calendar(s) => self.tick_calendar(s, &pad),
            Scene::Lesson(s) => self.tick_lesson(s, &pad),
            Scene::Settings(s) => self.tick_settings(s, &pad),
        }
    }

    /// Render the current frame into a 240x160 canvas.
    pub fn draw(&self, c: &mut Canvas) {
        let ui = ui::Ui::new(&self.bank, self.settings.high_contrast, false);
        match &self.scene {
            Scene::Title(s) => self.draw_title(c, &ui, s),
            Scene::World => self.draw_world(c, &ui),
            Scene::Menu(s) => {
                self.draw_world(c, &ui);
                self.draw_menu(c, &ui, s);
            }
            Scene::Battle(b) => self.draw_battle(c, b),
            Scene::Dex(s) => self.draw_dex(c, &ui, s),
            Scene::Notebook(s) => self.draw_notebook(c, &ui, s),
            Scene::Seals(s) => self.draw_seals(c, &ui, s),
            Scene::Calendar(s) => self.draw_calendar(c, &ui, s),
            Scene::Lesson(s) => self.draw_lesson(c, &ui, s),
            Scene::Settings(s) => self.draw_settings(c, &ui, s),
        }
        if let Some((text, t)) = &self.toast {
            let w = (text.chars().count() as i32 * 6 + 12).min(232);
            let x = (240 - w) / 2;
            let y = if *t > 140 { 2 + (150 - *t) / 2 } else { 6 };
            ui.panel(c, x, y, w, 15);
            ui.text(c, x + 6, y + 2, &ui::clip(text, 38), ui.col.text);
        }
    }

    /// A one-line description of the current scene (tests and diagnostics).
    pub fn scene_name(&self) -> &'static str {
        match &self.scene {
            Scene::Title(_) => "title",
            Scene::World if self.world.dialogue.is_some() => "dialogue",
            Scene::World => "world",
            Scene::Menu(_) => "menu",
            Scene::Battle(_) => "battle",
            Scene::Dex(_) => "dex",
            Scene::Notebook(_) => "notebook",
            Scene::Seals(_) => "seals",
            Scene::Calendar(_) => "calendar",
            Scene::Lesson(_) => "lesson",
            Scene::Settings(_) => "settings",
        }
    }

    pub fn progress(&self) -> &falabichos_core::progress::Progress {
        &self.save.progress
    }

    pub fn save_state(&self) -> &SaveGame {
        &self.save
    }

    pub fn current_map(&self) -> &str {
        &self.world.map_id
    }

    pub fn player_pos(&self) -> (i32, i32) {
        (self.world.x, self.world.y)
    }
}

impl Game {
    /// Test/diagnostic helper: put the player on the first walkable tile of `map` and enter the world.
    #[doc(hidden)]
    pub fn debug_teleport(&mut self, map: &str) {
        let content = self.content.clone();
        let m = &content.maps[map];
        let mut spot = (1, 1);
        'outer: for (y, row) in m.rows.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                if content
                    .tile(ch)
                    .map(|t| matches!(t.kind, falabichos_core::model::TileKind::Floor))
                    .unwrap_or(false)
                {
                    spot = (x as i32, y as i32);
                    break 'outer;
                }
            }
        }
        self.debug_teleport_at(map, spot.0, spot.1);
    }

    /// Put the player at an exact tile of `map` (for tools, screenshots and tests).
    pub fn debug_teleport_at(&mut self, map: &str, x: i32, y: i32) {
        let content = self.content.clone();
        self.world = World::new(&content, map, x, y, Dir::Down);
        self.scene = Scene::World;
        self.enter_current_map(true);
    }
}

impl Game {
    /// The shell changed the window mode itself (F11); keep the setting in sync.
    pub fn set_fullscreen(&mut self, on: bool) {
        if self.settings.fullscreen != on {
            self.settings.fullscreen = on;
            self.save_settings();
        }
    }
}

#[cfg(test)]
mod tests {
    /// Every `t("key")` / `tf("key", ..)` used in the source must exist in the English strings.
    #[test]
    fn all_ui_string_keys_exist() {
        let en = falabichos_core::i18n::parse_table(include_str!("../../../assets/i18n/en/ui.lang"));
        let sources = [
            include_str!("lib.rs"),
            include_str!("world.rs"),
            include_str!("dialogue.rs"),
            include_str!("menus.rs"),
            include_str!("lesson.rs"),
            include_str!("battle_ui.rs"),
        ];
        let mut missing = vec![];
        for src in sources {
            for call in ["self.t(\"", "self.tf(\""] {
                let mut rest = src;
                while let Some(i) = rest.find(call) {
                    rest = &rest[i + call.len()..];
                    let key: String = rest.chars().take_while(|c| *c != '"').collect();
                    if !en.contains_key(&key) {
                        missing.push(key);
                    }
                }
            }
        }
        // keys picked from tables (`self.t(match ... "menu.x")`) are covered by the lint's unused-key check
        assert!(missing.is_empty(), "missing UI strings: {missing:?}");
    }
}
