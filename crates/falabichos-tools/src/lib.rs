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

pub mod dashboard;
pub mod report;
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

fn diag(severity: Severity, location: &str, message: String) -> Diagnostic {
    Diagnostic {
        severity,
        location: location.to_string(),
        message,
    }
}

/// Characters of a UI string that are always on screen: the text without its `{placeholders}`.
fn visible_len(s: &str) -> usize {
    let mut n = 0;
    let mut in_ph = false;
    for c in s.chars() {
        match c {
            '{' => in_ph = true,
            '}' => in_ph = false,
            _ if !in_ph => n += 1,
            _ => {}
        }
    }
    n
}

/// How many characters of a UI string fit where the game draws it (6 px per character on a 240 px screen):
/// pause-menu items are narrow, settings rows share a line with their value, toasts, hints, captions and
/// calendar rows are one clipped 38-column line; everything else wraps over several lines.
fn display_budget(key: &str) -> usize {
    match key {
        "menu.time" | "menu.due" => 8,
        "settings.on" | "settings.off" | "settings.normal" | "settings.large" => 10,
        "lang.name" => 12,
        "calendar.nothing" | "party.empty" => 100,
        k if k.starts_with("menu.") => 11,
        k if k.starts_with("settings.") => 26,
        k if k.starts_with("toast.")
            || k.starts_with("ladder.")
            || k.starts_with("practice.")
            || k.starts_with("calendar.")
            || k.starts_with("battle.q.")
            || k.starts_with("title.")
            || k.contains("hint")
            || matches!(
                k,
                "battle.leave_keys" | "battle.listen_prompt" | "battle.dictation_prompt" | "battle.type_here"
            ) =>
        {
            38
        }
        _ => 100,
    }
}

/// `{name}`-style placeholders of a UI string.
pub fn placeholders(s: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut rest = s;
    while let Some(i) = rest.find('{') {
        rest = &rest[i + 1..];
        match rest.find('}') {
            Some(j) => {
                out.insert(rest[..j].to_string());
                rest = &rest[j + 1..];
            }
            None => break,
        }
    }
    out
}

/// Check one UI translation against the English table:
/// * missing keys (warning: the game falls back to English) and unknown keys (warning);
/// * placeholders that differ from the English string (**error**: the text would show `{x}` or lose a value);
/// * strings that are much longer than the English one (warning: the 240x160 screen clips them).
pub fn check_translation(
    path: &str,
    en: &std::collections::HashMap<String, String>,
    tbl: &std::collections::HashMap<String, String>,
) -> Vec<Diagnostic> {
    let mut d = vec![];
    let mut keys: Vec<&String> = en.keys().collect();
    keys.sort();
    for k in keys {
        let Some(v) = tbl.get(k) else {
            d.push(diag(
                Severity::Warning,
                path,
                format!("missing key `{k}` (falls back to English)"),
            ));
            continue;
        };
        let (pe, pt) = (placeholders(&en[k]), placeholders(v));
        if pe != pt {
            d.push(diag(
                Severity::Error,
                path,
                format!("key `{k}`: placeholders {pt:?} must be exactly the English ones {pe:?}"),
            ));
        }
        let (le, lt) = (visible_len(&en[k]), visible_len(v));
        let limit = display_budget(k).max(le);
        if lt > limit {
            d.push(diag(
                Severity::Warning,
                path,
                format!(
                    "key `{k}` is {lt} characters but about {limit} fit on screen here: it may be clipped"
                ),
            ));
        }
    }
    let mut unknown: Vec<&String> = tbl.keys().filter(|k| !en.contains_key(*k)).collect();
    unknown.sort();
    for k in unknown {
        d.push(diag(
            Severity::Warning,
            path,
            format!("unknown key `{k}` (not in English)"),
        ));
    }
    d
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

    // ---- audio clips: the slot of a vocab item is `audio` or `<category>/<id>`
    let mut with_audio = 0usize;
    for v in c.vocab.values() {
        if c.has_clip(&v.audio_slot()) {
            with_audio += 1;
        } else {
            rep.missing_audio.push(format!(
                "{}  (record to audio/pt-PT/{}.ogg)",
                v.id,
                v.audio_slot()
            ));
        }
    }
    let have_clips = c.audio_clips.len();
    let slots: BTreeSet<String> = c.vocab.values().map(|v| v.audio_slot()).collect();
    for clip in &c.audio_clips {
        if !slots.contains(clip) {
            rep.diagnostics.push(diag(
                Severity::Warning,
                &format!("audio/pt-PT/{clip}.ogg"),
                "clip does not match any vocab slot (<category>/<id>): misnamed?".into(),
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
        rep.diagnostics
            .extend(check_translation(&p, &en, &parse_table(t)));
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
    let _ = writeln!(
        s,
        "  verbs              {} ({} drill forms)",
        c.verbs.len(),
        c.conjugation_skills(None).len()
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
        "  audio clips        {have_clips} files, {with_audio}/{} vocab items have a recording",
        c.vocab.len()
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

    fn table(pairs: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn placeholders_are_extracted() {
        let p = placeholders("{a} and {b}, {a} again {unclosed");
        assert_eq!(
            p.into_iter().collect::<Vec<_>>(),
            vec!["a".to_string(), "b".to_string()]
        );
        assert!(placeholders("no braces").is_empty());
    }

    #[test]
    fn translation_check_reports_missing_unknown_placeholder_and_length_problems() {
        let en = table(&[
            ("a", "Hello {name}"),
            ("b", "Bye"),
            ("c", "Score {right}/{total}"),
            ("toast.d", "Short"),
        ]);
        let es = table(&[
            ("a", "Hola"),                                                       // dropped placeholder: error
            ("c", "Puntos {right}/{total}"),                                     // fine
            ("toast.d", "A very much longer sentence than the screen can hold"), // too long: warning
            ("zzz", "extra"),                                                    // unknown key: warning
        ]);
        let d = check_translation("i18n/es/ui.lang", &en, &es);
        let has =
            |sev: Severity, needle: &str| d.iter().any(|x| x.severity == sev && x.message.contains(needle));
        assert!(has(Severity::Error, "key `a`"), "{d:?}");
        assert!(has(Severity::Warning, "missing key `b`"), "{d:?}");
        assert!(has(Severity::Warning, "key `toast.d` is"), "{d:?}");
        // pause-menu items are narrow: an 11-character budget (English wins if it is longer)
        let en = table(&[("menu.save", "Save"), ("settings.music", "Music volume")]);
        let tr = table(&[
            ("menu.save", "Sauvegarder les données"),
            ("settings.music", "Volume de la musique"),
        ]);
        let d = check_translation("x", &en, &tr);
        assert!(d.iter().any(|x| x.message.contains("key `menu.save`")), "{d:?}");
        assert!(!d.iter().any(|x| x.message.contains("settings.music")), "{d:?}");
        // placeholders do not count as screen text
        let en = table(&[("a", "Error {error}")]);
        let tr = table(&[("a", "Un problème avec le fichier {error}")]);
        assert!(check_translation("x", &en, &tr).is_empty());
        assert!(has(Severity::Warning, "unknown key `zzz`"), "{d:?}");
        assert!(!d.iter().any(|x| x.message.contains("key `c`")), "{d:?}");
        // an extra placeholder is as bad as a missing one
        let bad = table(&[("a", "Hola {name} {extra}")]);
        let d = check_translation("x", &en, &bad);
        assert!(
            d.iter()
                .any(|x| x.severity == Severity::Error && x.message.contains("key `a`"))
        );
    }

    #[test]
    fn shipped_translations_are_complete_and_well_formed() {
        let assets = Assets::embedded();
        let en = parse_table(assets.text("i18n/en/ui.lang").unwrap());
        let langs: Vec<&str> = assets
            .list("i18n/")
            .filter_map(|p| p.strip_prefix("i18n/")?.strip_suffix("/ui.lang"))
            .collect();
        for needed in ["en", "es", "fr"] {
            assert!(langs.contains(&needed), "missing language {needed}");
        }
        for l in langs {
            let tbl = parse_table(assets.text(&format!("i18n/{l}/ui.lang")).unwrap());
            let d = check_translation(l, &en, &tbl);
            assert!(d.is_empty(), "{l}: {d:?}");
            assert!(tbl.contains_key("lang.name"), "{l} needs a lang.name");
        }
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
    fn scaffolded_files_load_and_lint_clean() {
        // Scaffold a vocab file, a grammar concept and a whole town into a copy of the real assets.
        let mut assets = Assets::embedded();
        for kind in ["vocab", "grammar", "town"] {
            for (path, body) in scaffold::files_for(kind, "demo").unwrap() {
                assets.insert(&path, body.into_bytes());
            }
        }
        let rep = lint(&assets);
        let errs: Vec<String> = rep
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .map(|d| d.to_string())
            .collect();
        assert!(
            errs.is_empty(),
            "scaffold templates must be valid out of the box:\n{}",
            errs.join("\n")
        );
        // A scaffolded Bicho points at placeholder vocab ids, so the linter must complain until it is edited.
        for (path, body) in scaffold::files_for("bicho", "demo").unwrap() {
            assets.insert(&path, body.into_bytes());
        }
        let rep = lint(&assets);
        assert!(
            rep.diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error && d.location == "species:demo")
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
