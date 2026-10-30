// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! No Python in the tree but the read-only kit's (Sprint 226; BLACKBOARD Decision 2026-10-29, "No Python runs", and the
//! user: "the 22% of python into 0"). The frozen Python host, its tests and scripts are at the tag
//! frozen-python-2026-10-29; `sdd-kit-3/` is the kit, never edited, its 13 example and library files marked
//! `linguist-vendored` in `.gitattributes`.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

/// The tag that keeps the Python removed in Sprint 226, and the tree's index of it (Sprint 229).
const TAG: &str = "frozen-python-2026-10-29";
const REMOVED: &str = "store/frozen_python/removed-2026-10-29.tsv";

/// `git` run at the repository root, its standard output; `quiet` is the exit status that means "none" (`git grep`'s
/// 1), read as no output.
fn git(root: &Path, args: &[&str], quiet: Option<i32>) -> String {
    let out = Command::new("git").arg("-C").arg(root).args(args).output().unwrap();
    if quiet.is_some() && out.status.code() == quiet {
        return String::new();
    }
    assert!(
        out.status.success(),
        "git {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// The tracked paths ending in `.py`, from `git ls-files` at the repository root.
fn tracked_python(root: &Path) -> Vec<String> {
    git(root, &["ls-files", "--", "*.py"], None)
        .lines()
        .map(str::to_string)
        .collect()
}

/// The `.py` files at the tag outside `sdd-kit-3/`, path to blob id, from `git ls-tree`.
fn tagged_python(root: &Path) -> BTreeMap<String, String> {
    git(root, &["ls-tree", "-r", "--format=%(objectname)\t%(path)", TAG], None)
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .filter(|(_, path)| path.ends_with(".py") && !path.starts_with("sdd-kit-3/"))
        .map(|(blob, path)| (path.to_string(), blob.to_string()))
        .collect()
}

/// The index's rows, path to blob id; `#` lines are its header.
fn removed(root: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(root.join(REMOVED))
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| {
            let (blob, path) = l
                .split_once('\t')
                .unwrap_or_else(|| panic!("{REMOVED}: not `blob\\tpath`: {l}"));
            (path.to_string(), blob.to_string())
        })
        .collect()
}

/// Each repository-relative `.py` path a tracked file under `store/` names, with the first file naming it: `git grep`
/// over the tracked files for a token of letters, digits, `_`, `.`, `/` and `-` ending in `.py` and not followed by a
/// letter, digit or `_` (so `x.pyc` is not one). A bare name (`run_frontier.py`) and an absolute path name no
/// repository path and are left out, and so is the index, which names every path.
fn cited_python(root: &Path) -> BTreeMap<String, String> {
    let index = format!(":(exclude){REMOVED}");
    let found = git(
        root,
        &[
            "grep",
            "-z",
            "-oIE",
            "[A-Za-z0-9_./-]+\\.py([^A-Za-z0-9_]|$)",
            "--",
            "store",
            &index,
        ],
        Some(1),
    );
    let mut cited = BTreeMap::new();
    // `-z`: each match is a line "file\0token", the token ending in `.py` or in the one character after it
    for line in found.lines() {
        let (file, token) = line
            .split_once('\0')
            .unwrap_or_else(|| panic!("git grep -z: no file in {line:?}"));
        let mut chars = token.chars();
        if !token.ends_with(".py") {
            chars.next_back();
        }
        let path = chars.as_str();
        if path.contains('/') && !path.starts_with('/') {
            cited.entry(path.to_string()).or_insert_with(|| file.to_string());
        }
    }
    cited
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

/// The index lists every `.py` the tag holds outside the kit, with its blob id, and nothing else (Sprint 229;
/// reviewer-3's Medium 3 of its review at 226: records the tree cites were removed, and only a checkout of the tag
/// read them).
#[test]
fn the_removed_index_is_the_tags_python() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let (index, tag) = (removed(&root), tagged_python(&root));
    let mut wrong = Vec::new();
    for (path, blob) in &tag {
        match index.get(path) {
            None => wrong.push(format!("{path}: at {TAG}, not in {REMOVED}")),
            Some(b) if b != blob => wrong.push(format!("{path}: {REMOVED} has blob {b}, {TAG} {blob}")),
            Some(_) => {}
        }
    }
    wrong.extend(
        index
            .keys()
            .filter(|p| !tag.contains_key(*p))
            .map(|p| format!("{p}: in {REMOVED}, not at {TAG}")),
    );
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Every repository-relative `.py` path a tracked record under `store/` names is a row of the index, so a citation
/// reads its file by `git show frozen-python-2026-10-29:<path>` (Sprint 229).
#[test]
fn every_python_path_a_record_names_is_in_the_index() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let index = removed(&root);
    let cited = cited_python(&root);
    let unlisted: Vec<String> = cited
        .iter()
        .filter(|(path, _)| !index.contains_key(*path))
        .map(|(path, file)| format!("{file} names {path}, which {REMOVED} does not list"))
        .collect();
    assert!(unlisted.is_empty(), "{unlisted:#?}");
    println!("{} paths named under store/, each in {REMOVED}", cited.len());
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
