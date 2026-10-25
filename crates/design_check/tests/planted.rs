// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 128's expected value and planted faults: the binary run on a scratch `docs/` and `BLACKBOARD.md`.

use std::fs;
use std::process::Command;

const GOAL: &str = "## Against the goal\n\nIn play the game makes 37,600 system calls a second and retires 41 million \
                    instructions a frame over six threads; milestone B ends at the first scene.\n";
const SYSTEMS: &str = "## Against the systems that solved it\n\n\
                       - QEMU's regions: https://example.org/qemu\n\
                       - crosvm: https://example.org/crosvm\n\
                       - Firecracker: https://example.org/fc\n\
                       - Wine's ntsync: https://example.org/wine\n\
                       - Rosetta 2: https://example.org/rosetta\n\
                       - FEX: https://example.org/fex\n";

fn run(name: &str, docs: &[(&str, String)], decisions: &str) -> (Option<i32>, String, String) {
    let root = std::env::temp_dir().join(format!("w2m-design-check-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("docs")).unwrap();
    for (file, text) in docs {
        fs::write(root.join("docs").join(file), text).unwrap();
    }
    fs::write(
        root.join("BLACKBOARD.md"),
        format!("# B\n\n## Decisions\n\n{decisions}\n## Built\n\n- **2026-10-25** — Architecture: none here.\n"),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_design_check"))
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(&root);
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
        String::from_utf8_lossy(&out.stderr).trim().to_string(),
    )
}

fn passing() -> String {
    format!("# Host\n\n{GOAL}\n{SYSTEMS}")
}

#[test]
fn a_design_with_every_figure_and_every_source_passes() {
    let (code, stdout, stderr) = run("pass", &[("design-host-2026-10-24.md", passing())], "");
    assert_eq!((code, stderr.as_str()), (Some(0), ""));
    assert_eq!(
        stdout,
        "design_check: 1 plans and designs state the goal and the systems"
    );
}

#[test]
fn a_plan_with_no_goal_section_fails_naming_it() {
    let (code, _, stderr) = run(
        "no-goal",
        &[("rust-port-plan-2026-10-24.md", format!("# Plan\n\n{SYSTEMS}"))],
        "",
    );
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr,
        "docs/rust-port-plan-2026-10-24.md: no \"## Against the goal\" section"
    );
}

#[test]
fn a_design_missing_one_figure_fails_naming_it() {
    let goal = GOAL.replace("37,600", "about forty thousand");
    let (code, _, stderr) = run(
        "no-figure",
        &[("design-host-2026-10-25.md", format!("{goal}\n{SYSTEMS}"))],
        "",
    );
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr,
        "docs/design-host-2026-10-25.md: \"## Against the goal\" does not state 37,600"
    );
}

#[test]
fn a_system_named_with_no_source_fails_naming_it() {
    let systems = SYSTEMS.replace(
        "Rosetta 2: https://example.org/rosetta",
        "Rosetta 2 translates ahead of time",
    );
    let (code, _, stderr) = run(
        "no-source",
        &[("design-host-2026-10-24.md", format!("{GOAL}\n{systems}"))],
        "",
    );
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr,
        "docs/design-host-2026-10-24.md: \"## Against the systems that solved it\" names Rosetta 2 with no source"
    );
}

#[test]
fn a_new_design_named_with_an_old_date_is_still_checked() {
    let (code, _, stderr) = run("old-name", &[("design-host-2026-10-20.md", format!("{GOAL}\n"))], "");
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr,
        "docs/design-host-2026-10-20.md: no \"## Against the systems that solved it\" section"
    );
}

#[test]
fn history_and_documents_that_are_not_plans_or_designs_are_not_checked() {
    let faulty = format!("{GOAL}\n");
    let (code, stdout, stderr) = run(
        "exempt",
        &[
            ("design-rust-host-2026-10-23.md", faulty.clone()),
            ("plan-2026-10-09.md", faulty.clone()),
            ("review-2026-10-24.md", faulty),
        ],
        "",
    );
    assert_eq!((code, stderr.as_str()), (Some(0), ""));
    assert_eq!(
        stdout,
        "design_check: 0 plans and designs state the goal and the systems"
    );
}

#[test]
fn a_marked_decision_must_name_a_checked_design_that_exists() {
    let decisions = "- **2026-10-23 agent-resolved** — Architecture: before the check, not read.\n\
                     - **2026-10-24 agent-resolved** — Architecture: the host's locks; see docs/design-host-2026-10-24.md.\n\
                     - **2026-10-24 agent-resolved** — A name, not a shape.\n\
                     - **2026-10-25 agent-resolved** — Architecture: the NT edge,\n  argued in docs/plan-2026-10-09.md.\n\
                     - **2026-10-26 agent-resolved** — Architecture: see docs/design-missing-2026-10-26.md.\n";
    let (code, _, stderr) = run("decisions", &[("design-host-2026-10-24.md", passing())], decisions);
    assert_eq!(code, Some(1));
    assert_eq!(
        stderr,
        "BLACKBOARD.md: the Decision of 2026-10-25 marked \"Architecture:\" names no checked plan or design\n\
         BLACKBOARD.md: the Decision of 2026-10-26 marked \"Architecture:\" names no checked plan or design"
    );
}

#[test]
fn a_tree_without_its_blackboard_fails() {
    let root = std::env::temp_dir().join(format!("w2m-design-check-{}-no-blackboard", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("docs")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_design_check"))
        .arg("--root")
        .arg(&root)
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(&root);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).starts_with("design_check: BLACKBOARD.md: "));
}
