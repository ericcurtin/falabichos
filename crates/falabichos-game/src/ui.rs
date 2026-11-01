// SPDX-License-Identifier: Apache-2.0
//! Drawing helpers: themed panels, text, bars. All coordinates are in logical pixels (240x160).
// Drawing primitives naturally take a canvas plus x/y/w/h/colour arguments.
#![allow(clippy::too_many_arguments)]

use falabichos_media::font::{ADVANCE, LINE_HEIGHT};
use falabichos_media::{ArtBank, Canvas, Rgba};

pub const W: i32 = 240;
pub const H: i32 = 160;

#[derive(Debug, Clone, Copy)]
pub struct Colors {
    pub panel: Rgba,
    pub border: Rgba,
    pub text: Rgba,
    pub dim: Rgba,
    pub accent: Rgba,
    pub good: Rgba,
    pub bad: Rgba,
    pub highlight: Rgba,
    pub shade: Rgba,
    pub gloss_bg: Rgba,
}

impl Colors {
    pub fn new(bank: &ArtBank, high_contrast: bool) -> Colors {
        let p = |c| bank.pal(c);
        if high_contrast {
            Colors {
                panel: p('K'),
                border: p('W'),
                text: p('W'),
                dim: p('w'),
                accent: p('y'),
                good: p('l'),
                bad: p('R'),
                highlight: p('n'),
                shade: [0, 0, 0, 255],
                gloss_bg: p('K'),
            }
        } else {
            Colors {
                panel: p('W'),
                border: p('n'),
                text: p('K'),
                dim: p('G'),
                accent: p('b'),
                good: p('e'),
                bad: p('r'),
                highlight: p('Y'),
                shade: p('k'),
                gloss_bg: p('c'),
            }
        }
    }
}

pub struct Ui<'a> {
    pub bank: &'a ArtBank,
    pub col: Colors,
    /// Text scale for reading text (1 = normal, 2 = large).
    pub scale: i32,
    pub high_contrast: bool,
}

impl<'a> Ui<'a> {
    pub fn new(bank: &'a ArtBank, high_contrast: bool, large: bool) -> Ui<'a> {
        Ui {
            bank,
            col: Colors::new(bank, high_contrast),
            scale: if large { 2 } else { 1 },
            high_contrast,
        }
    }

    /// Rounded panel with a border.
    pub fn panel(&self, c: &mut Canvas, x: i32, y: i32, w: i32, h: i32) {
        c.rect_fill(x, y, w, h, self.col.panel);
        c.rect_outline(x, y, w, h, self.col.border);
        // inner border for a framed look
        if !self.high_contrast {
            c.rect_outline(x + 1, y + 1, w - 2, h - 2, self.bank.pal('B'));
        }
        // rounded corners
        for (cx, cy) in [(x, y), (x + w - 1, y), (x, y + h - 1), (x + w - 1, y + h - 1)] {
            c.set(cx, cy, [0, 0, 0, 0]);
        }
    }

    pub fn text(&self, c: &mut Canvas, x: i32, y: i32, s: &str, color: Rgba) -> i32 {
        self.bank.font.draw(c, s, x, y, color, 1)
    }

    pub fn text_scaled(&self, c: &mut Canvas, x: i32, y: i32, s: &str, color: Rgba, scale: i32) -> i32 {
        self.bank.font.draw(c, s, x, y, color, scale)
    }

    pub fn text_shadow(&self, c: &mut Canvas, x: i32, y: i32, s: &str, color: Rgba, scale: i32) {
        self.bank
            .font
            .draw_shadow(c, s, (x, y), (color, self.col.shade), scale);
    }

    pub fn text_center(&self, c: &mut Canvas, cx: i32, y: i32, s: &str, color: Rgba, scale: i32) {
        let w = self.bank.font.width(s, scale);
        self.bank.font.draw(c, s, cx - w / 2, y, color, scale);
    }

    /// Wrapped text; returns the number of lines drawn.
    pub fn text_wrap(
        &self,
        c: &mut Canvas,
        x: i32,
        y: i32,
        cols: usize,
        s: &str,
        color: Rgba,
        max_lines: usize,
    ) -> usize {
        let lines = self.bank.font.wrap(s, cols);
        let n = lines.len().min(max_lines);
        for (i, l) in lines.iter().take(n).enumerate() {
            self.text(c, x, y + i as i32 * LINE_HEIGHT, l, color);
        }
        n
    }

    pub fn glyph_cursor(&self, c: &mut Canvas, x: i32, y: i32) {
        self.text(c, x, y, "\u{25BA}", self.col.accent);
    }

    pub fn bar(&self, c: &mut Canvas, x: i32, y: i32, w: i32, h: i32, frac: f32, fill: Rgba) {
        c.rect_fill(x, y, w, h, self.col.shade);
        c.rect_fill(x + 1, y + 1, w - 2, h - 2, self.col.panel);
        let fw = (((w - 2) as f32) * frac.clamp(0.0, 1.0)).round() as i32;
        c.rect_fill(x + 1, y + 1, fw, h - 2, fill);
    }

    /// Columns that fit in `px` pixels at a text scale.
    pub fn cols(px: i32, scale: i32) -> usize {
        (px / (ADVANCE * scale)).max(1) as usize
    }
}

/// Truncate a string to `n` characters with an ellipsis (display only).
pub fn clip(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(n.saturating_sub(1)).collect();
        t.push('\u{2026}');
        t
    }
}
