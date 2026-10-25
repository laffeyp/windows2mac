// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! `design_check [--root DIR]` (Sprint 128). Every plan and design written from 2026-10-24 on states the goal's
//! figures and names the systems that solved the same problem, with sources; one missing either fails the gates
//! (BLACKBOARD Decision 2026-10-24 user, fix (a): "one check before every design or decision lands ... a failure in
//! either check stops it").
//!
//! The re-review of 2026-10-24 found the port's plan and host design each checked only against the step before: the
//! plan's ten steps reached no frame, and the design sized its budget from a test guest's 90,000 exits a second, not
//! from the game's 37,600 system calls a second. It found the design's forms (a global runtime lock, a server round
//! trip per wait, a lock across each block's compile) to be ones QEMU and Wine had already taken out. This binary
//! checks that the figures and the sources are written down; whether the argument made with them holds stays the
//! reviewer's.
//!
//! What it checks:
//! - every file under `docs/` whose name begins `design-`, `solution-`, `recentering-` or `architecture-`, or holds
//!   `plan-`, except the seven that existed before the check (`HISTORY`): a new file is checked whatever date its
//!   name carries;
//! - every `## Decisions` entry of `BLACKBOARD.md` dated 2026-10-24 or later that carries the marker `Architecture:`
//!   (a decision that sets the system's shape, WORKING_AGREEMENT): it must name a checked document under `docs/` that
//!   exists, so the decision lands only with a design that passed.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The first date a decision marked `Architecture:` must point at a checked document (Decision 2026-10-24 user).
const FIRST_CHECKED_DATE: &str = "2026-10-24";
/// The plans and designs written before the check: history, not checked. The list is closed; it never grows.
const HISTORY: [&str; 7] = [
    "design-rust-host-2026-10-23.md",
    "design-several-vcpus-2026-10-17.md",
    "plan-2026-10-09.md",
    "recentering-2026-10-14.md",
    "recentering-2026-10-16-knowable-first.md",
    "rust-port-plan-2026-10-21.md",
    "solution-2026-10-16.md",
];
const DESIGN_PREFIXES: [&str; 4] = ["design-", "solution-", "recentering-", "architecture-"];
const DECISION_MARKER: &str = "Architecture:";
const GOAL_SECTION: &str = "## Against the goal";
const SYSTEMS_SECTION: &str = "## Against the systems that solved it";
/// The goal's figures, each from where it was measured:
/// - 37,600 system calls a second in play: 314 NT and 18 win32k calls a frame at 113 frames a second on the laptop
///   (docs/plan-2026-10-09.md 23-24, the review of 2026-10-16's E3);
/// - 41 million instructions a frame at 112 frames a second (docs/recentering-2026-10-14.md 35);
/// - six threads: the frame's instructions are spread over six busy threads, the busiest 28.9% (the same, 36);
/// - the first scene: 325.4 billion instructions, 62 s on the laptop (the same, 35), the end of milestone B's path.
const GOAL_FIGURES: [&str; 4] = ["37,600", "41 million", "six threads", "first scene"];
/// The systems that solved the problems the host faces, as the re-review names them: QEMU (translation, its regions
/// and lock-free lookup), crosvm and Firecracker (a thread per vCPU, a lock per device), Wine (NT objects and
/// waits), Rosetta 2 and FEX (x86 on arm64, ahead of time and at run time).
const SYSTEMS: [&str; 6] = ["QEMU", "crosvm", "Firecracker", "Wine", "Rosetta 2", "FEX"];

/// A file under `docs/` the check reads: a plan or a design by its name, and not history.
fn is_checked(name: &str) -> bool {
    let plan_or_design = DESIGN_PREFIXES.iter().any(|p| name.starts_with(p)) || name.contains("plan-");
    name.ends_with(".md") && plan_or_design && !HISTORY.contains(&name)
}

/// The lines under a level-two heading, to the next level-one or level-two heading; `None` when the heading is absent.
fn section<'a>(text: &'a str, heading: &str) -> Option<Vec<&'a str>> {
    let mut lines = text.lines().skip_while(|l| l.trim_end() != heading);
    lines.next()?;
    Some(
        lines
            .take_while(|l| !(l.starts_with("# ") || l.starts_with("## ")))
            .collect(),
    )
}

/// `name` stands in `line` as a whole word: no letter, digit or underscore touches either end.
fn names_whole(line: &str, name: &str) -> bool {
    let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    line.match_indices(name)
        .any(|(i, _)| !word(line[..i].chars().next_back()) && !word(line[i + name.len()..].chars().next()))
}

/// A section's lines as items, each a list entry or a paragraph with its wrapped lines joined by a space: a system and
/// its source count together when they stand in one item, however the prose wraps.
fn items(lines: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut open = false;
    for line in lines {
        let t = line.trim();
        let starts_item = t.starts_with("- ")
            || t.starts_with("* ")
            || t.split_once(". ")
                .is_some_and(|(n, _)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
        if t.is_empty() {
            open = false;
        } else if starts_item || !open {
            out.push(t.to_string());
            open = true;
        } else if let Some(last) = out.last_mut() {
            last.push(' ');
            last.push_str(t);
        }
    }
    out
}

/// Whether a source (`http`) follows a whole-word mention of `system` in `item` before the next mention of another
/// system: an item that names two systems must source each (Sprint 129: Rosetta 2's URL removed passed on FEX's).
fn sourced(item: &str, system: &str) -> bool {
    let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    let mentions = |name: &str| -> Vec<usize> {
        item.match_indices(name)
            .filter(|(i, _)| !word(item[..*i].chars().next_back()) && !word(item[i + name.len()..].chars().next()))
            .map(|(i, _)| i)
            .collect()
    };
    let others: Vec<usize> = SYSTEMS
        .iter()
        .filter(|s| **s != system)
        .flat_map(|s| mentions(s))
        .collect();
    mentions(system).into_iter().any(|start| {
        let end = others
            .iter()
            .filter(|o| **o > start)
            .min()
            .copied()
            .unwrap_or(item.len());
        item[start..end].contains("http")
    })
}

/// What one document lacks, one line each; empty when it passes.
fn faults(file: &str, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    match section(text, GOAL_SECTION) {
        None => out.push(format!("{file}: no \"{GOAL_SECTION}\" section")),
        Some(lines) => {
            let body = lines.join("\n").to_lowercase();
            for figure in GOAL_FIGURES {
                if !body.contains(figure) {
                    out.push(format!("{file}: \"{GOAL_SECTION}\" does not state {figure}"));
                }
            }
        }
    }
    match section(text, SYSTEMS_SECTION) {
        None => out.push(format!("{file}: no \"{SYSTEMS_SECTION}\" section")),
        Some(lines) => {
            let items = items(&lines);
            for system in SYSTEMS {
                let named: Vec<&String> = items.iter().filter(|i| names_whole(i, system)).collect();
                if named.is_empty() {
                    out.push(format!("{file}: \"{SYSTEMS_SECTION}\" does not name {system}"));
                } else if !named.iter().any(|i| sourced(i, system)) {
                    out.push(format!("{file}: \"{SYSTEMS_SECTION}\" names {system} with no source"));
                }
            }
        }
    }
    out
}

/// The names of the `docs/` files a text points at (`docs/<name>.md`).
fn docs_named(text: &str) -> Vec<&str> {
    text.match_indices("docs/")
        .filter_map(|(i, _)| {
            let rest = &text[i + 5..];
            let end = rest.find(".md")? + 3;
            let name = &rest[..end];
            name.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
                .then_some(name)
        })
        .collect()
}

/// Each `## Decisions` entry dated from 2026-10-24 that carries the marker and names no checked document that exists.
fn decision_faults(blackboard: &str, docs: &Path) -> Vec<String> {
    let Some(lines) = section(blackboard, "## Decisions") else {
        return vec!["BLACKBOARD.md: no \"## Decisions\" section".to_string()];
    };
    let mut entries: Vec<String> = Vec::new();
    for line in lines {
        if line.starts_with("- **") {
            entries.push(line.to_string());
        } else if let Some(last) = entries.last_mut() {
            last.push('\n');
            last.push_str(line);
        }
    }
    entries
        .iter()
        .filter_map(|e| {
            let date = e.get(4..14)?;
            if date < FIRST_CHECKED_DATE || !e.contains(DECISION_MARKER) {
                return None;
            }
            let points = docs_named(e).iter().any(|n| is_checked(n) && docs.join(n).is_file());
            (!points).then(|| {
                format!(
                    "BLACKBOARD.md: the Decision of {date} marked \"{DECISION_MARKER}\" names no checked plan or design"
                )
            })
        })
        .collect()
}

/// Every checked plan and design under `root/docs`, and every marked Decision, with what each lacks.
fn check(root: &Path) -> Result<(usize, Vec<String>), String> {
    let docs = root.join("docs");
    let mut names: Vec<String> = fs::read_dir(&docs)
        .map_err(|e| format!("{}: {e}", docs.display()))?
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| is_checked(n))
        .collect();
    names.sort();
    let mut all = Vec::new();
    for name in &names {
        let text = fs::read_to_string(docs.join(name)).map_err(|e| format!("docs/{name}: {e}"))?;
        all.extend(faults(&format!("docs/{name}"), &text));
    }
    // the Decisions are part of the check: a tree without its BLACKBOARD fails, never skips them
    let text = fs::read_to_string(root.join("BLACKBOARD.md")).map_err(|e| format!("BLACKBOARD.md: {e}"))?;
    all.extend(decision_faults(&text, &docs));
    Ok((names.len(), all))
}

fn run() -> Result<bool, String> {
    let mut args = std::env::args().skip(1);
    let mut root = PathBuf::from(".");
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = args.next().map(PathBuf::from).ok_or("--root needs a directory")?,
            a => return Err(format!("unknown argument {a}; usage: design_check [--root DIR]")),
        }
    }
    let (checked, found) = check(&root)?;
    for f in &found {
        eprintln!("{f}");
    }
    if found.is_empty() {
        println!("design_check: {checked} plans and designs state the goal and the systems");
    }
    Ok(found.is_empty())
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("design_check: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_and_designs_are_checked_whatever_their_date_and_history_is_not() {
        assert!(is_checked("design-rust-host-2026-10-24.md"));
        assert!(is_checked("design-x-2026-10-23.md"));
        assert!(is_checked("rust-port-plan-2026-10-24.md"));
        assert!(is_checked("architecture-nt-edge-2026-11-02.md"));
        assert!(is_checked("solution-2026-11-01.md"));
        assert!(!is_checked("design-rust-host-2026-10-23.md"));
        assert!(!is_checked("plan-2026-10-09.md"));
        assert!(!is_checked("review-2026-10-24.md"));
    }

    #[test]
    fn a_section_runs_to_the_next_heading_of_level_one_or_two() {
        let text = "intro\n## Against the goal\none\n### sub\ntwo\n## Next\nthree\n";
        assert_eq!(section(text, GOAL_SECTION), Some(vec!["one", "### sub", "two"]));
        assert_eq!(section(text, SYSTEMS_SECTION), None);
    }

    #[test]
    fn an_item_runs_over_its_wrapped_lines_and_ends_at_a_blank_line_or_the_next_entry() {
        let lines = [
            "- **QEMU** took its lock out",
            "  (https://example.org/qemu).",
            "- Rosetta",
            "  2 translates ahead of time.",
            "",
            "A paragraph",
            "wrapped.",
            "1. A numbered entry.",
        ];
        assert_eq!(
            items(&lines),
            [
                "- **QEMU** took its lock out (https://example.org/qemu).",
                "- Rosetta 2 translates ahead of time.",
                "A paragraph wrapped.",
                "1. A numbered entry.",
            ]
        );
    }

    #[test]
    fn each_system_in_a_shared_item_needs_its_own_source() {
        let item = "- Rosetta 2 translates ahead of time (WWDC20). FEX caches code (https://fex-emu.com/FEX-2609/).";
        assert!(!sourced(item, "Rosetta 2"));
        assert!(sourced(item, "FEX"));
        let both = "- Rosetta 2 (https://a) and FEX (https://b).";
        assert!(sourced(both, "Rosetta 2") && sourced(both, "FEX"));
    }

    #[test]
    fn a_system_inside_a_longer_word_is_not_named() {
        assert!(names_whole("- Wine's server: https://x", "Wine"));
        assert!(names_whole("FEX-Emu: https://x", "FEX"));
        assert!(!names_whole("the wineserver and Winelib", "Wine"));
        assert!(!names_whole("FEXCore", "FEX"));
    }

    #[test]
    fn the_documents_a_decision_names_are_read_from_its_text() {
        assert_eq!(
            docs_named("see `docs/design-a-2026-10-24.md` and docs/plan-2026-10-09.md, not docs/x y.md"),
            ["design-a-2026-10-24.md", "plan-2026-10-09.md"]
        );
    }
}
