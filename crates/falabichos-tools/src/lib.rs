// SPDX-License-Identifier: Apache-2.0
//! Linting logic shared by the `falabichos-lint` binary and tests.

use falabichos_assets::Assets;
use falabichos_content::{Diagnostic, Severity, load, review_items};
use falabichos_core::Content;
use falabichos_core::i18n::parse_table;
use falabichos_media::ArtBank;
use falabichos_media::synth::Song;
use std::collections::BTreeSet;
use std::fmt::Write as _;

pub mod scaffold;

#[derive(Default)]
pub struct LintReport {
    pub diagnostics: Vec<Diagnostic>,
    pub summary: String,
    pub review_list: Vec<(String, String)>,
    pub missing_audio: Vec<String>,
}

impl LintReport {
    pub fn count(&self, s: Severity) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == s).count()
    }
    pub fn errors(&self) -> usize {
        self.count(Severity::Error)
    }
}

/// Extract the contents of `"..."` literals from RON text, skipping `//` comments.
pub fn string_literals(text: &str) -> Vec<String> {
    let mut out = vec![];
    for line in text.lines() {
        let mut cur = String::new();
        let mut in_str = false;
        let mut esc = false;
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            if in_str {
                if esc {
                    cur.push(c);
                    esc = false;
                } else if c == '\\' {
                    esc = true;
                } else if c == '"' {
                    in_str = false;
                    out.push(std::mem::take(&mut cur));
                } else {
                    cur.push(c);
                }
            } else if c == '"' {
                in_str = true;
            } else if c == '/' && chars.peek() == Some(&'/') {
                break;
            }
        }
    }
    out
}

/// Displayed text of a `.lang` file: the values only, without comments or `{placeholders}`.
pub fn lang_values(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .filter_map(|l| l.split_once('='))
        .map(|(_, v)| {
            let mut out = String::new();
            let mut depth = 0;
            for ch in v.chars() {
                match ch {
                    '{' => depth += 1,
                    '}' if depth > 0 => depth -= 1,
                    _ if depth == 0 => out.push(ch),
                    _ => {}
                }
            }
            out
        })
        .collect()
}

pub fn lint(assets: &Assets) -> LintReport {
    let mut rep = LintReport::default();
    let loaded = load(assets);
    rep.diagnostics.extend(loaded.diagnostics.iter().cloned());
    let c: &Content = &loaded.content;
    let diag = |sev, loc: &str, msg: String| Diagnostic {
        severity: sev,
        location: loc.to_string(),
        message: msg,
    };

    // ---- art
    let bank = match ArtBank::load(assets) {
        Ok(b) => Some(b),
        Err(errs) => {
            for e in errs {
                rep.diagnostics.push(diag(Severity::Error, "art", e));
            }
            None
        }
    };
    if let Some(bank) = &bank {
        if bank.palette.opaque_count() > 32 {
            rep.diagnostics.push(diag(
                Severity::Error,
                "art/palette.txt",
                format!(
                    "{} opaque colours; the style guide allows 32",
                    bank.palette.opaque_count()
                ),
            ));
        }
        for t in &c.tiles {
            for a in &t.art {
                if bank.tile(a).is_none() {
                    rep.diagnostics.push(diag(
                        Severity::Error,
                        &format!("tile:{}", t.name),
                        format!("missing art `tiles/{a}`"),
                    ));
                }
            }
        }
        for s in c.species.values() {
            match bank.bicho(&s.art) {
                None => rep.diagnostics.push(diag(
                    Severity::Warning,
                    &format!("species:{}", s.id),
                    format!("missing art `bichos/{}` (a placeholder will be drawn)", s.art),
                )),
                Some(sp) if sp.w != 16 || sp.h != 16 => rep.diagnostics.push(diag(
                    Severity::Warning,
                    &format!("species:{}", s.id),
                    "Bicho sprites should be 16x16".into(),
                )),
                _ => {}
            }
        }
        for sl in c.seals.values() {
            if bank.seal(&sl.art).is_none() {
                rep.diagnostics.push(diag(
                    Severity::Warning,
                    &format!("seal:{}", sl.id),
                    format!("missing art `seals/{}`", sl.art),
                ));
            }
        }
        for m in c.maps.values() {
            for n in &m.npcs {
                if bank.character(&n.art).is_none() {
                    rep.diagnostics.push(diag(
                        Severity::Error,
                        &format!("map:{}", m.id),
                        format!(
                            "npc `{}`: missing character art `{}` (add it to art/chars.txt)",
                            n.id, n.art
                        ),
                    ));
                }
            }
        }
        for ch in c.challenges.values() {
            if bank.character(&ch.art).is_none() {
                rep.diagnostics.push(diag(
                    Severity::Error,
                    &format!("challenge:{}", ch.id),
                    format!("missing character art `{}`", ch.art),
                ));
            }
        }
        // font coverage of every string literal in content + i18n
        let mut missing: BTreeSet<(char, String)> = BTreeSet::new();
        for path in assets.list("content/").chain(assets.list("i18n/")) {
            // tile legends and map rows are layout data, not displayed text
            if path.ends_with("tiles.ron") || path.contains("/maps/") {
                continue;
            }
            if let Some(text) = assets.text(path) {
                let lits = if path.ends_with(".ron") {
                    string_literals(text)
                } else {
                    lang_values(text)
                };
                for l in lits {
                    for ch in l.chars() {
                        if !ch.is_whitespace() && !bank.font.covers(ch) {
                            missing.insert((ch, path.to_string()));
                        }
                    }
                }
            }
        }
        for (ch, path) in missing {
            rep.diagnostics.push(diag(
                Severity::Error,
                &path,
                format!(
                    "character `{ch}` (U+{:04X}) is not in the bitmap font (art/font.txt)",
                    ch as u32
                ),
            ));
        }
    }

    // ---- music
    let mut songs = 0;
    for p in assets.list("music/").filter(|p| p.ends_with(".song")) {
        songs += 1;
        match Song::parse(assets.text(p).unwrap_or("")) {
            Ok(_) => {}
            Err(e) => rep.diagnostics.push(diag(Severity::Error, p, e)),
        }
    }
    for m in c.maps.values() {
        if let Some(mu) = &m.music
            && !assets.contains(&format!("music/{mu}.song"))
        {
            rep.diagnostics.push(diag(
                Severity::Error,
                &format!("map:{}", m.id),
                format!("unknown music `{mu}`"),
            ));
        }
    }

    // ---- audio clips
    let mut with_audio = 0usize;
    for v in c.vocab.values() {
        if let Some(a) = &v.audio {
            with_audio += 1;
            if !assets.contains(&format!("audio/pt-PT/{a}.ogg")) {
                rep.missing_audio.push(v.id.clone());
            }
        } else {
            rep.missing_audio.push(v.id.clone());
        }
    }
    let have_clips = assets
        .list("audio/pt-PT/")
        .filter(|p| p.ends_with(".ogg"))
        .count();
    let referenced: BTreeSet<String> = c.vocab.values().filter_map(|v| v.audio.clone()).collect();
    for p in assets.list("audio/pt-PT/").filter(|p| p.ends_with(".ogg")) {
        let name = p.trim_start_matches("audio/pt-PT/").trim_end_matches(".ogg");
        if !referenced.contains(name) {
            rep.diagnostics.push(diag(
                Severity::Info,
                p,
                "clip is not referenced by any vocab item".into(),
            ));
        }
    }
    if !rep.missing_audio.is_empty() {
        rep.diagnostics.push(diag(
            Severity::Info,
            "audio",
            format!(
                "{} of {} vocab items have no recorded clip yet (see docs/AUDIO_GUIDE.md)",
                rep.missing_audio.len(),
                c.vocab.len()
            ),
        ));
    }

    // ---- i18n: every language must not have unknown keys; report missing keys vs English
    let en = assets
        .text("i18n/en/ui.lang")
        .map(parse_table)
        .unwrap_or_default();
    if en.is_empty() {
        rep.diagnostics.push(diag(
            Severity::Error,
            "i18n/en/ui.lang",
            "missing English UI strings".into(),
        ));
    }
    let langs: BTreeSet<String> = assets
        .list("i18n/")
        .filter_map(|p| p.split('/').nth(1))
        .map(String::from)
        .collect();
    for l in &langs {
        let p = format!("i18n/{l}/ui.lang");
        let Some(t) = assets.text(&p) else {
            rep.diagnostics.push(diag(
                Severity::Warning,
                &p,
                "language folder without ui.lang".into(),
            ));
            continue;
        };
        let tbl = parse_table(t);
        for k in en.keys() {
            if !tbl.contains_key(k) {
                rep.diagnostics.push(diag(
                    Severity::Warning,
                    &p,
                    format!("missing key `{k}` (falls back to English)"),
                ));
            }
        }
        for k in tbl.keys() {
            if !en.contains_key(k) {
                rep.diagnostics.push(diag(
                    Severity::Warning,
                    &p,
                    format!("unknown key `{k}` (not in English)"),
                ));
            }
        }
    }

    // ---- review
    for (kind, id) in review_items(c) {
        rep.review_list.push((kind.to_string(), id));
    }

    // ---- summary
    let exercises: usize = c.grammar.values().map(|g| g.exercises.len()).sum();
    let lines: usize = c
        .dialogues
        .values()
        .flat_map(|d| d.branches.iter())
        .map(|b| b.lines.len())
        .sum();
    let mut s = String::new();
    let _ = writeln!(s, "content summary");
    let _ = writeln!(s, "  vocab items        {}", c.vocab.len());
    let _ = writeln!(
        s,
        "  grammar concepts   {} ({} exercises)",
        c.grammar.len(),
        exercises
    );
    let _ = writeln!(s, "  bichos             {}", c.species.len());
    let _ = writeln!(s, "  towns / maps       {} / {}", c.towns.len(), c.maps.len());
    let _ = writeln!(s, "  dialogues / lines  {} / {}", c.dialogues.len(), lines);
    let _ = writeln!(
        s,
        "  challenges / seals {} / {}",
        c.challenges.len(),
        c.seals.len()
    );
    let _ = writeln!(s, "  songs              {songs}");
    let _ = writeln!(
        s,
        "  audio clips        {have_clips} files, {with_audio} vocab items reference a clip"
    );
    let _ = writeln!(s, "  needs native review {}", rep.review_list.len());
    rep.summary = s;
    rep.diagnostics
        .sort_by(|a, b| b.severity.cmp(&a.severity).then(a.location.cmp(&b.location)));
    rep
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lang_values_skip_comments_and_placeholders() {
        let v = lang_values("# <note>\na = Hello {name}!\nb = Bye");
        assert_eq!(v, vec![" Hello !".to_string(), " Bye".to_string()]);
    }

    #[test]
    fn string_literal_scanner() {
        let t = "(a: \"Olá, \\\"mundo\\\"\", // \"comment\"\n b: \"x\")";
        assert_eq!(
            string_literals(t),
            vec!["Olá, \"mundo\"".to_string(), "x".to_string()]
        );
    }

    #[test]
    fn shipped_assets_lint_clean() {
        let rep = lint(&Assets::embedded());
        let errs: Vec<String> = rep
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .map(|d| d.to_string())
            .collect();
        assert!(errs.is_empty(), "lint errors:\n{}", errs.join("\n"));
    }
}
