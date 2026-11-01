// SPDX-License-Identifier: Apache-2.0
//! Original 5x7 bitmap font (6x12 cell) covering Portuguese diacritics and the IPA symbols
//! needed for pt-PT. Glyph bitmaps live in `assets/art/font.txt`; accents are composed in code.
//!
//! Glyph space: 5 columns x 8 rows. Capitals use rows 0..=6, lowercase x-height rows 2..=6,
//! descenders row 7. Two rows above the glyph (`ACCENT_ROWS`) are reserved for capital accents.

use crate::canvas::{Canvas, Rgba};
use std::collections::HashMap;

pub const ADVANCE: i32 = 6;
pub const LINE_HEIGHT: i32 = 12;
const ACCENT_ROWS: i32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mark {
    Acute,
    Grave,
    Circumflex,
    Tilde,
    Diaeresis,
    Cedilla,
}

fn mark_rows(m: Mark) -> &'static [&'static str] {
    match m {
        Mark::Acute => &["...#.", "..#.."],
        Mark::Grave => &[".#...", "..#.."],
        Mark::Circumflex => &["..#..", ".#.#."],
        Mark::Tilde => &[".##.#", "#..#."],
        Mark::Diaeresis => &[".....", ".#.#."],
        Mark::Cedilla => &[".##.."],
    }
}

fn combining(c: char) -> Option<Mark> {
    Some(match c {
        '\u{0301}' => Mark::Acute,
        '\u{0300}' => Mark::Grave,
        '\u{0302}' => Mark::Circumflex,
        '\u{0303}' => Mark::Tilde,
        '\u{0308}' => Mark::Diaeresis,
        '\u{0327}' => Mark::Cedilla,
        _ => return None,
    })
}

fn decompose(c: char) -> (char, Option<Mark>) {
    use Mark::*;
    match c {
        'á' => ('a', Some(Acute)),
        'à' => ('a', Some(Grave)),
        'â' => ('a', Some(Circumflex)),
        'ã' => ('a', Some(Tilde)),
        'é' => ('e', Some(Acute)),
        'è' => ('e', Some(Grave)),
        'ê' => ('e', Some(Circumflex)),
        'í' => ('ı', Some(Acute)),
        'ì' => ('ı', Some(Grave)),
        'î' => ('ı', Some(Circumflex)),
        'ó' => ('o', Some(Acute)),
        'ò' => ('o', Some(Grave)),
        'ô' => ('o', Some(Circumflex)),
        'õ' => ('o', Some(Tilde)),
        'ẽ' => ('e', Some(Tilde)),
        'ĩ' => ('ı', Some(Tilde)),
        'ũ' => ('u', Some(Tilde)),
        'ñ' => ('n', Some(Tilde)),
        'ú' => ('u', Some(Acute)),
        'ù' => ('u', Some(Grave)),
        'û' => ('u', Some(Circumflex)),
        'ü' => ('u', Some(Diaeresis)),
        'ç' => ('c', Some(Cedilla)),
        'Á' => ('A', Some(Acute)),
        'À' => ('A', Some(Grave)),
        'Â' => ('A', Some(Circumflex)),
        'Ã' => ('A', Some(Tilde)),
        'É' => ('E', Some(Acute)),
        'Ê' => ('E', Some(Circumflex)),
        'Í' => ('I', Some(Acute)),
        'Ó' => ('O', Some(Acute)),
        'Ô' => ('O', Some(Circumflex)),
        'Õ' => ('O', Some(Tilde)),
        'Ú' => ('U', Some(Acute)),
        'Ç' => ('C', Some(Cedilla)),
        '“' | '”' | '«' | '»' => ('"', None),
        '‘' | '’' => ('\'', None),
        '–' | '—' => ('-', None),
        'ɡ' => ('g', None),
        '\u{a0}' => (' ', None),
        other => (other, None),
    }
}

#[derive(Debug, Clone, Default)]
pub struct Font {
    glyphs: HashMap<char, [u8; 8]>,
}

impl Font {
    pub fn parse(text: &str) -> Result<Font, String> {
        let mut glyphs = HashMap::new();
        for (n, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut it = line.split_whitespace();
            let key = it.next().ok_or("empty line")?;
            let ch = if let Some(hex) = key.strip_prefix("U+") {
                char::from_u32(u32::from_str_radix(hex, 16).map_err(|_| format!("line {}: bad code point", n + 1))?)
                    .ok_or(format!("line {}: bad code point", n + 1))?
            } else {
                let mut cs = key.chars();
                let c = cs.next().unwrap();
                if cs.next().is_some() {
                    return Err(format!("line {}: key `{key}` must be one character or U+XXXX", n + 1));
                }
                c
            };
            let rows: Vec<&str> = it.collect();
            if rows.len() != 8 {
                return Err(format!("line {}: glyph `{key}` needs 8 rows, has {}", n + 1, rows.len()));
            }
            let mut bits = [0u8; 8];
            for (r, row) in rows.iter().enumerate() {
                if row.chars().count() != 5 || row.chars().any(|c| c != '#' && c != '.') {
                    return Err(format!("line {}: glyph `{key}` row {r} must be 5 chars of # or .", n + 1));
                }
                for (x, c) in row.chars().enumerate() {
                    if c == '#' {
                        bits[r] |= 1 << (4 - x);
                    }
                }
            }
            if glyphs.insert(ch, bits).is_some() {
                return Err(format!("line {}: duplicate glyph `{key}`", n + 1));
            }
        }
        Ok(Font { glyphs })
    }

    /// Does the font cover this character (including composed accents)?
    pub fn covers(&self, c: char) -> bool {
        if combining(c).is_some() {
            return true;
        }
        let (b, _) = decompose(c);
        self.glyphs.contains_key(&b)
    }

    fn clusters(s: &str) -> Vec<(char, Option<Mark>)> {
        let mut out: Vec<(char, Option<Mark>)> = vec![];
        for c in s.chars() {
            if let (Some(m), Some(last)) = (combining(c), out.last_mut()) {
                if last.0 == 'i' {
                    last.0 = 'ı';
                }
                last.1 = Some(m);
                continue;
            }
            out.push(decompose(c));
        }
        out
    }

    /// Pixel width of `s` at `scale`.
    pub fn width(&self, s: &str, scale: i32) -> i32 {
        Self::clusters(s).len() as i32 * ADVANCE * scale
    }

    /// Draw `s` with its top-left at (`x`,`y`) (the accent zone is included in the cell).
    /// Returns the x position after the text.
    pub fn draw(&self, c: &mut Canvas, s: &str, x: i32, y: i32, color: Rgba, scale: i32) -> i32 {
        let mut cx = x;
        for (base, mark) in Self::clusters(s) {
            let bits = self.glyphs.get(&base).or_else(|| self.glyphs.get(&'?')).copied().unwrap_or([0; 8]);
            for (r, row) in bits.iter().enumerate() {
                for col in 0..5 {
                    if row & (1 << (4 - col)) != 0 {
                        c.rect_fill(cx + col * scale, y + (ACCENT_ROWS + r as i32) * scale, scale, scale, color);
                    }
                }
            }
            if let Some(m) = mark {
                let rows = mark_rows(m);
                let (top_row, below) = if m == Mark::Cedilla {
                    (ACCENT_ROWS + 7, true)
                } else if base.is_uppercase() {
                    (0, false)
                } else {
                    (ACCENT_ROWS, false)
                };
                let _ = below;
                for (i, row) in rows.iter().enumerate() {
                    for (col, ch) in row.chars().enumerate() {
                        if ch == '#' {
                            c.rect_fill(cx + col as i32 * scale, y + (top_row + i as i32) * scale, scale, scale, color);
                        }
                    }
                }
            }
            cx += ADVANCE * scale;
        }
        cx
    }

    /// Draw with a 1px drop shadow.
    pub fn draw_shadow(&self, c: &mut Canvas, s: &str, x: i32, y: i32, color: Rgba, shadow: Rgba, scale: i32) -> i32 {
        self.draw(c, s, x + scale, y + scale, shadow, scale);
        self.draw(c, s, x, y, color, scale)
    }

    /// Word-wrap to `cols` characters per line. Honours `\n`. Very long words are split.
    pub fn wrap(&self, s: &str, cols: usize) -> Vec<String> {
        let mut lines = vec![];
        for para in s.split('\n') {
            let mut line = String::new();
            let mut len = 0usize;
            for word in para.split_whitespace() {
                let wl = Self::clusters(word).len();
                if len > 0 && len + 1 + wl > cols {
                    lines.push(std::mem::take(&mut line));
                    len = 0;
                }
                if wl > cols {
                    // split overlong word
                    let cl: Vec<char> = word.chars().collect();
                    for chunk in cl.chunks(cols) {
                        if len > 0 {
                            lines.push(std::mem::take(&mut line));
                        }
                        line = chunk.iter().collect();
                        len = chunk.len();
                    }
                    continue;
                }
                if len > 0 {
                    line.push(' ');
                    len += 1;
                }
                line.push_str(word);
                len += wl;
            }
            lines.push(line);
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny() -> Font {
        Font::parse("A .###. #...# #...# ##### #...# #...# #...# .....\na ..... ..... .###. ....# .#### #...# .#### .....\nı ..... ..... .##.. ..#.. ..#.. ..#.. .###. .....\n? .###. #...# ....# ...#. ..#.. ..... ..#.. .....\nU+0020 ..... ..... ..... ..... ..... ..... ..... .....\n").unwrap()
    }

    #[test]
    fn parse_errors() {
        assert!(Font::parse("A .###.").is_err());
        assert!(Font::parse("AB .... .... .... .... .... .... .... ....").is_err());
    }

    #[test]
    fn composes_accents_and_draws() {
        let f = tiny();
        assert!(f.covers('á') && f.covers('í') && f.covers('Á'));
        assert!(!f.covers('z'));
        let mut c = Canvas::new(30, 14);
        let end = f.draw(&mut c, "á", 0, 0, [255, 255, 255, 255], 1);
        assert_eq!(end, 6);
        // acute accent pixel at accent row (row 2 for lowercase), col 3
        assert_eq!(c.get(3, 2), [255, 255, 255, 255]);
        // capital accent sits two rows higher
        let mut c2 = Canvas::new(30, 14);
        f.draw(&mut c2, "Á", 0, 0, [255, 255, 255, 255], 1);
        assert_eq!(c2.get(3, 0), [255, 255, 255, 255]);
    }

    #[test]
    fn combining_marks_attach() {
        let f = tiny();
        assert_eq!(f.width("a\u{0303}", 1), 6, "base + combining tilde is one cell");
    }

    #[test]
    fn wrapping() {
        let f = tiny();
        let l = f.wrap("aa aa aa\nA", 5);
        assert_eq!(l, vec!["aa aa", "aa", "A"]);
        let l = f.wrap("aaaaaaa", 3);
        assert_eq!(l, vec!["aaa", "aaa", "a"]);
    }
}
