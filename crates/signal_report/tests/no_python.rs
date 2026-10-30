// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! No Python in the tree but the read-only kit's (Sprint 226; BLACKBOARD Decision 2026-10-29, "No Python runs", and the
//! user: "the 22% of python into 0"). The frozen Python host, its tests and scripts are at the tag
//! frozen-python-2026-10-29; `sdd-kit-3/` is the kit, never edited, its 13 example and library files marked
//! `linguist-vendored` in `.gitattributes`.

use std::path::Path;
use std::process::Command;

/// The tracked paths ending in `.py`, from `git ls-files` at the repository root.
fn tracked_python(root: &Path) -> Vec<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--", "*.py"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}

/// A workflow line's Python commands: a token (letters, digits, `.`, `_`, `-`) that is a Python interpreter or `pip`,
/// outside the line's `#` comment. `corpus/python-3.14.8` is a path, one token, and not a command.
fn python_commands(line: &str) -> Vec<String> {
    let code = line.split(" #").next().unwrap_or("");
    let code = if code.trim_start().starts_with('#') { "" } else { code };
    code.split(|c: char| !(c.is_ascii_alphanumeric() || "._-".contains(c)))
        .filter(|t| ["python", "python3", "python.exe", "py", "pip", "pip3"].contains(t))
        .map(str::to_string)
        .collect()
}

/// No workflow runs Python and no probe names a Python generator (Sprint 227; reviewer-3's Medium 1 and Low 1 of its
/// review at Sprint 226: `windows-probe.yml` ran a probe's `generate` with `uv run python`, and `windows-game.yml` ran
/// `python -c` and `python -` on the runner, neither a tracked `.py`).
#[test]
fn no_workflow_runs_python_and_no_probe_names_it() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut found = Vec::new();
    for e in std::fs::read_dir(root.join(".github/workflows")).unwrap().flatten() {
        let text = std::fs::read_to_string(e.path()).unwrap();
        for (i, line) in text.lines().enumerate() {
            for cmd in python_commands(line) {
                found.push(format!("{}:{}: {cmd}", e.path().display(), i + 1));
            }
        }
    }
    for e in std::fs::read_dir(root.join("probes")).unwrap().flatten() {
        let Ok(text) = std::fs::read_to_string(e.path().join("probe.json")) else {
            continue;
        };
        if text.contains(".py\"") || text.contains(".py ") {
            found.push(format!("{}: names a .py", e.path().join("probe.json").display()));
        }
    }
    assert!(found.is_empty(), "Python run or named: {found:#?}");
}

#[test]
fn a_python_command_is_named_and_a_path_or_comment_is_not() {
    assert_eq!(python_commands("  tag=$(python -c \"x\")"), ["python"]);
    assert_eq!(python_commands("  uv run --no-dev python gen"), ["python"]);
    assert!(python_commands("  url=$(jq -r .source corpus/python-3.14.8.manifest.json)").is_empty());
    assert!(python_commands("  # once run with python -c").is_empty());
    assert!(python_commands("  zstd -d # python is gone").is_empty());
}

#[test]
fn no_python_is_tracked_outside_the_kit() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let outside: Vec<String> = tracked_python(&root)
        .into_iter()
        .filter(|p| !p.starts_with("sdd-kit-3/"))
        .collect();
    assert!(outside.is_empty(), "Python tracked outside sdd-kit-3/: {outside:#?}");
}
