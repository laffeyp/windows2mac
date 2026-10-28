// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 127's planted fault for `--check`: a card marked done with no report; Sprint 176's: a tail entry headed
//! closed before its card is done, or with an unfilled gates line.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn scratch(name: &str, cards: &[(&str, &str)], reports: &[&str]) -> PathBuf {
    scratch_with_tail(name, cards, reports, "")
}

fn scratch_with_tail(name: &str, cards: &[(&str, &str)], reports: &[&str], tail: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("w2m-signal-report-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("sprints")).unwrap();
    fs::create_dir_all(root.join("signal-reports")).unwrap();
    for (file, status) in cards {
        fs::write(
            root.join("sprints").join(file),
            format!("# card\n\n```yaml\n---\nstatus: {status}\n---\n```\n"),
        )
        .unwrap();
    }
    for r in reports {
        fs::write(root.join("signal-reports").join(r), "# report\n").unwrap();
    }
    fs::write(
        root.join("BLACKBOARD.md"),
        format!("# BLACKBOARD\n\n## Sprint tail\n{tail}"),
    )
    .unwrap();
    root
}

fn check(root: &PathBuf) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_signal_report"))
        .args(["--check", "--root"])
        .arg(root)
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(root);
    (out.status.code(), String::from_utf8_lossy(&out.stderr).to_string())
}

#[test]
fn a_done_card_without_a_report_fails_naming_the_sprint() {
    let root = scratch(
        "missing",
        &[
            ("sprint-107-old.md", "done"),
            ("sprint-200-x.md", "done"),
            ("sprint-201-y.md", "active"),
            ("sprint-202-z.md", "done"),
        ],
        &["sprint-202-report.md"],
    );
    let (code, stderr) = check(&root);
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr.trim(),
        "sprint 200: its card says done and signal-reports/sprint-200-report.md does not exist"
    );
}

#[test]
fn every_done_card_with_a_report_passes() {
    let root = scratch(
        "complete",
        &[("sprint-200-x.md", "done"), ("sprint-201-y.md", "parked")],
        &["sprint-200-report.md"],
    );
    assert_eq!(check(&root).0, Some(0));
}

#[test]
fn a_tail_entry_closed_before_its_card_or_its_gates_fails() {
    let root = scratch_with_tail(
        "early",
        &[("sprint-200-x.md", "done"), ("sprint-201-y.md", "active")],
        &["sprint-200-report.md"],
        "\n### Sprint 201 (2026-10-28, closed)\n- **Dual contract:** artifact pass (GATES-LINE).\n\n\
         ### Sprint 200 (2026-10-28, closed)\n- **Dual contract:** artifact pass (gates exit 0).\n",
    );
    let (code, stderr) = check(&root);
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr.trim(),
        "sprint 201: its tail entry is headed closed and holds GATES-LINE\n\
         sprint 201: its tail entry is headed closed and its card is not done"
    );
}
