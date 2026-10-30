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

/// A workflow line's Python commands: a token (letters, digits, `.`, `_`, `-`) that is a Python interpreter (`python`,
/// `python3`, a versioned `python3.12`, `python.exe`, `py`), `pip`, or the `setup-python` action, outside the line's `#`
/// comment. `corpus/python-3.14.8` is a path, one token, and not a command (reviewer-3's Low 1 of its review at 227:
/// the first form missed a versioned interpreter and `setup-python`).
fn python_commands(line: &str) -> Vec<String> {
    let code = line.split(" #").next().unwrap_or("");
    let code = if code.trim_start().starts_with('#') { "" } else { code };
    let versioned = |t: &str| {
        t.strip_prefix("python")
            .is_some_and(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_digit() || c == '.'))
    };
    code.split(|c: char| !(c.is_ascii_alphanumeric() || "._-".contains(c)))
        .filter(|t| ["python", "python.exe", "py", "pip", "pip3", "setup-python"].contains(t) || versioned(t))
        .map(str::to_string)
        .collect()
}

/// A probe's string values that name a Python file: every string in its JSON, at any depth, ending in `.py`.
fn python_names(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::String(s) if s.split_whitespace().any(|w| w.ends_with(".py")) => vec![s.clone()],
        serde_json::Value::Array(a) => a.iter().flat_map(python_names).collect(),
        serde_json::Value::Object(o) => o.values().flat_map(python_names).collect(),
        _ => Vec::new(),
    }
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
        let probe: serde_json::Value = serde_json::from_str(&text).unwrap();
        for name in python_names(&probe) {
            found.push(format!("{}: names {name:?}", e.path().join("probe.json").display()));
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
    assert_eq!(python_commands("  python3.12 -m x"), ["python3.12"]);
    assert_eq!(python_commands("    uses: actions/setup-python@v5"), ["setup-python"]);
    let probe = serde_json::json!({"generate": "gen.py --cases", "dll": {"src": "x/b.py"}, "source": "a.c"});
    assert_eq!(
        python_names(&probe).len(),
        2,
        "a .py at any depth, at the end of a value or before a space"
    );
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
