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
    // the tail archive `--check` reads for the rolled sprints' Built paragraphs (Sprint 232): none rolled here
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("docs/sprint-tail-archive-2026-10-08.md"),
        "# Sprint tail archive\n",
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
    // the work, changed after the pass, committed with the pass line as a close commits it
    fs::write(root.join("crates/a/lib.rs"), "fn a() { let _ = 1; }\n").unwrap();
    fs::create_dir_all(root.join("store/gates")).unwrap();
    let line = format!("exit 0 in 901 s\tdefault\t2026-10-28T07:00:00Z\tsprint 190\tabc\tdef\tcrates={ran}\n");
    fs::write(root.join("store/gates/passes.tsv"), &line).unwrap();
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
    // a later sprint commits its pass line and the next gates append theirs, so the work commit's copy of the file is
    // neither HEAD's nor the working file's (Sprint 188: `git rm --cached` refused that until it was forced)
    let later = format!("{line}exit 0 in 902 s\tdefault\t2026-10-28T08:00:00Z\tsprint 191\tabc\tdef\tcrates={ran}\n");
    fs::write(root.join("store/gates/passes.tsv"), &later).unwrap();
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
            "Sprint 191: later",
        ],
    );
    fs::write(
        root.join("store/gates/passes.tsv"),
        format!("{later}exit 0 in 903 s\tdefault\t2026-10-28T09:00:00Z\tsprint 192\tabc\tdef\tcrates={ran}\n"),
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

/// A commit in `dir` with `message`, every change staged.
fn commit(dir: &Path, message: &str) {
    git(dir, &["add", "-A"]);
    git(
        dir,
        &[
            "-c",
            "user.name=w2m",
            "-c",
            "user.email=w2m@example.invalid",
            "commit",
            "-q",
            "-m",
            message,
        ],
    );
}

/// `gates.sh`'s root tree, by the script `gates.sh` runs (Sprint 195; reviewer-2's Medium 4 of its review of 190-192:
/// the test had copied the steps).
fn gates_root_tree(dir: &Path) -> String {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/gated-tree.sh");
    let out = Command::new("sh").arg(script).current_dir(dir).output().unwrap();
    assert!(
        out.status.success(),
        "gated-tree.sh: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Sprint 192's plants 1 and 2 (reviewer-2's Medium 3 of its review of 185-189): from Sprint 192 the gates' tree is
/// the whole work tree less the bookkeeping paths, so a `Cargo.toml` changed after the pass fails, named; and a close
/// commit that changes a path outside the bookkeeping ones fails, named.
#[test]
fn the_root_tree_and_the_close_commit_are_held() {
    let tail =
        "\n### Sprint 194 (2026-10-28, closed)\n- **Dual contract:** artifact pass (`gates.sh` exit 0 in 905 s).\n";
    let root = scratch_with_tail("root", &[("sprint-194-x.md", "done")], &["sprint-194-report.md"], tail);
    fs::create_dir_all(root.join("crates/a")).unwrap();
    fs::write(root.join("crates/a/lib.rs"), "fn a() {}\n").unwrap();
    fs::write(root.join("Cargo.toml"), "[profile.release]\noverflow-checks = true\n").unwrap();
    git(&root, &["init", "-q"]);
    commit(&root, "Sprint 194's card");
    let ran = gates_root_tree(&root);
    fs::create_dir_all(root.join("store/gates")).unwrap();
    fs::write(
        root.join("store/gates/passes.tsv"),
        format!("exit 0 in 905 s\tdefault\t2026-10-28T07:00:00Z\tsprint 194\tabc\tdef\troot={ran}\n"),
    )
    .unwrap();
    // the work, with Cargo.toml changed after the pass
    fs::write(root.join("Cargo.toml"), "[profile.release]\noverflow-checks = false\n").unwrap();
    commit(&root, "Sprint 194: the work");
    let work = git(&root, &["rev-parse", "--short=9", "HEAD"]);
    let root_copy = root.clone();
    let (code, stderr) = check(&root);
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr.trim(),
        format!("sprint 194: its gates ran on a tree whose Cargo.toml its work commit {work} does not hold")
    );
    let _ = root_copy;

    // the same pass held, and a close commit that changes a crate file
    let root = scratch_with_tail("close", &[("sprint-194-x.md", "done")], &["sprint-194-report.md"], tail);
    fs::create_dir_all(root.join("crates/a")).unwrap();
    fs::write(root.join("crates/a/lib.rs"), "fn a() {}\n").unwrap();
    git(&root, &["init", "-q"]);
    let ran = gates_root_tree(&root);
    fs::create_dir_all(root.join("store/gates")).unwrap();
    fs::write(
        root.join("store/gates/passes.tsv"),
        format!("exit 0 in 905 s\tdefault\t2026-10-28T07:00:00Z\tsprint 194\tabc\tdef\troot={ran}\n"),
    )
    .unwrap();
    commit(&root, "Sprint 194: the work");
    fs::write(root.join("crates/a/lib.rs"), "fn a() { let _ = 2; }\n").unwrap();
    commit(&root, "Sprint 194 closed: its Signal Report");
    let close = git(&root, &["rev-parse", "--short=9", "HEAD"]);
    let (code, stderr) = check(&root);
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr.trim(),
        format!("sprint 194: its close commit {close} changes crates/a/lib.rs, outside the bookkeeping paths")
    );
}

/// Sprint 195's plants 1 and 2 (reviewer-2's Medium 4 and Low 4 of its review of 190-192): from Sprint 195 a commit
/// under another subject between the card and the next card fails, named; a stated multi-vCPU count the stage's rows
/// in the run do not hold fails; and a close that changes a document design_check reads fails.
#[test]
fn every_commit_from_the_card_is_walked_and_the_multi_vcpu_count_is_held() {
    let tail = "\n### Sprint 196 (2026-10-28, closed)\n- **Dual contract:** artifact pass (`gates.sh` exit 0 in 900 s; \
                multi-vCPU: 3 of 3 passed).\n";
    let root = scratch_with_tail("walk", &[("sprint-196-x.md", "done")], &["sprint-196-report.md"], tail);
    fs::create_dir_all(root.join("crates/a")).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("crates/a/lib.rs"), "fn a() {}\n").unwrap();
    fs::write(root.join("docs/design-x-2026-10-28.md"), "# x\n").unwrap();
    fs::write(root.join("docs/notes-2026-10-28.md"), "# notes\n").unwrap();
    git(&root, &["init", "-q"]);
    commit(&root, "Sprint 196 card: x");
    fs::write(root.join("crates/a/lib.rs"), "fn a() { let _ = 1; }\n").unwrap();
    commit(&root, "tidy");
    let tidy = git(&root, &["rev-parse", "--short=9", "HEAD"]);
    let ran = gates_root_tree(&root);
    fs::create_dir_all(root.join("store/gates")).unwrap();
    fs::write(
        root.join("store/gates/passes.tsv"),
        format!("exit 0 in 900 s\tdefault\t2026-10-28T07:00:00Z\tsprint 196\tabc\tdef\troot={ran}\n"),
    )
    .unwrap();
    // two passed inside the run (06:45:00 to 07:00:00), one passed before it
    fs::write(
        root.join("store/gates/multi-vcpu.tsv"),
        "2026-10-28T06:40:00Z\tpassed\tsprint 196\tt1\n\
         2026-10-28T06:59:00Z\tpassed\tsprint 196\tt2\n\
         2026-10-28T06:59:30Z\tfailed\tsprint 196\tt3\n\
         2026-10-28T07:00:00Z\tpassed\tsprint 196\tt1\n",
    )
    .unwrap();
    commit(&root, "Sprint 196: the work");
    fs::write(root.join("docs/notes-2026-10-28.md"), "# notes, more\n").unwrap();
    fs::write(root.join("docs/design-x-2026-10-28.md"), "# x, changed\n").unwrap();
    commit(&root, "Sprint 196 closed: its Signal Report");
    let close = git(&root, &["rev-parse", "--short=9", "HEAD"]);
    let (code, stderr) = check(&root);
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr.trim(),
        format!(
            "sprint 196: its close commit {close} changes docs/design-x-2026-10-28.md, outside the bookkeeping paths\n\
             sprint 196: its tail entry states multi-vCPU 3 of 3 passed, and store/gates/multi-vcpu.tsv holds 2 passed \
             in its gates run\n\
             sprint 196: commit {tidy} \"tidy\" stands between its card and the next card, and is neither its work nor \
             its close"
        )
    );
}

/// A one-sprint repository: its card, its work with the pass line and stage rows given, its close; `--check`'s stderr.
fn staged(name: &str, entry: &str, stages: &str, rows: &str) -> (Option<i32>, String) {
    let tail = format!(
        "\n### Sprint 199 (2026-10-28, closed)\n- **Dual contract:** artifact pass (`gates.sh` exit 0 in 900 s; {entry}).\n"
    );
    let root = scratch_with_tail(name, &[("sprint-199-x.md", "done")], &["sprint-199-report.md"], &tail);
    fs::create_dir_all(root.join("crates/a")).unwrap();
    fs::write(root.join("crates/a/lib.rs"), "fn a() {}\n").unwrap();
    git(&root, &["init", "-q"]);
    commit(&root, "Sprint 199 card: x");
    let ran = gates_root_tree(&root);
    fs::create_dir_all(root.join("store/gates")).unwrap();
    fs::write(
        root.join("store/gates/passes.tsv"),
        format!("exit 0 in 900 s\tdefault\t2026-10-28T07:00:00Z\tsprint 199\tabc\tdef\troot={ran}\t{stages}\n"),
    )
    .unwrap();
    fs::write(root.join("store/gates/multi-vcpu.tsv"), rows).unwrap();
    commit(&root, "Sprint 199: the work");
    check(&root)
}

/// Sprint 198's plants 1 and 2 (reviewer-2's Medium 2 of its review of 193-196): from Sprint 198 the multi-vCPU
/// sentence is held to the stages the pass line records: a skipped stage stated as "0 of 3 passed" fails, so does a run
/// of the stage with two rows; "not run" with no rows passes, and so do three rows with the count stated.
#[test]
fn the_multi_vcpu_sentence_is_held_to_the_stages_that_ran() {
    let rows = |n: usize| -> String {
        (0..n)
            .map(|i| format!("2026-10-28T06:59:0{i}Z\tpassed\tsprint 199\tt{i}\n"))
            .collect()
    };
    let (code, stderr) = staged(
        "skipped",
        "multi-vCPU: 0 of 3 passed",
        "rust=all python=0 samples=0",
        "",
    );
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr.trim(),
        "sprint 199: its gates run did not run the Python gates, and its tail entry does not state \
         \"multi-vCPU: not run (Python gates not run)\""
    );
    let (code, stderr) = staged(
        "two",
        "multi-vCPU: 2 of 3 passed",
        "rust=all python=1 samples=0",
        &rows(2),
    );
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr.trim(),
        "sprint 199: its gates run ran the multi-vCPU stage, and store/gates/multi-vcpu.tsv holds 2 rows in it, not 3"
    );
    let not_run = "multi-vCPU: not run (Python gates not run)";
    assert_eq!(staged("notrun", not_run, "rust=all python=0 samples=0", "").0, Some(0));
    assert_eq!(
        staged(
            "three",
            "multi-vCPU: 3 of 3 passed",
            "rust=all python=1 samples=0",
            &rows(3)
        )
        .0,
        Some(0)
    );
    // Sprint 222: the stage's size is the entry's stated N, held to its rows, not a constant 3: four rows stated as
    // "of 4" pass, and stated as "of 3" are named
    assert_eq!(
        staged(
            "four",
            "multi-vCPU: 4 of 4 passed",
            "rust=all python=1 samples=0",
            &rows(4)
        )
        .0,
        Some(0)
    );
    let (code, stderr) = staged(
        "fourasthree",
        "multi-vCPU: 3 of 3 passed",
        "rust=all python=1 samples=0",
        &rows(4),
    );
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr.trim(),
        "sprint 199: its gates run ran the multi-vCPU stage, and store/gates/multi-vcpu.tsv holds 4 rows in it, not 3"
    );
}

/// Sprint 198's plant 3 (reviewer-2's Low 2 of its review of 193-196): a change to `scripts/gates.sh` or
/// `scripts/gated-tree.sh` runs every stage: the pattern gates.sh sets `all=1` by matches both paths.
#[test]
fn a_change_to_the_gates_runs_every_stage() {
    let gates = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/gates.sh")).unwrap();
    let line = gates
        .lines()
        .find(|l| l.starts_with("touched '") && l.ends_with("&& all=1"))
        .expect("gates.sh sets all=1 by a touched pattern");
    let pattern = line.split('\'').nth(1).unwrap();
    for path in ["scripts/gates.sh", "scripts/gated-tree.sh"] {
        let status = Command::new("grep")
            .args(["-Eq", pattern])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut c| {
                use std::io::Write;
                c.stdin.take().unwrap().write_all(format!("{path}\n").as_bytes())?;
                c.wait()
            })
            .unwrap();
        assert!(status.success(), "a change to {path} does not run every stage");
    }
}

/// `--check` on `root`, which is kept: the exit code and stderr.
fn check_kept(root: &Path) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_signal_report"))
        .args(["--check", "--root"])
        .arg(root)
        .env_remove("CI")
        .output()
        .unwrap();
    (out.status.code(), String::from_utf8_lossy(&out.stderr).to_string())
}

/// A script of `scripts/` run in `dir`, its output trimmed.
fn script(dir: &Path, name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts").join(name);
    let out = Command::new("sh").arg(path).current_dir(dir).output().unwrap();
    assert!(out.status.success(), "{name}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// How `folded` builds its second sprint, 335 unless `first` says otherwise (Sprint 333).
#[derive(Default)]
struct Shape {
    /// a commit "tidy" between 334's close and card 335
    stray: bool,
    /// 335's close prepared on the working tree and not committed
    uncommitted: bool,
    /// 335 closes in a commit of its own after its work commit
    separate_close: bool,
    /// 335's work adds `store/binaries/new.exe`
    binary: bool,
    /// and changes the walk fixture
    walked: bool,
    /// the fixture's first sprint, 334 when none (Sprint 340: the NOT FIRED rule holds from 340)
    first: Option<u32>,
    /// the second sprint's work adds `RECORD`, whose baseline names no commit (Sprint 340)
    record: bool,
    /// and its report is made with `RECORD` moved away, as a report made before Sprint 340 missed it
    stale: bool,
    /// the second sprint's card's Emits names SESSION_INIT
    emits: bool,
    /// after the second sprint's card, a commit "Sprint M card amendment: ..." changing that card and a probe, M the
    /// sprint named (Sprint 349)
    amendment: Option<u32>,
    /// and the amendment also adds `crates/a/extra.rs`
    amendment_crates: bool,
    /// the second sprint's card's exit codes name a laptop job, `target/release/windows_machine probe p` (Sprint 377)
    laptop_card: bool,
    /// and the amendment also writes these paths
    amendment_paths: Vec<&'static str>,
}

const WALK_FIXTURE: &str = "store/translation-cache/walk-fixtures-2026-10-30.json";

/// A record `report` counts and does not narrate (`class-` is a grouped prefix), so no `substrate narrate` runs.
const RECORD: &str = "store/runs/class-fixture-1795600000000000000";

/// `signal_report n --write` on `root`, as a close writes its report (Sprint 340).
fn write_report(root: &Path, n: u32) {
    let out = Command::new(env!("CARGO_BIN_EXE_signal_report"))
        .args([&n.to_string(), "--write", "--root"])
        .arg(root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "signal_report {n} --write: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Sprint 333: a history whose sprints 334 and 335 each close in their work commit. After the previous sprint's close,
/// each sprint is its card commit and one commit "Sprint N: ..." holding the work, the card done, its report, its tail
/// entry headed closed and its gates row, made after the gates passed on the working tree: the row's `root=` is
/// `scripts/gated-tree.sh`'s and its `base=` `scripts/gates-base.sh`'s, as `scripts/gates.sh` takes them. 335's entry
/// cites 334's CI run, on file, whose head is 334's close. From Sprint 340 each report is `signal_report N --write`'s,
/// made on the prepared close before its commit, over a vocabulary of two tags.
fn folded(name: &str, shape: &Shape) -> (PathBuf, String) {
    let root = scratch_with_tail(name, &[], &[], "");
    let a = shape.first.unwrap_or(334);
    let b = a + 1;
    let card = |n: u32, status: &str| {
        let emits = if shape.emits && n == b {
            "\n## signal contract\n\n### Emits\n\n- SESSION_INIT on the sprint's record\n"
        } else {
            ""
        };
        let codes = if shape.laptop_card && n == b {
            "\n## artifact contract\n\n### Command exit codes\n\n- `scripts/gates.sh` returns 0\n- \
             `target/release/windows_machine probe p` returns 0\n\n## observation contract\n"
        } else {
            ""
        };
        fs::write(
            root.join(format!("sprints/sprint-{n}-x.md")),
            format!("# card\n\n```yaml\n---\nstatus: {status}\n---\n```\n{emits}{codes}"),
        )
        .unwrap();
    };
    let entry = |n: u32, seconds: u32, more: &str| {
        format!(
            "### Sprint {n} (2026-11-24, closed)\n- **Dual contract:** artifact pass (`gates.sh` exit 0 in {seconds} s; \
             multi-vCPU: not run (Python gates not run){more}).\n\n"
        )
    };
    let board = |entries: &str| {
        fs::write(
            root.join("BLACKBOARD.md"),
            format!("# BLACKBOARD\n\n## Sprint tail\n\n{entries}"),
        )
        .unwrap();
    };
    let mut passes = String::new();
    let mut gates = |n: u32, seconds: u32| {
        let ran = gates_root_tree(&root);
        let base = script(&root, "gates-base.sh");
        passes.push_str(&format!(
            "exit 0 in {seconds} s\tdefault\t2026-11-24T07:00:00Z\tsprint {n}\th\td\troot={ran}\t\
             rust=all python=0 samples=0\tbase={base}\n"
        ));
        fs::create_dir_all(root.join("store/gates")).unwrap();
        fs::write(root.join("store/gates/passes.tsv"), &passes).unwrap();
    };
    fs::create_dir_all(root.join("crates/a")).unwrap();
    fs::create_dir_all(root.join("store/translation-cache")).unwrap();
    fs::create_dir_all(root.join("store/runs")).unwrap();
    fs::create_dir_all(root.join("signals")).unwrap();
    fs::write(root.join("crates/a/lib.rs"), "fn a() {}\n").unwrap();
    fs::write(root.join(WALK_FIXTURE), "{}\n").unwrap();
    fs::write(root.join("signals/rust-current.json"), "{\"version\": \"0.13\"}\n").unwrap();
    fs::write(
        root.join("signals/0.13.json"),
        "{\"tags\": [{\"name\": \"SESSION_INIT\"}, {\"name\": \"SESSION_COMPLETED\"}]}\n",
    )
    .unwrap();
    git(&root, &["init", "-q"]);
    commit(
        &root,
        &format!("Sprint {} closed: its Signal Report; BLACKBOARD", a - 1),
    );
    card(a, "active");
    commit(&root, &format!("Sprint {a} card: x"));
    fs::write(root.join("crates/a/lib.rs"), format!("fn a() {{ let _ = {a}; }}\n")).unwrap();
    gates(a, 900);
    card(a, "done");
    write_report(&root, a);
    let entry_a = entry(a, 900, "");
    board(&entry_a);
    commit(&root, &format!("Sprint {a}: the work, and its close"));
    let close_a = git(&root, &["rev-parse", "HEAD"]);
    if shape.stray {
        fs::write(root.join("docs/notes-2026-11-24.md"), "# notes\n").unwrap();
        commit(&root, "tidy");
    }
    card(b, "active");
    commit(&root, &format!("Sprint {b} card: y"));
    if let Some(m) = shape.amendment {
        let path = root.join(format!("sprints/sprint-{b}-x.md"));
        let amended = format!("{}\n## amendment\n\nthe probe p\n", fs::read_to_string(&path).unwrap());
        fs::write(&path, amended).unwrap();
        fs::create_dir_all(root.join("probes/p")).unwrap();
        fs::write(root.join("probes/p/p.c"), "int main(void) { return 0; }\n").unwrap();
        if shape.amendment_crates {
            fs::write(root.join("crates/a/extra.rs"), "fn extra() {}\n").unwrap();
        }
        for path in &shape.amendment_paths {
            let path = root.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "amended\n").unwrap();
        }
        commit(&root, &format!("Sprint {m} card amendment: the probe p"));
    }
    fs::write(root.join("crates/a/lib.rs"), format!("fn a() {{ let _ = {b}; }}\n")).unwrap();
    if shape.binary {
        fs::create_dir_all(root.join("store/binaries")).unwrap();
        fs::write(root.join("store/binaries/new.exe"), "MZ\n").unwrap();
    }
    if shape.walked {
        fs::write(root.join(WALK_FIXTURE), "{\"new.exe\": {}}\n").unwrap();
    }
    if shape.record {
        fs::create_dir_all(root.join(RECORD)).unwrap();
        fs::write(
            root.join(RECORD).join("events-000001.jsonl"),
            "{\"seq\":0,\"t\":0.0,\"kind\":\"substrate.RunStarted\",\"payload\":{\"baseline\":{\"harness\":\"w2m\"}}}\n\
             {\"seq\":1,\"t\":0.5,\"kind\":\"SESSION_INIT\",\"payload\":{}}\n\
             {\"seq\":2,\"t\":1.0,\"kind\":\"substrate.RunFinalised\",\"payload\":{}}\n",
        )
        .unwrap();
    }
    if shape.separate_close {
        commit(&root, &format!("Sprint {b}: the work"));
    }
    gates(b, 901);
    fs::write(
        root.join("store/gates/ci.tsv"),
        format!("11\t{close_a}\tsuccess\t2026-11-24T08:00:00Z\tci\n"),
    )
    .unwrap();
    card(b, "done");
    if shape.stale {
        let away = root.join("record-away");
        fs::rename(root.join(RECORD), &away).unwrap();
        write_report(&root, b);
        fs::rename(&away, root.join(RECORD)).unwrap();
    } else {
        write_report(&root, b);
    }
    board(&format!(
        "{}{entry_a}",
        entry(b, 901, &format!("; Sprint {a}'s CI run 11 passed"))
    ));
    if shape.separate_close {
        commit(&root, &format!("Sprint {b} closed: its Signal Report; BLACKBOARD"));
    } else if !shape.uncommitted {
        commit(&root, &format!("Sprint {b}: the work, and its close"));
    }
    (root, close_a)
}

/// Sprint 340 (reviewer-4's eleventh review, High 1): a report made while its close is prepared on the working tree
/// counts the record that close adds and its tag fired, reads the same once the close is committed, and passes
/// `--check` before the commit and after it (plant 1 leaves the working tree's records out: "from 0 records").
#[test]
fn a_report_made_before_its_commit_counts_the_records_the_commit_adds() {
    let shape = Shape {
        first: Some(340),
        record: true,
        emits: true,
        uncommitted: true,
        ..Shape::default()
    };
    let (root, _) = folded("prepared-record", &shape);
    let made = fs::read_to_string(root.join("signal-reports/sprint-341-report.md")).unwrap();
    assert!(made.contains("from 1 records under store/runs/"), "{made}");
    assert!(
        made.contains("or which those commits or the commit that holds this report added"),
        "{made}"
    );
    assert!(
        made.contains("class-* 1 records: SESSION_INIT 1; producers failed 0; not finalised 0"),
        "{made}"
    );
    assert!(made.contains("- `SESSION_INIT`: fired 1 time"), "{made}");
    let (code, stderr) = check_kept(&root);
    assert_eq!((code, stderr.trim()), (Some(0), ""));
    commit(&root, "Sprint 341: the work, and its close");
    let (code, stderr) = check_kept(&root);
    assert_eq!((code, stderr.trim()), (Some(0), ""));
    let _ = fs::remove_dir_all(&root);
}

/// Sprint 340: a committed report that reads "(no record)" over a record its commit keeps differs from its
/// regeneration, and `--check` names it (plant 3, the comparison taken out, passes it). The card's Emits names no
/// tag, so no NOT FIRED line stands in for the comparison.
#[test]
fn a_committed_report_that_misses_its_record_is_named() {
    let shape = Shape {
        first: Some(340),
        record: true,
        stale: true,
        ..Shape::default()
    };
    let (root, _) = folded("stale-record", &shape);
    let kept = fs::read_to_string(root.join("signal-reports/sprint-341-report.md")).unwrap();
    assert!(kept.contains("(no record)"), "{kept}");
    let (code, stderr) = check_kept(&root);
    assert_eq!(
        (code, stderr.trim()),
        (
            Some(1),
            "sprint 341: signal-reports/sprint-341-report.md differs from its regeneration, `signal_report 341`, first \
             at line 3"
        )
    );
    let _ = fs::remove_dir_all(&root);
}

/// Sprint 340: a closed sprint from 340 whose card's Emits names a tag its report shows NOT FIRED is named (plant 2,
/// the rule taken out, passes it).
#[test]
fn a_declared_tag_not_fired_is_named() {
    let shape = Shape {
        first: Some(340),
        emits: true,
        ..Shape::default()
    };
    let (root, _) = folded("not-fired", &shape);
    let (code, stderr) = check_kept(&root);
    assert_eq!(
        (code, stderr.trim()),
        (
            Some(1),
            "sprint 341: its card's Emits names SESSION_INIT, and signal-reports/sprint-341-report.md shows it NOT FIRED"
        )
    );
    let _ = fs::remove_dir_all(&root);
}

/// Sprint 333 (the Decision "A sprint's close folds into its sprint's commit", 2026-11-24): two sprints each closed in
/// its work commit pass `--check`, committed and with the second's close prepared on the working tree before its commit;
/// a stray commit between a close and the next card is still named (plant 2 lets the walk accept any subject), and so,
/// from Sprint 334, is a close apart from its work.
#[test]
fn a_sprint_closes_in_its_work_commit() {
    let (root, _) = folded("folded", &Shape::default());
    let (code, stderr) = check_kept(&root);
    assert_eq!((code, stderr.trim()), (Some(0), ""));
    let _ = fs::remove_dir_all(&root);

    let uncommitted = Shape {
        uncommitted: true,
        ..Shape::default()
    };
    let (root, _) = folded("prepared", &uncommitted);
    let (code, stderr) = check_kept(&root);
    assert_eq!((code, stderr.trim()), (Some(0), ""));
    let _ = fs::remove_dir_all(&root);

    let (root, _) = folded(
        "stray",
        &Shape {
            stray: true,
            ..Shape::default()
        },
    );
    let tidy = git(&root, &["log", "-1", "--format=%h", "--abbrev=9", "--grep=^tidy$"]);
    let (code, stderr) = check_kept(&root);
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr.trim(),
        format!(
            "sprint 334: commit {tidy} \"tidy\" stands between its card and the next card, and is neither its work nor \
             its close"
        )
    );
    let _ = fs::remove_dir_all(&root);

    let (root, _) = folded(
        "separate",
        &Shape {
            separate_close: true,
            ..Shape::default()
        },
    );
    let close = git(&root, &["rev-parse", "--short=9", "HEAD"]);
    let (code, stderr) = check_kept(&root);
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr.trim(),
        format!(
            "sprint 335: commit {close} \"Sprint 335 closed: its Signal Report; BLACKBOARD\" closes it apart from its \
             work, and from Sprint 334 a sprint closes in its work commit"
        )
    );
    let _ = fs::remove_dir_all(&root);
}

/// Sprint 349 (reviewer-4's eighteenth review, Low 1): a commit "Sprint N card amendment: ..." inside sprint N that
/// changes card N and a probe passes the walk, with the close committed and with it prepared on the working tree, and is
/// taken for neither the work nor the close (plant 1, the rule taken out, names it "stands between its card and the next
/// card"); an amendment that names another sprint is named as before.
#[test]
fn a_card_amendment_of_its_card_and_a_probe_passes_inside_its_sprint() {
    let amended = Shape {
        amendment: Some(335),
        ..Shape::default()
    };
    let (root, _) = folded("amendment", &amended);
    let (code, stderr) = check_kept(&root);
    assert_eq!((code, stderr.trim()), (Some(0), ""));
    let _ = fs::remove_dir_all(&root);

    let (root, _) = folded(
        "amendmentprepared",
        &Shape {
            uncommitted: true,
            ..amended
        },
    );
    let (code, stderr) = check_kept(&root);
    assert_eq!((code, stderr.trim()), (Some(0), ""));
    let _ = fs::remove_dir_all(&root);

    let (root, _) = folded(
        "amendmentother",
        &Shape {
            amendment: Some(336),
            ..Shape::default()
        },
    );
    let other = git(
        &root,
        &["log", "-1", "--format=%h", "--abbrev=9", "--grep=card amendment"],
    );
    let (code, stderr) = check_kept(&root);
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr.trim(),
        format!(
            "sprint 335: commit {other} \"Sprint 336 card amendment: the probe p\" stands between its card and the next \
             card, and is neither its work nor its close"
        )
    );
    let _ = fs::remove_dir_all(&root);
}

/// Sprint 349: a card amendment that also changes a path outside its card and `probes/` is named by that path (plant 2,
/// the path limit taken out, passes it).
#[test]
fn a_card_amendment_that_changes_another_path_is_named() {
    let (root, _) = folded(
        "amendmentcrates",
        &Shape {
            amendment: Some(335),
            amendment_crates: true,
            ..Shape::default()
        },
    );
    let amendment = git(
        &root,
        &["log", "-1", "--format=%h", "--abbrev=9", "--grep=card amendment"],
    );
    let (code, stderr) = check_kept(&root);
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr.trim(),
        format!(
            "sprint 335: commit {amendment} \"Sprint 335 card amendment: the probe p\" amends its card and changes \
             crates/a/extra.rs, outside card 335 and probes/"
        )
    );
    let _ = fs::remove_dir_all(&root);
}

/// The laptop's paths a laptop card's amendment changes (Sprint 377): the executor, the sender and a game's state.
const LAPTOP_AMENDED: [&str; 3] = [
    "tools/windows_machine/run-job.ps1",
    "crates/windows_machine/src/lib.rs",
    "games/g/state.json",
];

/// Sprint 377 (the fifty-first review's follow-up): a card amendment of a laptop card, one whose exit codes name a
/// `target/release/windows_machine` job, passes when it changes the executor, the sender or a game's declared state
/// beside its card, committed and with the close prepared on the working tree; the same amendment touching
/// `crates/w2m_vm/` is named by that path (plant 4), and a card that names no laptop job is held to its card and
/// `probes/` as before.
#[test]
fn a_laptop_cards_amendment_of_the_laptops_paths_passes() {
    let laptop = || Shape {
        amendment: Some(335),
        laptop_card: true,
        amendment_paths: LAPTOP_AMENDED.to_vec(),
        ..Shape::default()
    };
    let (root, _) = folded("laptopamendment", &laptop());
    let (code, stderr) = check_kept(&root);
    assert_eq!((code, stderr.trim()), (Some(0), ""));
    let _ = fs::remove_dir_all(&root);

    let (root, _) = folded(
        "laptopamendmentprepared",
        &Shape {
            uncommitted: true,
            ..laptop()
        },
    );
    let (code, stderr) = check_kept(&root);
    assert_eq!((code, stderr.trim()), (Some(0), ""));
    let _ = fs::remove_dir_all(&root);

    let mut vm = LAPTOP_AMENDED.to_vec();
    vm.push("crates/w2m_vm/src/lib.rs");
    let (root, _) = folded(
        "laptopamendmentvm",
        &Shape {
            amendment_paths: vm,
            ..laptop()
        },
    );
    let amendment = git(
        &root,
        &["log", "-1", "--format=%h", "--abbrev=9", "--grep=card amendment"],
    );
    let (code, stderr) = check_kept(&root);
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr.trim(),
        format!(
            "sprint 335: commit {amendment} \"Sprint 335 card amendment: the probe p\" amends its card and changes \
             crates/w2m_vm/src/lib.rs, outside card 335, probes/ and the laptop's paths (tools/windows_machine/, \
             crates/windows_machine/, games/<game>/state.json)"
        )
    );
    let _ = fs::remove_dir_all(&root);

    let (root, _) = folded(
        "notlaptopamendment",
        &Shape {
            laptop_card: false,
            amendment_paths: vec![LAPTOP_AMENDED[0]],
            ..laptop()
        },
    );
    let amendment = git(
        &root,
        &["log", "-1", "--format=%h", "--abbrev=9", "--grep=card amendment"],
    );
    let (code, stderr) = check_kept(&root);
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr.trim(),
        format!(
            "sprint 335: commit {amendment} \"Sprint 335 card amendment: the probe p\" amends its card and changes \
             tools/windows_machine/run-job.ps1, outside card 335 and probes/"
        )
    );
    let _ = fs::remove_dir_all(&root);
}

/// Sprint 333 (reviewer-4's Low 1 at its fifth review, Sprint 331): a sprint whose change adds a file under
/// `store/binaries/` and leaves the walk fixture unchanged is named, committed or prepared on the working tree; with
/// the fixture changed it passes (plant 3, the rule taken out, passes the binary alone).
#[test]
fn a_binary_added_needs_its_walk_fixture_entry() {
    let alone = Shape {
        binary: true,
        ..Shape::default()
    };
    let named = format!(
        "sprint 335: its change adds store/binaries/new.exe and leaves {WALK_FIXTURE} unchanged, so image_walk's count \
         fails"
    );
    let (root, _) = folded("binary", &alone);
    let (code, stderr) = check_kept(&root);
    assert_eq!((code, stderr.trim()), (Some(1), named.as_str()));
    let _ = fs::remove_dir_all(&root);

    let (root, _) = folded(
        "binaryprepared",
        &Shape {
            uncommitted: true,
            ..alone
        },
    );
    let (code, stderr) = check_kept(&root);
    assert_eq!((code, stderr.trim()), (Some(1), named.as_str()));
    let _ = fs::remove_dir_all(&root);

    let (root, _) = folded(
        "binarywalked",
        &Shape {
            binary: true,
            walked: true,
            ..Shape::default()
        },
    );
    let (code, stderr) = check_kept(&root);
    assert_eq!((code, stderr.trim()), (Some(0), ""));
    let _ = fs::remove_dir_all(&root);
}

/// Sprint 200's plants 1 and 2 (reviewer-2's Medium 4 of its review of 196-198): from Sprint 199 a tail entry's quoted
/// `sed` commands are run through edit_guard's classifier: a quoted `a\` insertion the entry does not own as a
/// multi-line edit fails, named; owned, it passes; a one-line substitution passes.
#[test]
fn a_quoted_multi_line_sed_must_be_owned() {
    let not_run = "multi-vCPU: not run (Python gates not run)";
    let stages = "rust=all python=0 samples=0";
    let (code, stderr) = staged(
        "unowned",
        &format!("{not_run}; one-line `sed -i '' '5a\\ x' f.rs`"),
        stages,
        "",
    );
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr.trim(),
        "sprint 199: its tail entry quotes `sed -i '' '5a\\ x' f.rs`, a multi-line edit (sed -i with an a, i or c \
         command: a multi-line edit goes through the Edit tool), and owns no breach"
    );
    let owned = format!("{not_run}; `sed -i '' '5a\\ x' f.rs`, a multi-line edit outside the Edit tool");
    assert_eq!(staged("owned", &owned, stages, "").0, Some(0));
    let one_line = format!("{not_run}; `sed -i '' '3s/a/b/' f.rs`");
    assert_eq!(staged("oneline", &one_line, stages, "").0, Some(0));
}
