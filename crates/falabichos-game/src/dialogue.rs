// SPDX-License-Identifier: Apache-2.0
//! Dialogue boxes: typewriter text, English gloss toggle, word-by-word lookup.

use crate::Game;
use crate::input::Pad;
use crate::ui::Ui;
use falabichos_core::model::{Action, Line};
use falabichos_media::Canvas;
use falabichos_media::font::LINE_HEIGHT;

pub(crate) struct DialogueRun {
    pub lines: Vec<Line>,
    pub then: Vec<Action>,
    pub idx: usize,
    pub pages: Vec<Vec<String>>,
    pub page: usize,
    pub shown: usize,
    pub gloss: bool,
    pub word: usize,
    pub challenge_after: Option<String>,
    pub scale: i32,
}

/// Word tokens of a page: (line, start column, end column, text).
pub(crate) fn tokens(page: &[String]) -> Vec<(usize, usize, usize, String)> {
    let mut out = vec![];
    for (li, line) in page.iter().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == ' ' {
                i += 1;
                continue;
            }
            let start = i;
            while i < chars.len() && chars[i] != ' ' {
                i += 1;
            }
            out.push((li, start, i, chars[start..i].iter().collect()));
        }
    }
    out
}

fn paginate(game: &Game, text: &str, scale: i32) -> Vec<Vec<String>> {
    let cols = Ui::cols(216, scale);
    let rows = if scale > 1 { 2 } else { 3 };
    let lines = game.bank.font.wrap(text, cols);
    lines.chunks(rows).map(|c| c.to_vec()).collect()
}

impl Game {
    pub(crate) fn dialogue_scale(&self) -> i32 {
        if self.settings.text_size == falabichos_core::save::TextSize::Large {
            2
        } else {
            1
        }
    }

    /// Begin the dialogue with this id (the first matching branch). No-op if nothing matches.
    pub(crate) fn start_dialogue(&mut self, id: &str, challenge_after: Option<String>) {
        let Some(d) = self.content.dialogues.get(id) else {
            return;
        };
        let Some(branch) = d.resolve(&self.save.progress) else {
            return;
        };
        let scale = self.dialogue_scale();
        let mut run = DialogueRun {
            lines: branch.lines.clone(),
            then: branch.then.clone(),
            idx: 0,
            pages: vec![],
            page: 0,
            shown: 0,
            gloss: self.settings.gloss_default,
            word: 0,
            challenge_after,
            scale,
        };
        self.begin_line(&mut run);
        self.world.dialogue = Some(run);
        self.sfx("sfx_select");
    }

    fn begin_line(&mut self, d: &mut DialogueRun) {
        let line = &d.lines[d.idx];
        d.pages = paginate(self, &line.pt, d.scale);
        if d.pages.is_empty() {
            d.pages.push(vec![String::new()]);
        }
        d.page = 0;
        d.shown = 0;
        d.word = 0;
        if let Some(a) = &line.audio
            && self.assets.contains(&format!("audio/pt-PT/{a}.ogg"))
        {
            self.emit(crate::GameEvent::Clip(a.clone()));
        }
    }

    pub(crate) fn tick_dialogue(&mut self, pad: &Pad) {
        let Some(mut d) = self.world.dialogue.take() else {
            return;
        };
        if pad.select || pad.start {
            d.gloss = !d.gloss;
            d.word = 0;
            self.sfx("sfx_select");
        }
        let page = d.pages[d.page].clone();
        let total: usize =
            page.iter().map(|l| l.chars().count()).sum::<usize>() + page.len().saturating_sub(1);
        if d.shown < total {
            if pad.a || pad.b {
                d.shown = total;
            } else if self.tick.is_multiple_of(2) {
                d.shown += 1;
            }
            self.world.dialogue = Some(d);
            return;
        }
        if d.gloss {
            let n = tokens(&page).len();
            if n > 0 {
                if pad.right {
                    d.word = (d.word + 1) % n;
                    self.sfx("sfx_blip");
                }
                if pad.left {
                    d.word = (d.word + n - 1) % n;
                    self.sfx("sfx_blip");
                }
            }
        }
        if pad.a {
            if d.page + 1 < d.pages.len() {
                d.page += 1;
                d.shown = 0;
                d.word = 0;
                self.sfx("sfx_blip");
            } else if d.idx + 1 < d.lines.len() {
                d.idx += 1;
                self.begin_line(&mut d);
                self.sfx("sfx_blip");
            } else {
                self.finish_dialogue(d);
                return;
            }
        }
        self.world.dialogue = Some(d);
    }

    fn finish_dialogue(&mut self, d: DialogueRun) {
        let now = self.now();
        let fx = self.save.progress.apply(&d.then, now);
        self.run_effects(fx);
        if let Some(c) = d.challenge_after {
            self.begin_battle_transition(crate::world::PendingBattle::Challenge(c));
        }
    }

    pub(crate) fn draw_dialogue(&self, c: &mut Canvas, ui: &Ui, d: &DialogueRun) {
        let line = &d.lines[d.idx];
        let scale = d.scale;
        let (bx, by, bw, bh) = (4, 98, 232, 58);
        // speaker tab
        if let Some(who) = &line.who {
            let w = who.chars().count() as i32 * 6 + 12;
            ui.panel(c, bx + 6, by - 12, w, 15);
            ui.text(c, bx + 12, by - 10, who, ui.col.accent);
        }
        ui.panel(c, bx, by, bw, bh);
        let page = &d.pages[d.page];
        let mut left = d.shown;
        let pitch = LINE_HEIGHT * scale;
        let toks = tokens(page);
        for (i, l) in page.iter().enumerate() {
            let n = l.chars().count();
            let take = left.min(n);
            let shown: String = l.chars().take(take).collect();
            ui.text_scaled(c, bx + 8, by + 4 + i as i32 * pitch, &shown, ui.col.text, scale);
            left = left.saturating_sub(n + 1);
        }
        let total: usize =
            page.iter().map(|l| l.chars().count()).sum::<usize>() + page.len().saturating_sub(1);
        let complete = d.shown >= total;
        // gloss
        if d.gloss {
            let en = ui.bank.font.wrap(&line.en, 33);
            let h = (en.len().min(4) as i32 * LINE_HEIGHT + 24).max(36);
            c.rect_fill(bx, 2, bw, h, ui.col.gloss_bg);
            c.rect_outline(bx, 2, bw, h, ui.col.border);
            ui.text(c, bx + 6, 5, "EN", ui.col.accent);
            for (i, l) in en.iter().take(4).enumerate() {
                ui.text(c, bx + 24, 5 + i as i32 * LINE_HEIGHT, l, ui.col.text);
            }
            if complete && !toks.is_empty() {
                let sel = d.word.min(toks.len() - 1);
                let (gloss_text, span) = self.lookup_span(&toks, sel, &line.words);
                // underline the selected word(s)
                for (li, s, e, _) in toks.iter().skip(sel).take(span) {
                    let x = bx + 8 + *s as i32 * 6 * scale;
                    let y = by + 4 + *li as i32 * pitch + 10 * scale + 1;
                    c.rect_fill(x, y, (*e as i32 - *s as i32) * 6 * scale, 1, ui.col.accent);
                }
                let y = 2 + h - 13;
                let word: String = toks[sel..(sel + span).min(toks.len())]
                    .iter()
                    .map(|t| t.3.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                let word = word
                    .trim_matches(|ch: char| ".,!?;:\u{2026}\"".contains(ch))
                    .to_string();
                let msg = match gloss_text {
                    Some(g) => format!("{word} = {g}"),
                    None => format!("{word} = ?"),
                };
                ui.text(c, bx + 6, y, &crate::ui::clip(&msg, 38), ui.col.accent);
            } else if !complete {
                // nothing yet
            }
        }
        if complete && (self.tick / 20).is_multiple_of(2) {
            ui.text(c, bx + bw - 14, by + bh - 12, "\u{25BC}", ui.col.accent);
        }
        if complete {
            let hint = if d.gloss {
                self.t("dialogue.hint_gloss")
            } else {
                self.t("dialogue.hint_a")
            };
            ui.text(c, bx + 8, by + bh - 11, &hint, ui.col.dim);
        }
    }

    /// Look up the phrase starting at token `i`: try 3, 2 then 1 words. Returns (meaning, tokens used).
    fn lookup_span(
        &self,
        toks: &[(usize, usize, usize, String)],
        i: usize,
        hand: &std::collections::BTreeMap<String, String>,
    ) -> (Option<String>, usize) {
        for len in (1..=3).rev() {
            if i + len > toks.len() {
                continue;
            }
            let phrase: Vec<&str> = toks[i..i + len].iter().map(|t| t.3.as_str()).collect();
            let phrase = phrase.join(" ");
            if let Some(g) = self.content.lookup_word(&phrase, hand) {
                return (Some(g.en), len);
            }
        }
        (None, 1)
    }
}
