// SPDX-License-Identifier: Apache-2.0
//! Palette and `.px` text pixel-art format.
//!
//! ```text
//! # comment
//! size: 16x16        # frame size (required)
//! outline: K         # optional: auto-outline colour (palette letter)
//! repeat: 4x4        # optional: the grid is a WxH pattern tiled over `size`
//! bob: yes           # optional: add a second frame = first shifted down 1px (idle animation)
//! ................   # one row per line; '.' is transparent; letters come from palette.txt
//! ...
//! ---                # starts the next animation frame
//! ```

use crate::canvas::{Rgba, hex};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct Palette {
    colors: HashMap<char, Rgba>,
    order: Vec<char>,
}

impl Palette {
    /// `K = #1b1a2e  optional name` per line; `. = transparent`.
    pub fn parse(text: &str) -> Result<Palette, String> {
        let mut p = Palette::default();
        for (n, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (k, rest) = line
                .split_once('=')
                .ok_or(format!("palette line {}: expected `c = #rrggbb`", n + 1))?;
            let k = k.trim();
            let mut it = k.chars();
            let ch = it.next().ok_or("empty key")?;
            if it.next().is_some() {
                return Err(format!("palette line {}: key must be a single character", n + 1));
            }
            let val = rest.split_whitespace().next().unwrap_or("");
            let color = if val == "transparent" {
                [0, 0, 0, 0]
            } else {
                hex(val).ok_or(format!("palette line {}: bad colour `{val}`", n + 1))?
            };
            if p.colors.insert(ch, color).is_some() {
                return Err(format!("palette line {}: duplicate key `{ch}`", n + 1));
            }
            p.order.push(ch);
        }
        Ok(p)
    }

    pub fn get(&self, ch: char) -> Option<Rgba> {
        self.colors.get(&ch).copied()
    }

    /// Colour for a palette letter; magenta if missing (never panics in the renderer).
    pub fn rgb(&self, ch: char) -> Rgba {
        self.get(ch).unwrap_or([255, 0, 255, 255])
    }

    pub fn chars(&self) -> &[char] {
        &self.order
    }

    /// Number of opaque colours (the style guide says: at most 32).
    pub fn opaque_count(&self) -> usize {
        self.colors.values().filter(|c| c[3] != 0).count()
    }
}

#[derive(Debug, Clone)]
pub struct Sprite {
    pub w: usize,
    pub h: usize,
    pub frames: Vec<Vec<Rgba>>,
}

/// A parsed `.px` grid before colours are resolved (needed for role-based character templates).
#[derive(Debug, Clone)]
pub struct Grid {
    pub w: usize,
    pub h: usize,
    pub outline: Option<char>,
    pub bob: bool,
    pub frames: Vec<Vec<Vec<char>>>,
}

pub fn parse_grid(text: &str) -> Result<Grid, String> {
    let (mut w, mut h) = (0usize, 0usize);
    let mut outline = None;
    let mut repeat: Option<(usize, usize)> = None;
    let mut bob = false;
    let mut frames: Vec<Vec<Vec<char>>> = vec![];
    let mut cur: Vec<Vec<char>> = vec![];
    let parse_dim = |v: &str| -> Result<(usize, usize), String> {
        let (a, b) = v.split_once('x').ok_or(format!("bad size `{v}` (want WxH)"))?;
        Ok((
            a.trim().parse().map_err(|_| format!("bad size `{v}`"))?,
            b.trim().parse().map_err(|_| format!("bad size `{v}`"))?,
        ))
    };
    for (n, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim_end();
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "---" {
            frames.push(std::mem::take(&mut cur));
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            let (k, v) = (k.trim(), v.trim());
            match k {
                "size" => (w, h) = parse_dim(v)?,
                "outline" => outline = v.chars().next(),
                "repeat" => repeat = Some(parse_dim(v)?),
                "bob" => bob = matches!(v, "yes" | "true"),
                "name" => {}
                other => return Err(format!("line {}: unknown header `{other}`", n + 1)),
            }
            continue;
        }
        cur.push(line.chars().collect());
    }
    if !cur.is_empty() {
        frames.push(cur);
    }
    if w == 0 || h == 0 {
        return Err("missing `size: WxH`".into());
    }
    if frames.is_empty() {
        return Err("no pixel rows".into());
    }
    let (pw, ph) = repeat.unwrap_or((w, h));
    let mut out_frames = vec![];
    for (fi, f) in frames.iter().enumerate() {
        if f.len() != ph || f.iter().any(|r| r.len() != pw) {
            return Err(format!(
                "frame {fi}: expected {pw}x{ph} rows, got {}x{}",
                f.first().map(|r| r.len()).unwrap_or(0),
                f.len()
            ));
        }
        if repeat.is_some() {
            let mut full = vec![vec!['.'; w]; h];
            for (y, row) in full.iter_mut().enumerate() {
                for (x, px) in row.iter_mut().enumerate() {
                    *px = f[y % ph][x % pw];
                }
            }
            out_frames.push(full);
        } else {
            out_frames.push(f.clone());
        }
    }
    Ok(Grid {
        w,
        h,
        outline,
        bob,
        frames: out_frames,
    })
}

impl Grid {
    /// Resolve to colours. `roles` maps digit codes (character templates) to palette letters.
    pub fn render(&self, pal: &Palette, roles: &HashMap<char, char>) -> Result<Sprite, String> {
        let mut frames: Vec<Vec<Rgba>> = vec![];
        for f in &self.frames {
            let mut px: Vec<Rgba> = vec![[0; 4]; self.w * self.h];
            for (y, row) in f.iter().enumerate() {
                for (x, &ch) in row.iter().enumerate() {
                    let ch = roles.get(&ch).copied().unwrap_or(ch);
                    let c = pal
                        .get(ch)
                        .ok_or(format!("unknown palette letter `{ch}` at ({x},{y})"))?;
                    px[y * self.w + x] = c;
                }
            }
            if let Some(o) = self.outline {
                let oc = pal.get(o).ok_or(format!("unknown outline letter `{o}`"))?;
                let src = px.clone();
                for y in 0..self.h {
                    for x in 0..self.w {
                        if src[y * self.w + x][3] != 0 {
                            continue;
                        }
                        let near = [(0i32, 1i32), (0, -1), (1, 0), (-1, 0)].iter().any(|(dx, dy)| {
                            let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                            nx >= 0
                                && ny >= 0
                                && (nx as usize) < self.w
                                && (ny as usize) < self.h
                                && src[ny as usize * self.w + nx as usize][3] != 0
                                && src[ny as usize * self.w + nx as usize] != oc
                        });
                        if near {
                            px[y * self.w + x] = oc;
                        }
                    }
                }
            }
            frames.push(px);
        }
        if self.bob && frames.len() == 1 {
            let first = frames[0].clone();
            let mut second = vec![[0u8; 4]; self.w * self.h];
            for y in 1..self.h {
                for x in 0..self.w {
                    second[y * self.w + x] = first[(y - 1) * self.w + x];
                }
            }
            frames.push(second);
        }
        Ok(Sprite {
            w: self.w,
            h: self.h,
            frames,
        })
    }
}

pub fn parse_px(text: &str, pal: &Palette) -> Result<Sprite, String> {
    parse_grid(text)?.render(pal, &HashMap::new())
}

impl Sprite {
    /// Decode a PNG whose height is `n * frame_h` (frames stacked vertically).
    pub fn from_png(bytes: &[u8], frame_h: usize) -> Result<Sprite, String> {
        let dec = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut r = dec.read_info().map_err(|e| e.to_string())?;
        let mut buf = vec![0; r.output_buffer_size().ok_or("png too large")?];
        let info = r.next_frame(&mut buf).map_err(|e| e.to_string())?;
        let (w, h) = (info.width as usize, info.height as usize);
        let bpp = match info.color_type {
            png::ColorType::Rgba => 4,
            png::ColorType::Rgb => 3,
            other => return Err(format!("unsupported PNG colour type {other:?} (use RGBA)")),
        };
        if frame_h == 0 || h % frame_h != 0 {
            return Err(format!(
                "PNG height {h} is not a multiple of frame height {frame_h}"
            ));
        }
        let mut frames = vec![];
        for f in 0..h / frame_h {
            let mut px = vec![];
            for y in 0..frame_h {
                for x in 0..w {
                    let i = ((f * frame_h + y) * w + x) * bpp;
                    px.push(if bpp == 4 {
                        [buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]
                    } else {
                        [buf[i], buf[i + 1], buf[i + 2], 255]
                    });
                }
            }
            frames.push(px);
        }
        Ok(Sprite {
            w,
            h: frame_h,
            frames,
        })
    }

    pub fn flat_color(w: usize, h: usize, c: Rgba) -> Sprite {
        Sprite {
            w,
            h,
            frames: vec![vec![c; w * h]],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pal() -> Palette {
        Palette::parse("K = #000000\nW = #ffffff\n. = transparent\n").unwrap()
    }

    #[test]
    fn parses_frames_and_outline() {
        let t = "size: 4x3\noutline: K\n....\n.W..\n....\n---\n....\n..W.\n....\n";
        let s = parse_px(t, &pal()).unwrap();
        assert_eq!(s.frames.len(), 2);
        // pixel above the W is outlined
        assert_eq!(s.frames[0][1], [0, 0, 0, 255]);
        // the W itself is kept
        assert_eq!(s.frames[0][5], [255, 255, 255, 255]);
        // far corner stays transparent
        assert_eq!(s.frames[0][0][3], 0);
    }

    #[test]
    fn repeat_tiles_pattern() {
        let s = parse_px("size: 4x4\nrepeat: 2x2\nKW\nWK\n", &pal()).unwrap();
        assert_eq!(s.frames[0][0], s.frames[0][2]);
        assert_ne!(s.frames[0][0], s.frames[0][1]);
    }

    #[test]
    fn bob_adds_second_frame() {
        let s = parse_px("size: 2x2\nbob: yes\nW.\n..\n", &pal()).unwrap();
        assert_eq!(s.frames.len(), 2);
        assert_eq!(s.frames[1][2], [255, 255, 255, 255]);
    }

    #[test]
    fn errors_are_clear() {
        assert!(
            parse_px("size: 2x2\nWW\n", &pal())
                .unwrap_err()
                .contains("expected")
        );
        assert!(
            parse_px("size: 2x2\nQW\nWW\n", &pal())
                .unwrap_err()
                .contains("unknown palette letter")
        );
        assert!(parse_px("WW\n", &pal()).unwrap_err().contains("size"));
        assert!(Palette::parse("KK = #000000").is_err());
    }
}
