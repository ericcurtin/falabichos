// SPDX-License-Identifier: Apache-2.0
//! Cross-reference and sanity validation of a loaded [`Content`].

use super::{Diagnostic, Severity, diag};
use falabichos_core::Content;
use falabichos_core::model::*;
use std::collections::{BTreeSet, VecDeque};

/// Longest answer the typing screen accepts for a translation (see `battle_ui`).
const MAX_TYPED_ANSWER: usize = 60;

/// A word lower-cased with punctuation removed (accents kept: hints must match the spelling).
fn fold_word(w: &str) -> String {
    w.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_snake_id(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Is a tile walkable for the player?
pub fn walkable_kind(k: TileKind) -> bool {
    matches!(k, TileKind::Floor | TileKind::TallGrass)
}

pub fn walkable(c: &Content, m: &MapDef, x: i32, y: i32) -> bool {
    m.tile_char(x, y)
        .and_then(|ch| c.tile(ch))
        .map(|t| walkable_kind(t.kind))
        .unwrap_or(false)
}

pub fn validate(c: &Content) -> Vec<Diagnostic> {
    let mut d = vec![];
    let err = |loc: String, msg: String| diag(Severity::Error, loc, msg);
    let warn = |loc: String, msg: String| diag(Severity::Warning, loc, msg);

    // ---- tiles
    let mut chars = BTreeSet::new();
    for t in &c.tiles {
        if !chars.insert(t.ch) {
            d.push(err(
                format!("tile:{}", t.name),
                format!("duplicate tile character `{}`", t.ch),
            ));
        }
        if t.art.is_empty() {
            d.push(err(format!("tile:{}", t.name), "tile has no art".into()));
        }
    }

    // ---- vocab
    for v in c.vocab.values() {
        let loc = format!("vocab:{}", v.id);
        if !is_snake_id(&v.id) {
            d.push(err(loc.clone(), "id must be lowercase snake_case ASCII".into()));
        }
        if v.pt.trim().is_empty() {
            d.push(err(loc.clone(), "empty `pt`".into()));
        }
        if v.en.is_empty() || v.en.iter().any(|e| e.trim().is_empty()) {
            d.push(err(loc.clone(), "missing English translation (`en`)".into()));
        }
        if v.kind == WordKind::Noun && v.gender.is_none() {
            d.push(warn(loc.clone(), "noun without `gender`".into()));
        }
        if v.ipa.is_none() {
            d.push(warn(loc.clone(), "missing `ipa`".into()));
        }
        if !(1..=5).contains(&v.level) {
            d.push(err(loc.clone(), "`level` must be 1..=5".into()));
        }
        if let Some(a) = &v.audio
            && (a.contains("..") || a.starts_with('/') || a.ends_with(".ogg"))
        {
            d.push(err(
                loc.clone(),
                "`audio` must be a relative clip name without extension".into(),
            ));
        }
        if let Some(e) = &v.example
            && (e.pt.trim().is_empty() || e.en.trim().is_empty())
        {
            d.push(err(loc.clone(), "example needs both `pt` and `en`".into()));
        }
    }
    // Brazilian-form guard: flag headwords that are known pt-BR-only forms.
    const BR_ONLY: &[&str] = &[
        "ônibus",
        "celular",
        "café da manhã",
        "banheiro",
        "trem",
        "geladeira",
        "suco",
        "menina",
    ];
    for v in c.vocab.values() {
        if BR_ONLY.contains(&v.pt.to_lowercase().as_str()) {
            d.push(warn(
                format!("vocab:{}", v.id),
                format!("`{}` looks like a Brazilian-only form; use the pt-PT word", v.pt),
            ));
        }
    }

    // ---- grammar / exercises
    let mut ex_ids = BTreeSet::new();
    for g in c.grammar.values() {
        let loc = format!("grammar:{}", g.id);
        if g.exercises.len() < 3 {
            d.push(warn(loc.clone(), "fewer than 3 exercises".into()));
        }
        for e in &g.exercises {
            let eloc = format!("exercise:{}", e.id);
            if !is_snake_id(&e.id) {
                d.push(err(eloc.clone(), "id must be lowercase snake_case ASCII".into()));
            }
            if !ex_ids.insert(e.id.clone()) || c.vocab.contains_key(&e.id) {
                d.push(err(
                    eloc.clone(),
                    "exercise id collides with another exercise or vocab id".into(),
                ));
            }
            if e.answer.is_empty() {
                d.push(err(eloc.clone(), "no accepted `answer`".into()));
            }
            match e.kind {
                ExerciseKind::Choice | ExerciseKind::FillBlank => {
                    if e.options.len() < 2 {
                        d.push(err(eloc.clone(), "choice needs at least 2 options".into()));
                    }
                    for a in &e.answer {
                        if !e.options.iter().any(|o| o == a) {
                            d.push(err(
                                eloc.clone(),
                                format!("answer `{a}` is not among the options"),
                            ));
                        }
                    }
                    let mut u = e.options.clone();
                    u.sort();
                    u.dedup();
                    if u.len() != e.options.len() {
                        d.push(err(eloc.clone(), "duplicate options".into()));
                    }
                    if e.kind == ExerciseKind::FillBlank && !e.prompt.contains("___") {
                        d.push(err(eloc.clone(), "FillBlank prompt must contain `___`".into()));
                    }
                }
                ExerciseKind::Order => {
                    if e.answer.iter().any(|a| a.split_whitespace().count() < 2) {
                        d.push(err(eloc.clone(), "Order answers need at least 2 words".into()));
                    }
                }
                ExerciseKind::Type => {}
                ExerciseKind::Translate => {
                    if e.prompt_lang != Lang::En {
                        d.push(warn(
                            eloc.clone(),
                            "Translate prompts are English sentences: set `prompt_lang: En`".into(),
                        ));
                    }
                    if e.prompt.chars().count() > falabichos_core::exercise::TRANSLATE_PROMPT_CHARS {
                        d.push(warn(
                            eloc.clone(),
                            format!(
                                "prompt is longer than {} characters and will be clipped on screen",
                                falabichos_core::exercise::TRANSLATE_PROMPT_CHARS
                            ),
                        ));
                    }
                    if e.answer.iter().any(|a| a.chars().count() > MAX_TYPED_ANSWER) {
                        d.push(err(
                            eloc.clone(),
                            format!("an answer is longer than the {MAX_TYPED_ANSWER}-character typing limit"),
                        ));
                    }
                    // a hint for a word that is not in any accepted answer is almost surely a typo
                    let folded: Vec<Vec<String>> = e
                        .answer
                        .iter()
                        .map(|a| a.split_whitespace().map(fold_word).collect())
                        .collect();
                    for (en, pt) in &e.word_hints {
                        if en.trim().is_empty() || pt.trim().is_empty() {
                            d.push(err(eloc.clone(), "empty word hint".into()));
                            continue;
                        }
                        let in_answer = folded.iter().any(|ws| {
                            let joined = ws.join(" ");
                            joined
                                .contains(&pt.split_whitespace().map(fold_word).collect::<Vec<_>>().join(" "))
                        });
                        if !in_answer {
                            d.push(warn(
                                eloc.clone(),
                                format!("word hint `{en}={pt}` does not appear in any accepted answer"),
                            ));
                        }
                    }
                }
            }
            if e.kind != ExerciseKind::Translate && !e.word_hints.is_empty() {
                d.push(warn(
                    eloc.clone(),
                    "`word_hints` are only used by `Translate` exercises".into(),
                ));
            }
            if e.explanation.is_none() {
                d.push(diag(Severity::Info, eloc.clone(), "no explanation"));
            }
        }
    }

    // ---- verbs (conjugation tables)
    for v in c.verbs.values() {
        let loc = format!("verb:{}", v.id);
        if !is_snake_id(&v.id) {
            d.push(err(loc.clone(), "id must be lowercase snake_case ASCII".into()));
        }
        if v.inf.trim().is_empty() || v.en.trim().is_empty() {
            d.push(err(loc.clone(), "needs `inf` and `en`".into()));
        }
        if !matches!(v.group.as_str(), "ar" | "er" | "ir" | "irregular") {
            d.push(err(
                loc.clone(),
                format!("`group` must be ar, er, ir or irregular, not `{}`", v.group),
            ));
        }
        for t in Tense::ALL {
            let forms = v.forms(t);
            if forms.len() != PERSONS.len() {
                d.push(err(
                    loc.clone(),
                    format!(
                        "{} needs {} forms ({}), found {}",
                        t.pt(),
                        PERSONS.len(),
                        PERSONS.join(", "),
                        forms.len()
                    ),
                ));
                continue;
            }
            if forms
                .iter()
                .any(|f| f.trim().is_empty() || f.contains(char::is_whitespace))
            {
                d.push(err(loc.clone(), format!("{}: every form is one word", t.pt())));
            }
            let mut uniq: Vec<&String> = forms.iter().collect();
            uniq.sort();
            uniq.dedup();
            if uniq.len() < 3 {
                d.push(warn(
                    loc.clone(),
                    format!("{}: suspiciously few distinct forms", t.pt()),
                ));
            }
            // regular verbs must follow their group's pattern
            if v.group != "irregular" && t == Tense::Perfeito {
                let stem = v.inf.trim_end_matches(&v.group);
                if forms.iter().any(|f| !f.starts_with(stem)) {
                    d.push(warn(
                        loc.clone(),
                        "a regular form does not start with the verb stem".into(),
                    ));
                }
            }
        }
        if v.id != falabichos_core::normalize::fold(&v.inf) {
            d.push(warn(
                loc.clone(),
                "id should be the infinitive without accents".into(),
            ));
        }
    }

    // ---- species
    let mut dex = BTreeSet::new();
    for s in c.species.values() {
        let loc = format!("species:{}", s.id);
        if !dex.insert(s.dex_no) {
            d.push(err(loc.clone(), format!("duplicate dex_no {}", s.dex_no)));
        }
        for v in &s.vocab {
            if !c.vocab.contains_key(v) {
                d.push(err(loc.clone(), format!("unknown vocab id `{v}`")));
            }
        }
        for g in &s.grammar {
            if !c.grammar.contains_key(g) {
                d.push(err(loc.clone(), format!("unknown grammar id `{g}`")));
            }
        }
        match s.kind {
            SpeciesKind::Vocab if s.vocab.len() < 3 => {
                d.push(err(loc.clone(), "needs at least 3 vocab items".into()))
            }
            SpeciesKind::Grammar if s.grammar.is_empty() => {
                d.push(err(loc.clone(), "grammar Bicho needs a grammar concept".into()))
            }
            _ => {}
        }
        if s.trust_needed == 0 {
            d.push(err(loc.clone(), "trust_needed must be >= 1".into()));
        }
    }

    // ---- seals
    for s in c.seals.values() {
        if s.art.is_empty() {
            d.push(err(format!("seal:{}", s.id), "no art".into()));
        }
    }

    // ---- towns
    for t in c.towns.values() {
        if !c.maps.contains_key(&t.start_map) {
            d.push(err(
                format!("town:{}", t.id),
                format!("unknown start_map `{}`", t.start_map),
            ));
        }
    }

    // ---- dialogues
    let check_cond = |loc: &str, cond: &Cond, d: &mut Vec<Diagnostic>| match cond {
        Cond::HasBicho(s) if !c.species.contains_key(s) => {
            d.push(err(loc.into(), format!("unknown species `{s}` in condition")))
        }
        Cond::HasSeal(s) if !c.seals.contains_key(s) => {
            d.push(err(loc.into(), format!("unknown seal `{s}` in condition")))
        }
        _ => {}
    };
    for dlg in c.dialogues.values() {
        let loc = format!("dialogue:{}", dlg.id);
        if dlg.branches.is_empty() {
            d.push(err(loc.clone(), "no branches".into()));
        }
        if dlg.branches.last().map(|b| !b.when.is_empty()).unwrap_or(false) {
            d.push(warn(
                loc.clone(),
                "last branch has conditions; nothing plays if none hold".into(),
            ));
        }
        for b in &dlg.branches {
            if b.lines.is_empty() {
                d.push(err(loc.clone(), "branch with no lines".into()));
            }
            for cond in &b.when {
                check_cond(&loc, cond, &mut d);
            }
            for l in &b.lines {
                if l.pt.trim().is_empty() {
                    d.push(err(loc.clone(), "empty Portuguese line".into()));
                }
                if l.en.trim().is_empty() {
                    d.push(err(loc.clone(), format!("line `{}` has no English gloss", l.pt)));
                }
            }
            for a in &b.then {
                match a {
                    Action::Challenge(ch) if !c.challenges.contains_key(ch) => {
                        d.push(err(loc.clone(), format!("unknown challenge `{ch}`")))
                    }
                    Action::GiveBicho(s) if !c.species.contains_key(s) => {
                        d.push(err(loc.clone(), format!("unknown species `{s}`")))
                    }
                    Action::Warp { map, x, y } => match c.maps.get(map) {
                        None => d.push(err(loc.clone(), format!("warp to unknown map `{map}`"))),
                        Some(m) if !walkable(c, m, *x, *y) => d.push(err(
                            loc.clone(),
                            format!("warp target {map}({x},{y}) is not walkable"),
                        )),
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }

    // ---- challenges
    for ch in c.challenges.values() {
        let loc = format!("challenge:{}", ch.id);
        let pool = c.pool_skills(&ch.pool);
        if pool.is_empty() {
            d.push(err(loc.clone(), "empty exercise pool".into()));
        }
        if ch.pass_correct > ch.questions {
            d.push(err(loc.clone(), "pass_correct exceeds questions".into()));
        }
        if ch.questions == 0 || ch.confidence == 0 {
            d.push(err(loc.clone(), "questions and confidence must be >= 1".into()));
        }
        for cat in &ch.pool.categories {
            if !c.vocab.values().any(|v| &v.category == cat) {
                d.push(err(loc.clone(), format!("unknown vocab category `{cat}`")));
            }
        }
        for v in &ch.pool.vocab {
            if !c.vocab.contains_key(v) {
                d.push(err(loc.clone(), format!("unknown vocab `{v}`")));
            }
        }
        for g in &ch.pool.grammar {
            if !c.grammar.contains_key(g) {
                d.push(err(loc.clone(), format!("unknown grammar `{g}`")));
            }
        }
        if let Some(s) = &ch.seal
            && !c.seals.contains_key(s)
        {
            d.push(err(loc.clone(), format!("unknown seal `{s}`")));
        }
        if ch.kind == ChallengeKind::Mestre && ch.seal.is_none() {
            d.push(warn(loc.clone(), "Mestre without a seal".into()));
        }
        for dl in [&ch.win_dialogue, &ch.lose_dialogue].into_iter().flatten() {
            if !c.dialogues.contains_key(dl) {
                d.push(err(loc.clone(), format!("unknown dialogue `{dl}`")));
            }
        }
    }

    // ---- maps
    for m in c.maps.values() {
        validate_map(c, m, &mut d);
    }

    // ---- story
    match &c.story {
        None => d.push(err("story.ron".into(), "missing story.ron".into())),
        Some(s) => {
            match c.maps.get(&s.start_map) {
                None => d.push(err(
                    "story.ron".into(),
                    format!("unknown start_map `{}`", s.start_map),
                )),
                Some(m) if !walkable(c, m, s.start_x, s.start_y) => {
                    d.push(err("story.ron".into(), "start position is not walkable".into()))
                }
                _ => {}
            }
            for dl in [&s.intro_dialogue, &s.finale_dialogue] {
                if !c.dialogues.contains_key(dl) {
                    d.push(err("story.ron".into(), format!("unknown dialogue `{dl}`")));
                }
            }
        }
    }
    d
}

fn validate_map(c: &Content, m: &MapDef, d: &mut Vec<Diagnostic>) {
    let loc = format!("map:{}", m.id);
    let e = |msg: String| diag(Severity::Error, loc.clone(), msg);
    let w = |msg: String| diag(Severity::Warning, loc.clone(), msg);
    if m.rows.is_empty() {
        d.push(e("no rows".into()));
        return;
    }
    let width = m.width();
    for (y, r) in m.rows.iter().enumerate() {
        if r.chars().count() != width {
            d.push(e(format!(
                "row {y} has width {} but row 0 has {width}",
                r.chars().count()
            )));
        }
        for (x, ch) in r.chars().enumerate() {
            if c.tile(ch).is_none() {
                d.push(e(format!("unknown tile character `{ch}` at ({x},{y})")));
            }
        }
    }
    let in_bounds = |x: i32, y: i32| x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < m.height();
    let mut occupied: BTreeSet<(i32, i32)> = BTreeSet::new();
    for wp in &m.warps {
        if !in_bounds(wp.x, wp.y) {
            d.push(e(format!("warp at ({},{}) is out of bounds", wp.x, wp.y)));
        }
        match c.maps.get(&wp.map) {
            None => d.push(e(format!("warp to unknown map `{}`", wp.map))),
            Some(t) => {
                if !walkable(c, t, wp.to_x, wp.to_y) {
                    d.push(e(format!(
                        "warp arrival {}({},{}) is not walkable",
                        wp.map, wp.to_x, wp.to_y
                    )));
                }
            }
        }
    }
    for n in &m.npcs {
        let nl = format!("npc `{}`", n.id);
        if !in_bounds(n.x, n.y) {
            d.push(e(format!("{nl} out of bounds")));
            continue;
        }
        if !walkable(c, m, n.x, n.y) {
            d.push(e(format!("{nl} stands on a non-walkable tile ({},{})", n.x, n.y)));
        }
        if !occupied.insert((n.x, n.y)) {
            d.push(e(format!("{nl} overlaps another NPC at ({},{})", n.x, n.y)));
        }
        if let Some(dl) = &n.dialogue
            && !c.dialogues.contains_key(dl)
        {
            d.push(e(format!("{nl}: unknown dialogue `{dl}`")));
        }
        if let Some(t) = &n.trainer {
            if !c.challenges.contains_key(&t.challenge) {
                d.push(e(format!("{nl}: unknown challenge `{}`", t.challenge)));
            }
            if !c.dialogues.contains_key(&t.spot_dialogue) {
                d.push(e(format!("{nl}: unknown dialogue `{}`", t.spot_dialogue)));
            }
        }
        for cond in &n.show_when {
            match cond {
                Cond::HasBicho(s) if !c.species.contains_key(s) => {
                    d.push(e(format!("{nl}: unknown species `{s}`")))
                }
                Cond::HasSeal(s) if !c.seals.contains_key(s) => {
                    d.push(e(format!("{nl}: unknown seal `{s}`")))
                }
                _ => {}
            }
        }
    }
    for s in &m.signs {
        if !in_bounds(s.x, s.y) {
            d.push(e(format!("sign at ({},{}) out of bounds", s.x, s.y)));
        }
        if !c.dialogues.contains_key(&s.dialogue) {
            d.push(e(format!("sign: unknown dialogue `{}`", s.dialogue)));
        }
    }
    for t in &m.triggers {
        if !in_bounds(t.x, t.y) {
            d.push(e(format!("trigger at ({},{}) out of bounds", t.x, t.y)));
        }
        if !c.dialogues.contains_key(&t.dialogue) {
            d.push(e(format!("trigger: unknown dialogue `{}`", t.dialogue)));
        }
    }
    if !m.encounters.is_empty() {
        let has_grass = m
            .rows
            .iter()
            .flat_map(|r| r.chars())
            .any(|ch| c.tile(ch).map(|t| t.kind == TileKind::TallGrass).unwrap_or(false));
        if !has_grass {
            d.push(w("has encounters but no tall-grass tiles".into()));
        }
        for en in &m.encounters {
            if !c.species.contains_key(&en.species) {
                d.push(e(format!("encounter: unknown species `{}`", en.species)));
            }
            if en.weight == 0 {
                d.push(e("encounter weight 0".into()));
            }
        }
    }
    if m.encounters.is_empty() {
        let has_grass = m
            .rows
            .iter()
            .flat_map(|r| r.chars())
            .any(|ch| c.tile(ch).map(|t| t.kind == TileKind::TallGrass).unwrap_or(false));
        if has_grass {
            d.push(w("has tall-grass tiles but no encounters".into()));
        }
    }

    // ---- reachability: every warp tile and NPC must be reachable from some entry point.
    let mut entries: Vec<(i32, i32)> = vec![];
    for other in c.maps.values() {
        for wp in &other.warps {
            if wp.map == m.id {
                entries.push((wp.to_x, wp.to_y));
            }
        }
    }
    if let Some(s) = &c.story
        && s.start_map == m.id
    {
        entries.push((s.start_x, s.start_y));
    }
    if entries.is_empty() {
        d.push(diag(
            Severity::Info,
            loc.clone(),
            "map has no entry point (nothing warps here)",
        ));
        return;
    }
    let mut seen: BTreeSet<(i32, i32)> = BTreeSet::new();
    let mut q: VecDeque<(i32, i32)> = VecDeque::new();
    for &p in &entries {
        if walkable(c, m, p.0, p.1) && seen.insert(p) {
            q.push_back(p);
        }
    }
    let npc_blocks: BTreeSet<(i32, i32)> = m
        .npcs
        .iter()
        .filter(|n| n.show_when.is_empty())
        .map(|n| (n.x, n.y))
        .collect();
    while let Some((x, y)) = q.pop_front() {
        for (dx, dy) in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
            let p = (x + dx, y + dy);
            if walkable(c, m, p.0, p.1) && !npc_blocks.contains(&p) && seen.insert(p) {
                q.push_back(p);
            }
        }
    }
    for wp in &m.warps {
        if walkable(c, m, wp.x, wp.y) && !seen.contains(&(wp.x, wp.y)) {
            d.push(e(format!(
                "warp at ({},{}) is unreachable from the map entry points",
                wp.x, wp.y
            )));
        }
    }
    for n in &m.npcs {
        let near = [(0, 1), (0, -1), (1, 0), (-1, 0)]
            .iter()
            .any(|(dx, dy)| seen.contains(&(n.x + dx, n.y + dy)));
        // Counters: NPCs behind a counter are talked to across it, so check one more tile out.
        let across = [(0, 2), (0, -2), (2, 0), (-2, 0)]
            .iter()
            .any(|(dx, dy)| seen.contains(&(n.x + dx, n.y + dy)));
        if !near && !across && n.show_when.is_empty() {
            d.push(w(format!("npc `{}` has no reachable neighbouring tile", n.id)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn translate(id: &str, prompt: &str, answer: &str, hints: &[(&str, &str)]) -> ExerciseDef {
        ExerciseDef {
            id: id.into(),
            kind: ExerciseKind::Translate,
            prompt: prompt.into(),
            prompt_lang: Lang::En,
            context: None,
            options: vec![],
            answer: vec![answer.into()],
            hint: None,
            word_hints: hints
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect(),
            explanation: Some("because".into()),
            needs_native_review: true,
        }
    }

    fn diagnostics_for(exercises: Vec<ExerciseDef>) -> Vec<Diagnostic> {
        let mut c = Content::default();
        c.grammar.insert(
            "g".into(),
            GrammarConcept {
                id: "g".into(),
                title: "t".into(),
                summary: "s".into(),
                rules: vec![],
                examples: vec![],
                pt_pt_note: None,
                exercises,
                needs_native_review: true,
            },
        );
        c.finalize();
        validate(&c)
    }

    fn filler(n: usize) -> Vec<ExerciseDef> {
        (0..n)
            .map(|i| translate(&format!("f_{i}"), "Hello.", "Olá.", &[]))
            .collect()
    }

    #[test]
    fn a_good_translation_exercise_is_clean() {
        let mut ex = filler(2);
        ex.push(translate(
            "t_ok",
            "The bridge is high.",
            "A ponte é alta.",
            &[("bridge", "ponte"), ("high", "alta")],
        ));
        let d = diagnostics_for(ex);
        // (an empty test Content has no story or maps: only look at the exercises)
        assert!(
            d.iter()
                .filter(|x| x.location.starts_with("exercise:") || x.location.starts_with("grammar:"))
                .all(|x| x.severity == Severity::Info),
            "unexpected diagnostics: {d:?}"
        );
    }

    #[test]
    fn translation_lints_catch_typos_and_overlong_text() {
        let mut ex = filler(2);
        // hint word missing from the answer (typo `pnte`), prompt too long, wrong prompt language
        let mut bad = translate(
            "t_bad",
            "This English sentence is much too long for the typing screen.",
            "A ponte é alta.",
            &[("bridge", "pnte"), ("high", "ALTA")],
        );
        bad.prompt_lang = Lang::Pt;
        ex.push(bad);
        ex.push(translate("t_long", "Hello.", &"a ".repeat(40), &[]));
        let d = diagnostics_for(ex);
        let has = |sev: Severity, loc: &str, needle: &str| {
            d.iter()
                .any(|x| x.severity == sev && x.location == loc && x.message.contains(needle))
        };
        assert!(has(Severity::Warning, "exercise:t_bad", "pnte"), "{d:?}");
        assert!(
            !has(Severity::Warning, "exercise:t_bad", "ALTA"),
            "case-insensitive: {d:?}"
        );
        assert!(has(Severity::Warning, "exercise:t_bad", "prompt_lang"), "{d:?}");
        assert!(has(Severity::Warning, "exercise:t_bad", "clipped"), "{d:?}");
        assert!(has(Severity::Error, "exercise:t_long", "typing limit"), "{d:?}");
    }

    #[test]
    fn word_hints_on_other_kinds_are_flagged() {
        let mut ex = filler(2);
        let mut e = translate("t_kind", "Hello.", "Olá.", &[("hello", "olá")]);
        e.kind = ExerciseKind::Type;
        ex.push(e);
        let d = diagnostics_for(ex);
        assert!(
            d.iter()
                .any(|x| x.location == "exercise:t_kind" && x.message.contains("word_hints"))
        );
    }
}
