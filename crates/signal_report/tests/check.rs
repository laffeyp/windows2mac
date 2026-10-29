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
        "sprint 199: its tail entry quotes `sed -i '' '5a\\ x' f.rs`, a multi-line edit (sed -i with an a\\, i\\ or c\\ \
         command: a multi-line edit goes through the Edit tool), and owns no breach"
    );
    let owned = format!("{not_run}; `sed -i '' '5a\\ x' f.rs`, a multi-line edit outside the Edit tool");
    assert_eq!(staged("owned", &owned, stages, "").0, Some(0));
    let one_line = format!("{not_run}; `sed -i '' '3s/a/b/' f.rs`");
    assert_eq!(staged("oneline", &one_line, stages, "").0, Some(0));
}
