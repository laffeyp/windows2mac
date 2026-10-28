// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 127's planted fault for `--check`: a card marked done with no report; Sprint 176's: a tail entry headed
//! closed before its card is done, or with an unfilled gates line.

use std::fs;
use std::path::{Path, PathBuf};
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
    let (code, stderr, _) = check_on(root, false);
    (code, stderr)
}

/// `--check` with `CI` set to "true" or removed (CI itself sets it): the exit code, stderr and stdout.
fn check_on(root: &PathBuf, ci: bool) -> (Option<i32>, String, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_signal_report"));
    cmd.args(["--check", "--root"]).arg(root);
    if ci {
        cmd.env("CI", "true");
    } else {
        cmd.env_remove("CI");
    }
    let out = cmd.output().unwrap();
    let _ = fs::remove_dir_all(root);
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        String::from_utf8_lossy(&out.stdout).to_string(),
    )
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
        &[("sprint-170-x.md", "done"), ("sprint-171-y.md", "active")],
        &["sprint-170-report.md"],
        "\n### Sprint 171 (2026-10-28, closed)\n- **Dual contract:** artifact pass (GATES-LINE).\n\n\
         ### Sprint 170 (2026-10-28, closed)\n- **Dual contract:** artifact pass (gates exit 0).\n",
    );
    // sprints before 179, whose gates runs were not kept
    let (code, stderr) = check(&root);
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr.trim(),
        "sprint 171: its tail entry is headed closed and holds GATES-LINE\n\
         sprint 171: its tail entry is headed closed and its card is not done"
    );
}

/// Sprint 179's plant 3 (reviewer-2's Medium 3 of its review of 175-177): a card marked done while its tail entry is
/// not headed closed, and a closed entry naming a gates run `gates.sh` never kept.
#[test]
fn a_done_card_needs_a_closed_tail_naming_a_kept_gates_run() {
    let root = scratch_with_tail(
        "gates",
        &[("sprint-180-x.md", "done"), ("sprint-181-y.md", "done")],
        &["sprint-180-report.md", "sprint-181-report.md"],
        "\n### Sprint 181 (2026-10-28, gates running)\n- **Dual contract:** artifact pass (gates exit 0 in 909 s).\n\n\
         ### Sprint 180 (2026-10-28, closed)\n- **Dual contract:** artifact pass (`gates.sh` exit 0 in 912 s).\n",
    );
    fs::create_dir_all(root.join("store/gates")).unwrap();
    fs::write(
        root.join("store/gates/passes.tsv"),
        "exit 0 in 909 s\tdefault\t2026-10-28T07:00:00Z\n",
    )
    .unwrap();
    let (code, stderr) = check(&root);
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr.trim(),
        "sprint 181: its card is done and its tail entry is not headed closed\n\
         sprint 180: its tail entry names \"exit 0 in 912 s\", which store/gates/passes.tsv does not hold"
    );
}

/// Sprint 184's plant 2 (reviewer-2's Medium 3 of its review of 178-181): from Sprint 184 a closed entry's gates run
/// must be its own sprint's; a run of the same seconds kept for another sprint fails, and no kept file fails locally.
#[test]
fn a_closed_entry_needs_a_gates_run_of_its_own_sprint() {
    let tail = "\n### Sprint 186 (2026-10-28, closed)\n- **Dual contract:** artifact pass (`gates.sh` exit 0 in 909 s).\n\n\
                ### Sprint 185 (2026-10-28, closed)\n- **Dual contract:** artifact pass (`gates.sh` exit 0 in 912 s).\n";
    let cards = [("sprint-185-x.md", "done"), ("sprint-186-y.md", "done")];
    let reports = ["sprint-185-report.md", "sprint-186-report.md"];
    let root = scratch_with_tail("bound", &cards, &reports, tail);
    fs::create_dir_all(root.join("store/gates")).unwrap();
    fs::write(
        root.join("store/gates/passes.tsv"),
        "exit 0 in 909 s\tdefault\t2026-10-28T07:00:00Z\tsprint 185\tabc\tdef\n\
         exit 0 in 912 s\tdefault\t2026-10-28T06:00:00Z\tsprint 185\tabc\tdef\n",
    )
    .unwrap();
    let (code, stderr) = check(&root);
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr.trim(),
        "sprint 186: its tail entry names \"exit 0 in 909 s\", which store/gates/passes.tsv holds for another sprint"
    );
    // no kept file fails, on CI too (Sprint 187: the file is tracked)
    let root = scratch_with_tail("unkept", &cards, &reports, tail);
    let (code, stderr, _) = check_on(&root, true);
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr.trim(),
        "store/gates/passes.tsv does not exist: no closed entry's gates run can be read"
    );
}

/// `git` in `dir`, its output trimmed; a failure fails the test.
fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git").arg("-C").arg(dir).args(args).output().unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Sprint 187's plant 1 (reviewer-2's Medium 2 of its review of 182-184): from Sprint 187 a closed entry's gates run
/// must have tested its work commit's trees; a source line changed after the pass and before the work commit fails,
/// naming the tree.
#[test]
fn a_gates_run_must_have_tested_the_work_commits_trees() {
    let tail =
        "\n### Sprint 190 (2026-10-28, closed)\n- **Dual contract:** artifact pass (`gates.sh` exit 0 in 901 s).\n";
    let root = scratch_with_tail("trees", &[("sprint-190-x.md", "done")], &["sprint-190-report.md"], tail);
    fs::create_dir_all(root.join("crates/a")).unwrap();
    fs::write(root.join("crates/a/lib.rs"), "fn a() {}\n").unwrap();
    git(&root, &["init", "-q"]);
    git(&root, &["add", "-A"]);
    git(
        &root,
        &[
            "-c",
            "user.name=w2m",
            "-c",
            "user.email=w2m@example.invalid",
            "commit",
            "-q",
            "-m",
            "Sprint 190's card",
        ],
    );
    // the tree the gates ran on
    let ran = git(&root, &["write-tree", "--prefix=crates/"]);
    // the work, changed after the pass
    fs::write(root.join("crates/a/lib.rs"), "fn a() { let _ = 1; }\n").unwrap();
    git(&root, &["add", "-A"]);
    git(
        &root,
        &[
            "-c",
            "user.name=w2m",
            "-c",
            "user.email=w2m@example.invalid",
            "commit",
            "-q",
            "-m",
            "Sprint 190: the work",
        ],
    );
    let held = git(&root, &["rev-parse", "HEAD:crates"]);
    let commit = git(&root, &["rev-parse", "--short=9", "HEAD"]);
    fs::create_dir_all(root.join("store/gates")).unwrap();
    fs::write(
        root.join("store/gates/passes.tsv"),
        format!("exit 0 in 901 s\tdefault\t2026-10-28T07:00:00Z\tsprint 190\tabc\tdef\tcrates={ran}\n"),
    )
    .unwrap();
    let (code, stderr) = check(&root);
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr.trim(),
        format!("sprint 190: its gates ran on crates/ {ran}, and its work commit {commit} holds {held}")
    );
}

/// Sprint 184 (reviewer-2's Medium 2 of its review of 178-181): "closed-partial" heads only a `closed-partial` card,
/// and a Deferred entry tagged "(Sprint N, partial)" needs card N `closed-partial`.
#[test]
fn a_partial_close_needs_its_cards_status() {
    let root = scratch_with_tail(
        "partial",
        &[
            ("sprint-170-a.md", "done"),
            ("sprint-171-b.md", "closed-partial"),
            ("sprint-172-c.md", "done"),
        ],
        &["sprint-170-report.md", "sprint-172-report.md"],
        "",
    );
    fs::write(
        root.join("BLACKBOARD.md"),
        "# BLACKBOARD\n\n## Deferred\n\n- **2026-10-28 (Sprint 172, partial)** — a class left.\n\
         - **2026-10-28 (Sprint 171, partial)** — another.\n\n## Sprint tail\n\n\
         ### Sprint 171 (2026-10-28, closed-partial)\n- **Dual contract:** observation partial.\n\n\
         ### Sprint 170 (2026-10-28, closed-partial)\n- **Dual contract:** observation partial.\n",
    )
    .unwrap();
    let (code, stderr) = check(&root);
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr.trim(),
        "sprint 170: its tail entry is headed closed-partial and its card is not closed-partial\n\
         sprint 172: a Deferred entry names it partial and its card is not closed-partial"
    );
}
