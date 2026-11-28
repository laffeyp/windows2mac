// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 377: no workflow GitHub runs makes a Windows run (the user, 2026-11-27: "Every probe run and game run goes to
//! the Windows laptop, never a GitHub runner"). `windows-probe.yml` and `windows-game.yml` left `.github/workflows/` for
//! `tools/retired-workflows/`, unchanged, outside the folder GitHub reads: "This event will only trigger a workflow run if
//! the workflow file exists on the default branch" (GitHub's events reference, workflow_dispatch, read 2026-11-27).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The folder GitHub reads workflows from.
const WORKFLOWS: &str = ".github/workflows";
/// Where the two Windows workflows were retired to, and the last commit that held them at `WORKFLOWS`.
const RETIRED: &str = "tools/retired-workflows";
const LAST_HELD: &str = "af000b0d4";
const RETIRED_FILES: [&str; 2] = ["windows-probe.yml", "windows-game.yml"];

/// Each way `text`, a workflow's file, could put a job on Windows, by line: a `runs-on` whose value names Windows; a
/// `runs-on` that is an expression (`windows-probe.yml`'s `${{ inputs.runner }}`, its default windows-2025), whose
/// runner the file does not fix; and a job that calls another repository's workflow, whose runner the file does not
/// hold. A `runs-on` written as a list is read from the lines under it.
fn windows_jobs(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut found = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if let Some(value) = trimmed.strip_prefix("runs-on:") {
            let mut value = value.split(" #").next().unwrap_or("").trim().to_string();
            if value.is_empty() {
                // the list form, "runs-on:" then "- label" lines
                value = lines[i + 1..]
                    .iter()
                    .map(|l| l.trim_start())
                    .take_while(|l| l.starts_with("- "))
                    .map(|l| l[2..].trim())
                    .collect::<Vec<_>>()
                    .join(", ");
            }
            if value.to_lowercase().contains("windows") {
                found.push(format!("line {}: runs-on {value} names Windows", i + 1));
            } else if value.contains("${{") {
                found.push(format!(
                    "line {}: runs-on {value} is an expression, and the runner it names is not fixed",
                    i + 1
                ));
            }
        }
        if let Some(called) = trimmed.strip_prefix("uses:")
            && called.contains(".github/workflows/")
            && !called.trim().starts_with("./")
        {
            found.push(format!(
                "line {}: uses {} calls another repository's workflow, whose runner is not held here",
                i + 1,
                called.trim()
            ));
        }
    }
    found
}

/// Every workflow under `.github/workflows/` runs no job on Windows; the two retired workflows stand under
/// `tools/retired-workflows/` as `LAST_HELD` held them. Prints the folder's files.
#[test]
fn no_workflow_github_runs_makes_a_windows_run() {
    let root = root();
    let mut files: Vec<PathBuf> = fs::read_dir(root.join(WORKFLOWS))
        .unwrap_or_else(|e| panic!("{WORKFLOWS}: {e}"))
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_file())
        .collect();
    files.sort();
    let names: Vec<String> = files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    let mut wrong = Vec::new();
    for (path, name) in files.iter().zip(&names) {
        let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("{WORKFLOWS}/{name}: {e}"));
        wrong.extend(
            windows_jobs(&text)
                .into_iter()
                .map(|w| format!("{WORKFLOWS}/{name} {w}")),
        );
    }
    println!("{WORKFLOWS}: {}; Windows jobs: {}", names.join(", "), wrong.len());
    assert!(
        wrong.is_empty(),
        "workflows GitHub runs that make a Windows run:\n{}",
        wrong.join("\n")
    );
    assert!(
        names.iter().any(|n| n == "ci.yml"),
        "{WORKFLOWS} holds no ci.yml: {names:?}"
    );
    for name in RETIRED_FILES {
        assert!(
            !names.iter().any(|n| n == name),
            "{WORKFLOWS}/{name} is back where GitHub runs it"
        );
        let out = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["show", &format!("{LAST_HELD}:{WORKFLOWS}/{name}")])
            .output()
            .unwrap();
        assert!(out.status.success(), "git show {LAST_HELD}:{WORKFLOWS}/{name}");
        assert_eq!(
            fs::read(root.join(RETIRED).join(name)).unwrap_or_else(|e| panic!("{RETIRED}/{name}: {e}")),
            out.stdout,
            "{RETIRED}/{name} is not the file {LAST_HELD} held at {WORKFLOWS}/{name}"
        );
    }
}

/// The reader on the forms a workflow writes a runner in: the retired two (a literal and an expression), ci.yml's
/// Linux label, the list and flow forms, a comment after the label, another repository's workflow and one of this
/// repository's.
#[test]
fn the_reader_names_each_way_to_a_windows_runner() {
    let retired = |name: &str| fs::read_to_string(root().join(RETIRED).join(name)).unwrap();
    assert_eq!(
        windows_jobs(&retired("windows-game.yml")),
        ["line 34: runs-on windows-2025 names Windows"]
    );
    assert_eq!(
        windows_jobs(&retired("windows-probe.yml")),
        ["line 36: runs-on ${{ inputs.runner }} is an expression, and the runner it names is not fixed"]
    );
    assert!(windows_jobs(&fs::read_to_string(root().join(WORKFLOWS).join("ci.yml")).unwrap()).is_empty());
    assert_eq!(
        windows_jobs("jobs:\n  a:\n    runs-on:\n      - self-hosted\n      - Windows\n"),
        ["line 3: runs-on self-hosted, Windows names Windows"]
    );
    assert_eq!(
        windows_jobs("    runs-on: [self-hosted, windows-11]\n"),
        ["line 1: runs-on [self-hosted, windows-11] names Windows"]
    );
    assert!(windows_jobs("    runs-on: ubuntu-24.04  # not windows\n").is_empty());
    assert_eq!(
        windows_jobs("    uses: other/repo/.github/workflows/build.yml@v1\n"),
        [
            "line 1: uses other/repo/.github/workflows/build.yml@v1 calls another repository's workflow, whose runner is \
          not held here"
        ]
    );
    assert!(windows_jobs("    uses: ./.github/workflows/ci.yml\n").is_empty());
    assert!(windows_jobs("      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262\n").is_empty());
}
