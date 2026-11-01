// SPDX-License-Identifier: Apache-2.0
//! Loads every sprite, the palette and the font from `assets/art/`.
//!
//! Layout:
//! * `art/palette.txt`, `art/font.txt`
//! * `art/tiles/<id>.px`, `art/bichos/<id>.px`, `art/ui/<id>.px`, `art/seals/<id>.px`: sprites
//! * `art/chars/_body_<a|b>.px` + `art/chars.txt`: recolourable character sprites
//! * any of the above may be replaced by `<id>.png` (frames stacked vertically, square frames)
//!
//! Files whose name starts with `_` are templates, not sprites.

use crate::canvas::Rgba;
use crate::font::Font;
use crate::sprite::{Palette, Sprite, parse_grid};
use falabichos_assets::Assets;
use std::collections::{BTreeMap, HashMap};

pub struct ArtBank {
    pub palette: Palette,
    pub font: Font,
    sprites: BTreeMap<String, Sprite>,
}

/// Character role digits used in `_body_*.px` templates.
const ROLES: [(char, &str); 6] = [
    ('1', "hair"),
    ('2', "skin"),
    ('3', "shirt"),
    ('4', "pants"),
    ('5', "shoes"),
    ('6', "extra"),
];

impl ArtBank {
    pub fn load(assets: &Assets) -> Result<ArtBank, Vec<String>> {
        let mut errs = vec![];
        let palette = match assets.text("art/palette.txt").map(Palette::parse) {
            Some(Ok(p)) => p,
            Some(Err(e)) => {
                return Err(vec![format!("art/palette.txt: {e}")]);
            }
            None => return Err(vec!["art/palette.txt missing".into()]),
        };
        let font = match assets.text("art/font.txt").map(Font::parse) {
            Some(Ok(f)) => f,
            Some(Err(e)) => return Err(vec![format!("art/font.txt: {e}")]),
            None => return Err(vec!["art/font.txt missing".into()]),
        };
        let mut sprites: BTreeMap<String, Sprite> = BTreeMap::new();
        let paths: Vec<String> = assets.list("art/").map(String::from).collect();
        for path in &paths {
            let rel = &path["art/".len()..];
            let parts: Vec<&str> = rel.split('/').collect();
            let [cat, file] = parts.as_slice() else { continue };
            let Some((stem, ext)) = file.rsplit_once('.') else {
                continue;
            };
            if stem.starts_with('_') || !matches!(ext, "px" | "png") {
                continue;
            }
            let key = format!("{cat}/{stem}");
            if ext == "png" {
                match Sprite::from_png(assets.get(path).unwrap_or(&[]), 0) {
                    Ok(s) => {
                        sprites.insert(key, s);
                    }
                    Err(_) => {
                        // frame height = width (square frames); decode again with that height
                        let probe = Sprite::from_png(assets.get(path).unwrap_or(&[]), 1);
                        match probe {
                            Ok(p) => match Sprite::from_png(assets.get(path).unwrap_or(&[]), p.w) {
                                Ok(s) => {
                                    sprites.insert(key, s);
                                }
                                Err(e) => errs.push(format!("{path}: {e}")),
                            },
                            Err(e) => errs.push(format!("{path}: {e}")),
                        }
                    }
                }
                continue;
            }
            // .px: a PNG with the same stem wins
            if assets.contains(&format!("art/{cat}/{stem}.png")) {
                continue;
            }
            let text = assets.text(path).unwrap_or("");
            match parse_grid(text).and_then(|g| g.render(&palette, &HashMap::new())) {
                Ok(s) => {
                    sprites.insert(key, s);
                }
                Err(e) => errs.push(format!("{path}: {e}")),
            }
        }
        // characters
        if let Some(chars) = assets.text("art/chars.txt") {
            for (n, line) in chars.lines().enumerate() {
                let line = line.split('#').next().unwrap_or("").trim();
                if line.is_empty() {
                    continue;
                }
                let Some((id, rest)) = line.split_once(':') else {
                    errs.push(format!(
                        "art/chars.txt line {}: expected `id: role=letter ...`",
                        n + 1
                    ));
                    continue;
                };
                let mut roles: HashMap<char, char> = HashMap::new();
                let mut body = "a".to_string();
                for kv in rest.split_whitespace() {
                    let Some((k, v)) = kv.split_once('=') else {
                        errs.push(format!("art/chars.txt line {}: bad `{kv}`", n + 1));
                        continue;
                    };
                    if k == "body" {
                        body = v.to_string();
                    } else if let Some((digit, _)) = ROLES.iter().find(|(_, name)| *name == k) {
                        roles.insert(*digit, v.chars().next().unwrap_or('K'));
                    } else {
                        errs.push(format!("art/chars.txt line {}: unknown role `{k}`", n + 1));
                    }
                }
                let tpl_path = format!("art/chars/_body_{body}.px");
                let Some(tpl) = assets.text(&tpl_path) else {
                    errs.push(format!(
                        "art/chars.txt line {}: missing template {tpl_path}",
                        n + 1
                    ));
                    continue;
                };
                match parse_grid(tpl).and_then(|g| g.render(&palette, &roles)) {
                    Ok(s) => {
                        sprites.insert(format!("chars/{}", id.trim()), s);
                    }
                    Err(e) => errs.push(format!("{tpl_path} for `{}`: {e}", id.trim())),
                }
            }
        }
        if errs.is_empty() {
            Ok(ArtBank {
                palette,
                font,
                sprites,
            })
        } else {
            Err(errs)
        }
    }

    pub fn get(&self, cat: &str, id: &str) -> Option<&Sprite> {
        self.sprites.get(&format!("{cat}/{id}"))
    }
    pub fn tile(&self, id: &str) -> Option<&Sprite> {
        self.get("tiles", id)
    }
    pub fn bicho(&self, id: &str) -> Option<&Sprite> {
        self.get("bichos", id)
    }
    pub fn character(&self, id: &str) -> Option<&Sprite> {
        self.get("chars", id)
    }
    pub fn ui(&self, id: &str) -> Option<&Sprite> {
        self.get("ui", id)
    }
    pub fn seal(&self, id: &str) -> Option<&Sprite> {
        self.get("seals", id)
    }

    /// All sprite ids in a category.
    pub fn names(&self, cat: &str) -> Vec<&str> {
        let p = format!("{cat}/");
        self.sprites
            .keys()
            .filter_map(|k| k.strip_prefix(p.as_str()))
            .collect()
    }

    pub fn all(&self) -> impl Iterator<Item = (&str, &Sprite)> {
        self.sprites.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Palette colour by letter.
    pub fn pal(&self, ch: char) -> Rgba {
        self.palette.rgb(ch)
    }
}
