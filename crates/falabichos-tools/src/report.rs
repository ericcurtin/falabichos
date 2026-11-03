// SPDX-License-Identifier: Apache-2.0
//! Teacher tools: turn a learner's save file (their SRS state and progress) into a progress report as
//! plain text, CSV (one row per word or exercise, for a spreadsheet) or JSON.
//!
//! Everything is computed locally from the save file; nothing is sent anywhere. The JSON writer is
//! hand-written (the project avoids a JSON dependency for one report) and is tested with a validator.

use falabichos_core::Content;
use falabichos_core::progress::bicho_level;
use falabichos_core::save::SaveGame;
use falabichos_core::srs::{Card, DAY, Phase, ReviewCalendar};
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// A card counts as *mastered* when it has been reviewed into an interval of three weeks or more
/// (the usual "mature card" threshold of spaced-repetition tools).
pub const MASTERED_DAYS: f32 = 21.0;

#[derive(Debug, Clone, PartialEq)]
pub struct SkillRow {
    pub skill: String,
    /// `vocab`, `grammar` or `unknown` (a card whose content was removed).
    pub kind: &'static str,
    /// Vocabulary category or grammar concept id.
    pub group: String,
    pub label_pt: String,
    pub label_en: String,
    /// Bichos that teach this skill.
    pub species: Vec<String>,
    /// `new`, `learning` or `review`.
    pub phase: &'static str,
    pub interval_days: f32,
    pub ease: f32,
    pub due: i64,
    pub reps: u32,
    pub lapses: u32,
    pub seen: u32,
    pub correct: u32,
    pub mastery: f32,
    pub mastered: bool,
}

impl SkillRow {
    pub fn accuracy(&self) -> Option<f32> {
        (self.seen > 0).then(|| self.correct as f32 / self.seen as f32)
    }
    pub fn started(&self) -> bool {
        self.phase != "new"
    }
}

/// Totals for a vocabulary category or a grammar concept.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupRow {
    pub name: String,
    pub total: usize,
    pub started: usize,
    pub mastered: usize,
    pub seen: u32,
    pub correct: u32,
}

impl GroupRow {
    pub fn accuracy(&self) -> Option<f32> {
        (self.seen > 0).then(|| self.correct as f32 / self.seen as f32)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpeciesRow {
    pub id: String,
    pub name: String,
    pub caught: bool,
    pub level: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub generated_at: i64,
    pub saved_at: i64,
    pub play_secs: u64,
    pub steps: u64,
    pub bichos_seen: usize,
    pub bichos_caught: usize,
    pub seals: Vec<String>,
    /// Adaptive strictness rung (0 = forgiving, 2 = strict); only used when the learner switched it on.
    pub strictness_rung: u8,
    pub species: Vec<SpeciesRow>,
    pub calendar: ReviewCalendar,
    pub skills: Vec<SkillRow>,
    pub categories: Vec<GroupRow>,
    pub concepts: Vec<GroupRow>,
}

impl Report {
    pub fn started(&self) -> usize {
        self.skills.iter().filter(|s| s.started()).count()
    }
    pub fn mastered(&self) -> usize {
        self.skills.iter().filter(|s| s.mastered).count()
    }
    /// Mean mastery over the skills that were started (0 when none).
    pub fn average_mastery(&self) -> f32 {
        let started: Vec<&SkillRow> = self.skills.iter().filter(|s| s.started()).collect();
        if started.is_empty() {
            0.0
        } else {
            started.iter().map(|s| s.mastery).sum::<f32>() / started.len() as f32
        }
    }
    /// Overall accuracy of every answer ever given (None before the first answer).
    pub fn accuracy(&self) -> Option<f32> {
        let (seen, correct) = self.skills.iter().fold((0u64, 0u64), |(a, b), s| {
            (a + s.seen as u64, b + s.correct as u64)
        });
        (seen > 0).then(|| correct as f32 / seen as f32)
    }
}

fn phase_name(c: &Card) -> &'static str {
    match c.phase {
        Phase::New => "new",
        Phase::Learning(_) => "learning",
        Phase::Review => "review",
    }
}

fn row(
    skill: &str,
    card: Option<&Card>,
    kind: &'static str,
    group: &str,
    pt: &str,
    en: &str,
    species: Vec<String>,
) -> SkillRow {
    let default = Card::default();
    let c = card.unwrap_or(&default);
    SkillRow {
        skill: skill.to_string(),
        kind,
        group: group.to_string(),
        label_pt: pt.to_string(),
        label_en: en.to_string(),
        species,
        phase: phase_name(c),
        interval_days: c.interval_days,
        ease: c.ease,
        due: c.due,
        reps: c.reps,
        lapses: c.lapses,
        seen: c.seen,
        correct: c.correct,
        mastery: c.mastery(),
        mastered: c.phase == Phase::Review && c.interval_days >= MASTERED_DAYS,
    }
}

/// Build the report for `save` against `content` as of `now` (unix seconds).
pub fn build(content: &Content, save: &SaveGame, now: i64) -> Report {
    let mut skills: Vec<SkillRow> = vec![];
    for v in content.vocab.values() {
        let species: Vec<String> = content
            .species
            .values()
            .filter(|s| s.vocab.contains(&v.id))
            .map(|s| s.id.clone())
            .collect();
        skills.push(row(
            &v.id,
            save.srs.card(&v.id),
            "vocab",
            &v.category,
            &v.with_article(),
            v.primary_en(),
            species,
        ));
    }
    for g in content.grammar.values() {
        let species: Vec<String> = content
            .species
            .values()
            .filter(|s| s.grammar.contains(&g.id))
            .map(|s| s.id.clone())
            .collect();
        for e in &g.exercises {
            skills.push(row(
                &e.id,
                save.srs.card(&e.id),
                "grammar",
                &g.id,
                &e.prompt,
                e.context.as_deref().unwrap_or(""),
                species.clone(),
            ));
        }
    }
    // cards whose content no longer exists (kept so the teacher sees the whole history)
    let known: std::collections::BTreeSet<String> = skills.iter().map(|s| s.skill.clone()).collect();
    for (id, card) in &save.srs.cards {
        if !known.contains(id) {
            skills.push(row(id, Some(card), "unknown", "", id, "", vec![]));
        }
    }

    let group = |kind: &str| -> Vec<GroupRow> {
        let mut m: BTreeMap<String, GroupRow> = BTreeMap::new();
        for s in skills.iter().filter(|s| s.kind == kind) {
            let g = m.entry(s.group.clone()).or_insert_with(|| GroupRow {
                name: s.group.clone(),
                total: 0,
                started: 0,
                mastered: 0,
                seen: 0,
                correct: 0,
            });
            g.total += 1;
            g.started += usize::from(s.started());
            g.mastered += usize::from(s.mastered);
            g.seen += s.seen;
            g.correct += s.correct;
        }
        m.into_values().collect()
    };
    let (categories, concepts) = (group("vocab"), group("grammar"));

    let mut species: Vec<SpeciesRow> = content
        .species
        .values()
        .map(|s| SpeciesRow {
            id: s.id.clone(),
            name: s.name.clone(),
            caught: save.progress.has_bicho(&s.id),
            level: bicho_level(content, &save.srs, &s.id),
        })
        .collect();
    species.sort_by_key(|s| content.species[&s.id].dex_no);
    let (seen, caught) = save.progress.dex_counts();
    Report {
        generated_at: now,
        saved_at: save.saved_at,
        play_secs: save.play_secs,
        steps: save.steps,
        bichos_seen: seen,
        bichos_caught: caught,
        seals: save.progress.seals.iter().cloned().collect(),
        strictness_rung: save.ladder.rung,
        species,
        calendar: save.srs.calendar(now),
        skills,
        categories,
        concepts,
    }
}

fn percent(x: Option<f32>) -> String {
    x.map(|v| format!("{:.0}%", v * 100.0))
        .unwrap_or_else(|| "-".into())
}

fn hours(due: i64, now: i64) -> String {
    format!("{:.1}", (due - now) as f32 / 3600.0)
}

/// Human-readable report.
pub fn to_text(r: &Report) -> String {
    let mut o = String::new();
    let _ = writeln!(o, "Falabichos progress report");
    let _ = writeln!(o, "==========================");
    let _ = writeln!(
        o,
        "Play time: {} min | steps: {} | Bichos caught {} / seen {} | Selos: {}",
        r.play_secs / 60,
        r.steps,
        r.bichos_caught,
        r.bichos_seen,
        if r.seals.is_empty() {
            "none".to_string()
        } else {
            r.seals.join(", ")
        }
    );
    let total = r.skills.iter().filter(|s| s.kind != "unknown").count();
    let _ = writeln!(
        o,
        "Started {} of {} skills | mastered (interval >= {} days): {} | accuracy: {} | average mastery: {:.0}%",
        r.started(),
        total,
        MASTERED_DAYS as u32,
        r.mastered(),
        percent(r.accuracy()),
        r.average_mastery() * 100.0
    );
    let _ = writeln!(
        o,
        "Typed-answer strictness rung (adaptive, 0 forgiving to 2 strict): {}",
        r.strictness_rung
    );
    let c = &r.calendar;
    let _ = writeln!(
        o,
        "Due now: {} | within 24 h: {} | 24-48 h: {} | next 7 days: {:?} | later: {}",
        c.due_now, c.later_today, c.tomorrow, c.week, c.later
    );
    let _ = writeln!(
        o,
        "\nVocabulary by category (started / total, mastered, accuracy)"
    );
    for g in &r.categories {
        let _ = writeln!(
            o,
            "  {:<18} {:>3} / {:<3}  mastered {:>3}  accuracy {}",
            g.name,
            g.started,
            g.total,
            g.mastered,
            percent(g.accuracy())
        );
    }
    let _ = writeln!(
        o,
        "\nGrammar concepts (started / total exercises, mastered, accuracy)"
    );
    for g in &r.concepts {
        let _ = writeln!(
            o,
            "  {:<22} {:>3} / {:<3}  mastered {:>3}  accuracy {}",
            g.name,
            g.started,
            g.total,
            g.mastered,
            percent(g.accuracy())
        );
    }
    let _ = writeln!(o, "\nBichos");
    for s in &r.species {
        let _ = writeln!(
            o,
            "  {:<10} {:<8} level {}",
            s.name,
            if s.caught { "caught" } else { "-" },
            s.level
        );
    }
    // the words that need attention: lapsed or answered badly
    let mut weak: Vec<&SkillRow> = r
        .skills
        .iter()
        .filter(|s| s.started() && s.seen >= 2 && s.accuracy().is_some_and(|a| a < 0.6))
        .collect();
    weak.sort_by(|a, b| {
        a.accuracy()
            .partial_cmp(&b.accuracy())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let _ = writeln!(
        o,
        "\nNeeds attention (answered correctly less than 60% of the time, at least 2 answers)"
    );
    if weak.is_empty() {
        let _ = writeln!(o, "  nothing yet");
    }
    for s in weak.iter().take(15) {
        let _ = writeln!(
            o,
            "  {:<22} {:<28} {}/{} right, {} lapse(s)",
            s.skill,
            s.label_pt.chars().take(28).collect::<String>(),
            s.correct,
            s.seen,
            s.lapses
        );
    }
    o
}

/// Neutralise spreadsheet formulas: a text cell that starts with `= + - @` (or a control character)
/// could be executed when the CSV is opened in a spreadsheet.
fn csv_text(s: &str) -> String {
    let danger = s
        .chars()
        .next()
        .is_some_and(|c| matches!(c, '=' | '+' | '-' | '@' | '\t' | '\r'));
    let s = if danger { format!("'{s}") } else { s.to_string() };
    csv_quote(&s)
}

/// RFC 4180 quoting: quote when the cell holds a comma, quote, CR or LF.
fn csv_quote(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// One row per skill (every vocabulary item and grammar exercise, started or not).
pub fn to_csv(r: &Report) -> String {
    let mut o = String::from(
        "skill,kind,group,label_pt,label_en,bichos,phase,interval_days,ease,due_in_hours,reps,lapses,seen,correct,accuracy,mastery,mastered\r\n",
    );
    for s in &r.skills {
        let due = if s.started() {
            hours(s.due, r.generated_at)
        } else {
            String::new()
        };
        let accuracy = s.accuracy().map(|a| format!("{a:.3}")).unwrap_or_default();
        let _ = write!(
            o,
            "{},{},{},{},{},{},{},{:.2},{:.2},{},{},{},{},{},{},{:.3},{}\r\n",
            csv_text(&s.skill),
            s.kind,
            csv_text(&s.group),
            csv_text(&s.label_pt),
            csv_text(&s.label_en),
            csv_text(&s.species.join(" ")),
            s.phase,
            s.interval_days,
            s.ease,
            due,
            s.reps,
            s.lapses,
            s.seen,
            s.correct,
            accuracy,
            s.mastery,
            s.mastered
        );
    }
    o
}

/// JSON string literal (quotes, backslashes and control characters escaped; other text kept as UTF-8).
pub fn json_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn json_num(x: f32) -> String {
    if x.is_finite() {
        format!("{x:.4}")
    } else {
        "null".into()
    }
}

fn json_opt(x: Option<f32>) -> String {
    x.map(json_num).unwrap_or_else(|| "null".into())
}

fn json_group(g: &GroupRow) -> String {
    format!(
        "{{\"name\":{},\"total\":{},\"started\":{},\"mastered\":{},\"answers\":{},\"correct\":{},\"accuracy\":{}}}",
        json_str(&g.name),
        g.total,
        g.started,
        g.mastered,
        g.seen,
        g.correct,
        json_opt(g.accuracy())
    )
}

/// The whole report as one JSON document.
pub fn to_json(r: &Report) -> String {
    let c = &r.calendar;
    let week: Vec<String> = c.week.iter().map(|n| n.to_string()).collect();
    let seals: Vec<String> = r.seals.iter().map(|s| json_str(s)).collect();
    let species: Vec<String> = r
        .species
        .iter()
        .map(|s| {
            format!(
                "{{\"id\":{},\"name\":{},\"caught\":{},\"level\":{}}}",
                json_str(&s.id),
                json_str(&s.name),
                s.caught,
                s.level
            )
        })
        .collect();
    let cats: Vec<String> = r.categories.iter().map(json_group).collect();
    let concepts: Vec<String> = r.concepts.iter().map(json_group).collect();
    let skills: Vec<String> = r
        .skills
        .iter()
        .map(|s| {
            let bichos: Vec<String> = s.species.iter().map(|x| json_str(x)).collect();
            format!(
                "{{\"skill\":{},\"kind\":{},\"group\":{},\"label_pt\":{},\"label_en\":{},\"bichos\":[{}],\"phase\":{},\
                 \"interval_days\":{},\"ease\":{},\"due_in_hours\":{},\"reps\":{},\"lapses\":{},\"answers\":{},\"correct\":{},\
                 \"accuracy\":{},\"mastery\":{},\"mastered\":{}}}",
                json_str(&s.skill),
                json_str(s.kind),
                json_str(&s.group),
                json_str(&s.label_pt),
                json_str(&s.label_en),
                bichos.join(","),
                json_str(s.phase),
                json_num(s.interval_days),
                json_num(s.ease),
                if s.started() { json_num((s.due - r.generated_at) as f32 / 3600.0) } else { "null".into() },
                s.reps,
                s.lapses,
                s.seen,
                s.correct,
                json_opt(s.accuracy()),
                json_num(s.mastery),
                s.mastered
            )
        })
        .collect();
    format!(
        "{{\"format\":\"falabichos-progress-report\",\"version\":1,\"generated_at\":{},\
         \"learner\":{{\"saved_at\":{},\"play_seconds\":{},\"steps\":{},\"bichos_seen\":{},\"bichos_caught\":{},\"seals\":[{}],\"strictness_rung\":{},\"bichos\":[{}]}},\
         \"srs\":{{\"skills_started\":{},\"skills_mastered\":{},\"mastered_interval_days\":{},\"average_mastery\":{},\"accuracy\":{},\
         \"calendar\":{{\"due_now\":{},\"due_now_learning\":{},\"within_24h\":{},\"within_24_to_48h\":{},\"next_7_days\":[{}],\"later\":{}}}}},\
         \"categories\":[{}],\"concepts\":[{}],\"skills\":[{}]}}\n",
        r.generated_at,
        r.saved_at,
        r.play_secs,
        r.steps,
        r.bichos_seen,
        r.bichos_caught,
        seals.join(","),
        r.strictness_rung,
        species.join(","),
        r.started(),
        r.mastered(),
        MASTERED_DAYS as u32,
        json_num(r.average_mastery()),
        json_opt(r.accuracy()),
        c.due_now,
        c.due_now_learning,
        c.later_today,
        c.tomorrow,
        week.join(","),
        c.later,
        cats.join(","),
        concepts.join(","),
        skills.join(",")
    )
}

/// Output formats of `falabichos-report`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Text,
    Csv,
    Json,
}

impl Format {
    pub fn parse(s: &str) -> Option<Format> {
        match s {
            "text" | "txt" => Some(Format::Text),
            "csv" => Some(Format::Csv),
            "json" => Some(Format::Json),
            _ => None,
        }
    }
}

pub fn render(r: &Report, f: Format) -> String {
    match f {
        Format::Text => to_text(r),
        Format::Csv => to_csv(r),
        Format::Json => to_json(r),
    }
}

/// Seconds in a day, re-exported for the binary's `--now` help text.
pub const SECONDS_PER_DAY: i64 = DAY;

#[cfg(test)]
mod tests {
    use super::*;
    use falabichos_assets::Assets;
    use falabichos_core::model::*;
    use falabichos_core::save::PlayerState;

    const NOW: i64 = 2_000_000;

    fn content() -> Content {
        falabichos_content::embedded_content()
    }

    fn save_with_progress(c: &Content) -> SaveGame {
        let mut s = SaveGame::new(
            PlayerState {
                map: "x".into(),
                x: 0,
                y: 0,
                facing: Dir::Down,
            },
            NOW - 100,
        );
        s.play_secs = 1830;
        s.ladder.rung = 1;
        s.steps = 4242;
        s.progress.add_bicho("galo", NOW);
        s.progress.see("sardinha");
        s.progress.award_seal("selo_polvo");
        let mk = |phase, interval: f32, due, seen, correct, lapses| Card {
            phase,
            interval_days: interval,
            due,
            seen,
            correct,
            lapses,
            ..Card::default()
        };
        // galo: mastered; galinha: learning and wrong a lot; vaca: review, due tomorrow
        s.srs
            .cards
            .insert("galo".into(), mk(Phase::Review, 30.0, NOW + 20 * DAY, 10, 9, 0));
        s.srs
            .cards
            .insert("galinha".into(), mk(Phase::Learning(0), 0.0, NOW - 10, 5, 1, 3));
        s.srs
            .cards
            .insert("vaca".into(), mk(Phase::Review, 3.0, NOW + DAY + 100, 4, 4, 0));
        s.srs
            .cards
            .insert("removed_skill".into(), mk(Phase::Review, 2.0, NOW + 5, 1, 1, 0));
        // a grammar exercise
        let ex = c.grammar["ter_haver"].exercises[0].id.clone();
        s.srs
            .cards
            .insert(ex, mk(Phase::Review, 25.0, NOW + 10 * DAY, 3, 3, 0));
        s
    }

    #[test]
    fn report_numbers_match_the_save() {
        let c = content();
        let r = build(&c, &save_with_progress(&c), NOW);
        assert_eq!((r.bichos_caught, r.bichos_seen), (1, 2));
        assert_eq!(r.seals, vec!["selo_polvo".to_string()]);
        assert_eq!(r.started(), 5);
        assert_eq!(
            r.mastered(),
            2,
            "galo (30 days) and the grammar exercise (25 days)"
        );
        let galo = r.skills.iter().find(|s| s.skill == "galo").unwrap();
        assert_eq!(
            (galo.kind, galo.group.as_str(), galo.phase),
            ("vocab", "farm", "review")
        );
        assert_eq!(galo.species, vec!["galo".to_string()]);
        assert_eq!(galo.accuracy(), Some(0.9));
        let unk = r.skills.iter().find(|s| s.skill == "removed_skill").unwrap();
        assert_eq!(unk.kind, "unknown");
        let farm = r.categories.iter().find(|g| g.name == "farm").unwrap();
        assert_eq!((farm.started, farm.mastered), (3, 1));
        assert_eq!(
            farm.total,
            c.vocab.values().filter(|v| v.category == "farm").count()
        );
        let th = r.concepts.iter().find(|g| g.name == "ter_haver").unwrap();
        assert_eq!((th.started, th.mastered, th.seen, th.correct), (1, 1, 3, 3));
        assert_eq!(
            r.calendar.due_now, 1,
            "only galinha is due (the removed skill is due in 5 s)"
        );
        // every vocabulary item and exercise is listed once, started or not
        let total: usize = c.vocab.len() + c.grammar.values().map(|g| g.exercises.len()).sum::<usize>();
        assert_eq!(r.skills.len(), total + 1);
        assert_eq!(r.species.len(), c.species.len());
        assert!(r.species[0].caught || r.species.iter().any(|s| s.id == "galo" && s.caught));
    }

    #[test]
    fn empty_save_gives_a_sane_report_in_every_format() {
        let c = content();
        let save = SaveGame::new(
            PlayerState {
                map: "x".into(),
                x: 0,
                y: 0,
                facing: Dir::Down,
            },
            NOW,
        );
        let r = build(&c, &save, NOW);
        assert_eq!((r.started(), r.mastered()), (0, 0));
        assert_eq!(r.accuracy(), None);
        let text = to_text(&r);
        assert!(text.contains("Started 0 of"), "{text}");
        assert!(text.contains("nothing yet"));
        assert!(to_csv(&r).lines().count() > 100);
        assert_valid_json(&to_json(&r));
    }

    #[test]
    fn text_report_mentions_the_essentials() {
        let c = content();
        let r = build(&c, &save_with_progress(&c), NOW);
        let t = to_text(&r);
        for needle in [
            "Play time: 30 min",
            "steps: 4242",
            "caught 1 / seen 2",
            "selo_polvo",
            "mastered (interval >= 21 days): 2",
            "farm",
            "ter_haver",
            "rung (adaptive, 0 forgiving to 2 strict): 1",
            "Needs attention",
            "galinha",
        ] {
            assert!(t.contains(needle), "missing {needle:?} in\n{t}");
        }
    }

    #[test]
    fn csv_is_rfc4180_and_neutralises_formulas() {
        assert_eq!(csv_quote("plain"), "plain");
        assert_eq!(csv_quote("a,b"), "\"a,b\"");
        assert_eq!(csv_quote("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_quote("two\nlines"), "\"two\nlines\"");
        assert_eq!(csv_text("=1+1"), "'=1+1");
        assert_eq!(csv_text("-3"), "'-3");
        assert_eq!(csv_text("@sum"), "'@sum");
        assert_eq!(csv_text("normal"), "normal");
        let c = content();
        let r = build(&c, &save_with_progress(&c), NOW);
        let csv = to_csv(&r);
        let mut lines = csv.split("\r\n").filter(|l| !l.is_empty());
        let header = lines.next().unwrap();
        assert!(header.starts_with("skill,kind,group,label_pt"));
        // every grammar prompt holds commas or quotes somewhere: parse the CSV back with a tiny reader
        let parsed = parse_csv(&csv);
        assert_eq!(parsed.len(), r.skills.len() + 1);
        let cols = header.split(',').count();
        assert!(parsed.iter().all(|row| row.len() == cols), "ragged rows");
        let galo = parsed.iter().find(|row| row[0] == "galo").unwrap();
        assert_eq!(galo[1], "vocab");
        assert_eq!(galo[6], "review");
        assert_eq!(galo[15], "1.000", "mastery column: {galo:?}");
    }

    /// Minimal CSV reader (quotes, doubled quotes, CRLF) used to check the writer.
    fn parse_csv(text: &str) -> Vec<Vec<String>> {
        let (mut rows, mut row, mut cell) = (vec![], vec![], String::new());
        let mut chars = text.chars().peekable();
        let mut quoted = false;
        while let Some(ch) = chars.next() {
            match (quoted, ch) {
                (true, '"') if chars.peek() == Some(&'"') => {
                    cell.push('"');
                    chars.next();
                }
                (true, '"') => quoted = false,
                (true, c) => cell.push(c),
                (false, '"') => quoted = true,
                (false, ',') => row.push(std::mem::take(&mut cell)),
                (false, '\r') => {}
                (false, '\n') => {
                    row.push(std::mem::take(&mut cell));
                    rows.push(std::mem::take(&mut row));
                }
                (false, c) => cell.push(c),
            }
        }
        rows
    }

    #[test]
    fn json_strings_are_escaped() {
        assert_eq!(json_str("a\"b\\c\nd\te\u{1}ç"), "\"a\\\"b\\\\c\\nd\\te\\u0001ç\"");
        assert_eq!(json_num(f32::NAN), "null");
    }

    #[test]
    fn json_report_is_valid_and_complete() {
        let c = content();
        let r = build(&c, &save_with_progress(&c), NOW);
        let j = to_json(&r);
        assert_valid_json(&j);
        for needle in [
            "\"format\":\"falabichos-progress-report\"",
            "\"play_seconds\":1830",
            "\"seals\":[\"selo_polvo\"]",
            "\"strictness_rung\":1",
            "\"skills_started\":5",
            "\"skills_mastered\":2",
            "\"skill\":\"galo\"",
            "\"phase\":\"learning\"",
            "\"due_now\":1",
        ] {
            assert!(j.contains(needle), "missing {needle}");
        }
        // an unstarted skill has no due time
        assert!(j.contains("\"phase\":\"new\"") && j.contains("\"due_in_hours\":null"));
    }

    #[test]
    fn hostile_labels_cannot_break_json_or_csv() {
        let c = content();
        let mut r = build(&c, &save_with_progress(&c), NOW);
        r.skills[0].label_pt = "=HYPERLINK(\"http://x\",\"y\"),\n\"quote\"\\".into();
        let csv = to_csv(&r);
        assert!(csv.contains("\"'=HYPERLINK("), "formula is neutralised");
        assert_valid_json(&to_json(&r));
        let parsed = parse_csv(&csv);
        assert_eq!(parsed[1][3], "'=HYPERLINK(\"http://x\",\"y\"),\n\"quote\"\\");
    }

    #[test]
    fn formats_parse() {
        assert_eq!(Format::parse("csv"), Some(Format::Csv));
        assert_eq!(Format::parse("json"), Some(Format::Json));
        assert_eq!(Format::parse("text"), Some(Format::Text));
        assert_eq!(Format::parse("xml"), None);
        let _ = Assets::embedded();
    }

    // ---- a small strict JSON validator (no dependency): returns on the first syntax error
    fn assert_valid_json(s: &str) {
        let b: Vec<char> = s.chars().collect();
        let mut i = 0;
        fn ws(b: &[char], i: &mut usize) {
            while *i < b.len() && b[*i].is_whitespace() {
                *i += 1;
            }
        }
        fn value(b: &[char], i: &mut usize) {
            ws(b, i);
            match b.get(*i).copied() {
                Some('{') => {
                    *i += 1;
                    ws(b, i);
                    if b[*i] == '}' {
                        *i += 1;
                        return;
                    }
                    loop {
                        ws(b, i);
                        string(b, i);
                        ws(b, i);
                        assert_eq!(b[*i], ':', "expected ':' at {i}");
                        *i += 1;
                        value(b, i);
                        ws(b, i);
                        match b[*i] {
                            ',' => *i += 1,
                            '}' => {
                                *i += 1;
                                return;
                            }
                            c => panic!("unexpected {c:?} at {i}"),
                        }
                    }
                }
                Some('[') => {
                    *i += 1;
                    ws(b, i);
                    if b[*i] == ']' {
                        *i += 1;
                        return;
                    }
                    loop {
                        value(b, i);
                        ws(b, i);
                        match b[*i] {
                            ',' => *i += 1,
                            ']' => {
                                *i += 1;
                                return;
                            }
                            c => panic!("unexpected {c:?} at {i}"),
                        }
                    }
                }
                Some('"') => string(b, i),
                Some('t') => lit(b, i, "true"),
                Some('f') => lit(b, i, "false"),
                Some('n') => lit(b, i, "null"),
                Some(c) if c == '-' || c.is_ascii_digit() => {
                    let start = *i;
                    while *i < b.len()
                        && (b[*i].is_ascii_digit() || matches!(b[*i], '-' | '+' | '.' | 'e' | 'E'))
                    {
                        *i += 1;
                    }
                    let n: String = b[start..*i].iter().collect();
                    assert!(n.parse::<f64>().is_ok(), "bad number {n}");
                }
                other => panic!("unexpected {other:?} at {i}"),
            }
        }
        fn lit(b: &[char], i: &mut usize, w: &str) {
            let got: String = b[*i..(*i + w.len()).min(b.len())].iter().collect();
            assert_eq!(got, w);
            *i += w.len();
        }
        fn string(b: &[char], i: &mut usize) {
            assert_eq!(b[*i], '"', "expected string at {i}");
            *i += 1;
            loop {
                match b[*i] {
                    '"' => {
                        *i += 1;
                        return;
                    }
                    '\\' => {
                        assert!(matches!(
                            b[*i + 1],
                            '"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't' | 'u'
                        ));
                        *i += 2;
                    }
                    c => {
                        assert!(c as u32 >= 0x20, "raw control character in string at {i}");
                        *i += 1;
                    }
                }
            }
        }
        value(&b, &mut i);
        ws(&b, &mut i);
        assert_eq!(i, b.len(), "trailing characters");
    }
}
