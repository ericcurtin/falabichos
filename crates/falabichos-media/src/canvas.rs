// SPDX-License-Identifier: Apache-2.0
//! A tiny RGBA software canvas.

use crate::sprite::Sprite;

pub type Rgba = [u8; 4];

#[derive(Debug, Clone)]
pub struct Canvas {
    pub w: usize,
    pub h: usize,
    /// RGBA8, row-major.
    pub px: Vec<u8>,
}

impl Canvas {
    pub fn new(w: usize, h: usize) -> Canvas {
        Canvas {
            w,
            h,
            px: vec![0; w * h * 4],
        }
    }

    pub fn clear(&mut self, c: Rgba) {
        for p in self.px.chunks_exact_mut(4) {
            p.copy_from_slice(&c);
        }
    }

    #[inline]
    pub fn set(&mut self, x: i32, y: i32, c: Rgba) {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h || c[3] == 0 {
            return;
        }
        let i = (y as usize * self.w + x as usize) * 4;
        if c[3] == 255 {
            self.px[i..i + 4].copy_from_slice(&c);
        } else {
            // simple alpha blend
            let a = c[3] as u32;
            for (dst, src) in self.px[i..i + 3].iter_mut().zip(c.iter()) {
                *dst = ((*src as u32 * a + *dst as u32 * (255 - a)) / 255) as u8;
            }
            self.px[i + 3] = 255;
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Rgba {
        let i = (y * self.w + x) * 4;
        [self.px[i], self.px[i + 1], self.px[i + 2], self.px[i + 3]]
    }

    pub fn rect_fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgba) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.set(xx, yy, c);
            }
        }
    }

    pub fn rect_outline(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgba) {
        self.rect_fill(x, y, w, 1, c);
        self.rect_fill(x, y + h - 1, w, 1, c);
        self.rect_fill(x, y, 1, h, c);
        self.rect_fill(x + w - 1, y, 1, h, c);
    }

    /// Draw one frame of a sprite at integer scale. `flip` mirrors horizontally.
    pub fn blit(&mut self, s: &Sprite, frame: usize, x: i32, y: i32, scale: i32, flip: bool) {
        let f = &s.frames[frame % s.frames.len()];
        for sy in 0..s.h {
            for sx in 0..s.w {
                let c = f[sy * s.w + sx];
                if c[3] == 0 {
                    continue;
                }
                let dx = if flip { s.w - 1 - sx } else { sx } as i32;
                self.rect_fill(x + dx * scale, y + sy as i32 * scale, scale, scale, c);
            }
        }
    }

    /// Draw only the sub-rectangle (`sx`,`sy`,`w`,`h`) of a frame (for clipped tiles).
    pub fn blit_region(
        &mut self,
        s: &Sprite,
        frame: usize,
        region: (usize, usize, usize, usize),
        x: i32,
        y: i32,
    ) {
        let f = &s.frames[frame % s.frames.len()];
        let (rx, ry, rw, rh) = region;
        for sy in ry..(ry + rh).min(s.h) {
            for sx in rx..(rx + rw).min(s.w) {
                self.set(x + (sx - rx) as i32, y + (sy - ry) as i32, f[sy * s.w + sx]);
            }
        }
    }

    /// Darken/tint the whole canvas toward a colour (`amount` 0..=255), used for fades.
    pub fn fade_to(&mut self, c: Rgba, amount: u8) {
        let a = amount as u32;
        for p in self.px.chunks_exact_mut(4) {
            for (dst, src) in p[..3].iter_mut().zip(c.iter()) {
                *dst = ((*src as u32 * a + *dst as u32 * (255 - a)) / 255) as u8;
            }
        }
    }

    /// Encode as PNG (for screenshots).
    pub fn to_png(&self) -> Vec<u8> {
        let mut out = vec![];
        {
            let mut enc = png::Encoder::new(&mut out, self.w as u32, self.h as u32);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().expect("png header");
            w.write_image_data(&self.px).expect("png data");
        }
        out
    }

    /// Nearest-neighbour upscale (for screenshots that are easier to look at).
    pub fn upscaled(&self, k: usize) -> Canvas {
        let mut out = Canvas::new(self.w * k, self.h * k);
        for y in 0..out.h {
            for x in 0..out.w {
                let c = self.get(x / k, y / k);
                let i = (y * out.w + x) * 4;
                out.px[i..i + 4].copy_from_slice(&c);
            }
        }
        out
    }
}

pub fn hex(c: &str) -> Option<Rgba> {
    let c = c.strip_prefix('#')?;
    if c.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(c, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8, 255])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_clips_and_blends() {
        let mut c = Canvas::new(4, 4);
        c.set(-1, 0, [1, 2, 3, 255]);
        c.set(4, 0, [1, 2, 3, 255]);
        c.set(1, 1, [10, 20, 30, 255]);
        assert_eq!(c.get(1, 1), [10, 20, 30, 255]);
        c.set(1, 1, [110, 120, 130, 128]);
        let g = c.get(1, 1);
        assert!(g[0] > 10 && g[0] < 110);
    }

    #[test]
    fn png_roundtrip_header() {
        let c = Canvas::new(3, 2);
        let p = c.to_png();
        assert_eq!(&p[1..4], b"PNG");
    }

    #[test]
    fn hex_parse() {
        assert_eq!(hex("#ff0080"), Some([255, 0, 128, 255]));
        assert_eq!(hex("ff0080"), None);
    }
}
