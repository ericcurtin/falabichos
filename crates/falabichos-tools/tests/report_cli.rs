// SPDX-License-Identifier: Apache-2.0
//! End-to-end tests of the `falabichos-report` and `falabichos-dashboard` binaries.

use falabichos_core::model::Dir;
use falabichos_core::save::{PlayerState, SaveGame};
use falabichos_core::srs::{Card, Phase};
use std::path::PathBuf;
use std::process::Command;

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("falabichos-tools-test-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn write_save(dir: &std::path::Path) -> PathBuf {
    let mut s = SaveGame::new(
        PlayerState {
            map: "vila_azul".into(),
            x: 1,
            y: 1,
            facing: Dir::Down,
        },
        1_000_000,
    );
    s.srs.cards.insert(
        "bica".into(),
        Card {
            phase: Phase::Review,
            interval_days: 40.0,
            due: 1_000_000 + 5 * 86_400,
            seen: 6,
            correct: 5,
            ..Card::default()
        },
    );
    let p = dir.join("save.ron");
    std::fs::write(&p, s.to_ron().unwrap()).unwrap();
    p
}

fn run(bin: &str, args: &[&str]) -> std::process::Output {
    Command::new(bin).args(args).output().expect("binary runs")
}

#[test]
fn report_binary_writes_text_csv_and_json() {
    let bin = env!("CARGO_BIN_EXE_falabichos-report");
    let dir = scratch("report");
    let save = write_save(&dir);
    let save = save.to_str().unwrap();

    let out = run(bin, &[save]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains("Falabichos progress report") && text.contains("mastered (interval >= 21 days): 1")
    );

    let csv = dir.join("out.csv");
    let out = run(bin, &[save, "--format", "csv", "-o", csv.to_str().unwrap()]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let csv = std::fs::read_to_string(csv).unwrap();
    assert!(csv.starts_with("skill,kind,group,label_pt"));
    assert!(
        csv.lines()
            .any(|l| l.starts_with("bica,vocab,cafe,a bica,espresso")),
        "{csv}"
    );

    // flags may come before the save path
    let out = run(bin, &["--format", "json", save]);
    assert!(out.status.success());
    let json = String::from_utf8(out.stdout).unwrap();
    assert!(json.starts_with("{\"format\":\"falabichos-progress-report\""));
    assert!(json.contains("\"skills_mastered\":1"));
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn report_binary_fails_clearly() {
    let bin = env!("CARGO_BIN_EXE_falabichos-report");
    let dir = scratch("report-errors");
    let save = write_save(&dir);
    let out = run(bin, &[dir.join("missing.ron").to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("cannot read"));
    let out = run(bin, &[save.to_str().unwrap(), "--format", "xml"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown format"));
    let junk = dir.join("junk.ron");
    std::fs::write(&junk, "this is not a save").unwrap();
    let out = run(bin, &[junk.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("not a usable save"));
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn dashboard_binary_writes_a_static_page() {
    let bin = env!("CARGO_BIN_EXE_falabichos-dashboard");
    let dir = scratch("dashboard");
    let page = dir.join("nested").join("dashboard.html");
    let out = run(bin, &["-o", page.to_str().unwrap()]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let html = std::fs::read_to_string(page).unwrap();
    assert!(html.contains("Falabichos content dashboard") && !html.to_lowercase().contains("<script"));
    std::fs::remove_dir_all(dir).ok();
}
