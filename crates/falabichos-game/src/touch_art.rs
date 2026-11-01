// SPDX-License-Identifier: Apache-2.0
//! On-screen touch controls: procedurally drawn artwork and hit-testing. The platform shell places
//! the images using [`TouchLayout`] and feeds touch positions to [`TouchLayout::read`].

use crate::input::Input;
use falabichos_media::{ArtBank, Canvas, Rgba};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
}

/// Safe-area insets in window pixels (notches, rounded corners, home indicator).
#[derive(Debug, Clone, Copy, Default)]
pub struct Insets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

/// Window-space rectangles of every touch control (origin top-left, y down).
#[derive(Debug, Clone, Copy)]
pub struct TouchLayout {
    pub dpad: Rect,
    pub a: Rect,
    pub b: Rect,
    pub menu: Rect,
    pub hint: Rect,
}

impl TouchLayout {
    /// Landscape-first layout: d-pad bottom-left, A/B bottom-right, MENU/HINT top-right.
    /// `dpi` is the window scale factor (1.0 on standard-density screens).
    pub fn compute(w: f32, h: f32, safe: Insets, dpi: f32) -> TouchLayout {
        let m = 14.0 * dpi;
        let unit = (h.min(w) * 0.30).clamp(90.0 * dpi.min(2.0), 220.0 * dpi.min(2.0));
        let dpad = Rect {
            x: safe.left + m,
            y: h - safe.bottom - m - unit,
            w: unit,
            h: unit,
        };
        let btn = unit * 0.45;
        let a = Rect {
            x: w - safe.right - m - btn,
            y: h - safe.bottom - m - unit * 0.62 - btn * 0.5,
            w: btn,
            h: btn,
        };
        let b = Rect {
            x: a.x - btn - m * 0.6,
            y: h - safe.bottom - m - btn,
            w: btn,
            h: btn,
        };
        let (pw, ph) = (btn * 1.25, btn * 0.5);
        let menu = Rect {
            x: w - safe.right - m - pw,
            y: safe.top + m * 0.6,
            w: pw,
            h: ph,
        };
        let hint = Rect {
            x: menu.x - pw - m * 0.6,
            y: menu.y,
            w: pw,
            h: ph,
        };
        TouchLayout {
            dpad,
            a,
            b,
            menu,
            hint,
        }
    }

    /// Translate the current touch points (window pixels) into held buttons.
    pub fn read(&self, touches: &[(f32, f32)]) -> Input {
        let mut i = Input::default();
        for &(x, y) in touches {
            if self.dpad.contains(x, y) || dist2(self.dpad.center(), (x, y)) < (self.dpad.w * 0.7).powi(2) {
                let (cx, cy) = self.dpad.center();
                let (dx, dy) = (x - cx, y - cy);
                if dx.abs().max(dy.abs()) > self.dpad.w * 0.12 {
                    if dx.abs() > dy.abs() * 0.8 {
                        if dx > 0.0 { i.right = true } else { i.left = true }
                    }
                    if dy.abs() > dx.abs() * 0.8 {
                        if dy > 0.0 { i.down = true } else { i.up = true }
                    }
                }
            } else if self.a.contains(x, y) {
                i.a = true;
            } else if self.b.contains(x, y) {
                i.b = true;
            } else if self.menu.contains(x, y) {
                i.start = true;
            } else if self.hint.contains(x, y) {
                i.select = true;
            }
        }
        i
    }
}

fn dist2(a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)
}

fn disc(c: &mut Canvas, cx: f32, cy: f32, r: f32, col: Rgba) {
    for y in 0..c.h {
        for x in 0..c.w {
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            if d <= r {
                c.set(x as i32, y as i32, col);
            }
        }
    }
}

/// The d-pad image (square, `size` px).
pub fn dpad_image(bank: &ArtBank, size: usize) -> Canvas {
    let mut c = Canvas::new(size, size);
    let s = size as f32;
    disc(&mut c, s / 2.0, s / 2.0, s / 2.0 - 1.0, [20, 20, 40, 110]);
    let arrow = [255, 248, 232, 190];
    for dir in 0..4 {
        // triangle pointing outwards
        for y in 0..size {
            for x in 0..size {
                let (fx, fy) = (x as f32 / s - 0.5, y as f32 / s - 0.5);
                let (u, v) = match dir {
                    0 => (fx, -fy),
                    1 => (fx, fy),
                    2 => (-fy, -fx),
                    _ => (-fy, fx),
                };
                // u across, v outward
                if v > 0.16 && v < 0.40 && u.abs() < (0.40 - v) * 0.9 {
                    c.set(x as i32, y as i32, arrow);
                }
            }
        }
    }
    let _ = bank;
    c
}

/// A round button with a letter (`A` / `B`).
pub fn round_button(bank: &ArtBank, size: usize, label: &str, fill: Rgba) -> Canvas {
    let mut c = Canvas::new(size, size);
    let s = size as f32;
    disc(&mut c, s / 2.0, s / 2.0, s / 2.0 - 1.0, fill);
    let scale = (size / 14).max(1) as i32;
    let w = bank.font.width(label, scale);
    bank.font.draw(
        &mut c,
        label,
        (size as i32 - w) / 2,
        (size as i32 - 10 * scale) / 2,
        [255, 248, 232, 240],
        scale,
    );
    c
}

/// A pill-shaped small button with a short label (`MENU`, `HINT`).
pub fn pill_button(bank: &ArtBank, w: usize, h: usize, label: &str) -> Canvas {
    let mut c = Canvas::new(w, h);
    let r = h as f32 / 2.0;
    for y in 0..h {
        for x in 0..w {
            let cx = (x as f32 + 0.5).clamp(r, w as f32 - r);
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - r).powi(2)).sqrt();
            if d <= r {
                c.set(x as i32, y as i32, [20, 20, 40, 130]);
            }
        }
    }
    let scale = (h / 22).max(1) as i32;
    let tw = bank.font.width(label, scale);
    bank.font.draw(
        &mut c,
        label,
        (w as i32 - tw) / 2,
        (h as i32 - 10 * scale) / 2,
        [255, 248, 232, 220],
        scale,
    );
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_places_controls_inside_the_window_and_respects_safe_area() {
        let safe = Insets {
            left: 44.0,
            top: 0.0,
            right: 44.0,
            bottom: 20.0,
        };
        let l = TouchLayout::compute(844.0, 390.0, safe, 3.0);
        for r in [l.dpad, l.a, l.b, l.menu, l.hint] {
            assert!(
                r.x >= 0.0 && r.y >= 0.0 && r.x + r.w <= 844.0 && r.y + r.h <= 390.0,
                "{r:?}"
            );
        }
        assert!(l.dpad.x >= 44.0, "d-pad clear of the notch");
        assert!(l.a.x + l.a.w <= 844.0 - 44.0);
        assert!(l.a.x >= l.b.x + l.b.w - 1.0, "A is to the right of B");
    }

    #[test]
    fn reading_touches() {
        let l = TouchLayout::compute(800.0, 400.0, Insets::default(), 1.0);
        let (cx, cy) = l.dpad.center();
        let up = l.read(&[(cx, cy - l.dpad.h * 0.4)]);
        assert!(up.up && !up.down && !up.left && !up.right);
        let right = l.read(&[(cx + l.dpad.w * 0.4, cy)]);
        assert!(right.right && !right.up);
        let (ax, ay) = l.a.center();
        assert!(l.read(&[(ax, ay)]).a);
        let (bx, by) = l.b.center();
        let both = l.read(&[(ax, ay), (bx, by), (cx - 40.0, cy)]);
        assert!(both.a && both.b && both.left, "multi-touch works");
        assert_eq!(l.read(&[]), Input::default());
        let (mx, my) = l.menu.center();
        assert!(l.read(&[(mx, my)]).start);
        let (hx, hy) = l.hint.center();
        assert!(l.read(&[(hx, hy)]).select);
    }
}
