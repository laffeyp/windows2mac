// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The exit ledger's judge (Sprint 206; reviewer-2's High 1 and Medium 1 of its review of Sprints 199-205): the frozen
//! Python host's batch run and the Rust host's, each process's host exits to mono's load, compared row by row. The
//! frozen host's ledger is a fixture (`build/exit_ledger/python.tsv` and a child's `.<pid>` beside it, pinned by
//! `store/exit_ledger/python.sha256`; WORKING_AGREEMENT gives its generator); `scripts/gates.sh` writes the Rust host's
//! (`w2m-vm --exit-ledger build/exit_ledger/rust.tsv`). The first difference of each process is held to
//! `store/exit_ledger/first-difference-2026-10-28.tsv`, so a change in where the hosts part fails by name until the
//! file is rewritten with the card that moved it.
//!
//! The last row is the stop, mono's LoadLibrary, which the fixture records at `_load_dll` before the request's answer
//! exists: it is compared without its site and answer columns.

use std::fs;
use std::path::{Path, PathBuf};

const HELD: &str = "store/exit_ledger/first-difference-2026-10-28.tsv";
const PINS: &str = "store/exit_ledger/python.sha256";
const COLUMNS: [&str; 13] = [
    "seq", "kind", "site", "executed", "warp", "op", "a0", "a1", "a2", "a3", "second", "result", "written",
];
/// The stop row's columns that the fixture cannot know: the site, the second, the result and the written ranges.
const UNKNOWN_AT_STOP: [usize; 4] = [2, 10, 11, 12];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn rows(path: &Path) -> Vec<Vec<String>> {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .lines()
        .map(|l| l.split('\t').map(str::to_string).collect())
        .collect()
}

/// For each column, the first row at which `rust` departs from `python` there, one line of the held file each
/// (process, column, row, the frozen host's value, the Rust host's, the row's kind), then the row counts: each column
/// apart, so a difference in one (the thunks' numbering in `site`) does not hide another (the count in `executed`).
fn first_differences(process: &str, python: &[Vec<String>], rust: &[Vec<String>]) -> Vec<String> {
    let last = python.len().saturating_sub(1);
    let cell = |row: &Vec<String>, c: usize| row.get(c).map_or("-", |s| s).to_string();
    let mut out: Vec<String> = (1..COLUMNS.len())
        .map(|c| {
            python
                .iter()
                .zip(rust)
                .enumerate()
                .find(|(i, (p, r))| {
                    !(*i == last && UNKNOWN_AT_STOP.contains(&c))
                        && p.get(c).is_none_or(|v| v != "*")
                        && p.get(c) != r.get(c)
                })
                .map_or(format!("{process}\t{}\tnone", COLUMNS[c]), |(i, (p, r))| {
                    format!(
                        "{process}\t{}\t{i}\t{}\t{}\t{}",
                        COLUMNS[c],
                        cell(p, c),
                        cell(r, c),
                        cell(p, 1)
                    )
                })
        })
        .collect();
    out.push(format!("{process}\trows\t{}\t{}", python.len(), rust.len()));
    out
}

/// The fixture's ledgers by process ("main", or the child's pid), each held to its pinned sha256: the first run's,
/// with the cells where the second run (`python-repeat.tsv`) differs set to "*", the varying cells (the review's rule:
/// a cell varies only where the Python's own runs differ; FILE_STAT's written times, which reading a file moves).
fn fixture(root: &Path) -> Option<Vec<(String, Vec<Vec<String>>)>> {
    let pins = fs::read_to_string(root.join(PINS)).unwrap();
    let mut files = std::collections::BTreeMap::new();
    for line in pins.lines().filter(|l| !l.starts_with('#')) {
        let (sha, name) = line.split_once("  ").unwrap();
        let path = root.join("build/exit_ledger").join(name);
        let Ok(bytes) = fs::read(&path) else {
            return None;
        };
        assert_eq!(
            guest_runtime::sha256::hex(&bytes),
            sha,
            "{} is not the pinned frozen-host fixture",
            path.display()
        );
        files.insert(name.to_string(), rows(&path));
    }
    let mut out = Vec::new();
    for (name, first) in &files {
        let Some(suffix) = name.strip_prefix("python.tsv") else {
            continue;
        };
        let repeat = &files[&format!("python-repeat.tsv{suffix}")];
        assert_eq!(
            first.len(),
            repeat.len(),
            "the two frozen-host runs of {name} differ in length"
        );
        let marked = first
            .iter()
            .zip(repeat)
            .map(|(a, b)| {
                a.iter()
                    .zip(b)
                    .map(|(x, y)| if x == y { x.clone() } else { "*".into() })
                    .collect()
            })
            .collect();
        out.push((suffix.strip_prefix('.').unwrap_or("main").to_string(), marked));
    }
    Some(out)
}

#[test]
fn each_process_parts_from_the_frozen_host_where_held() {
    let root = root();
    // the fixture and the Rust ledger exist where the gates made them (macOS, the depot); elsewhere nothing to judge
    let Some(processes) = fixture(&root) else {
        return;
    };
    if !root.join("build/exit_ledger/rust.tsv").is_file() {
        return;
    }
    let mut found = Vec::new();
    for (process, python) in processes {
        let rust = if process == "main" {
            root.join("build/exit_ledger/rust.tsv")
        } else {
            root.join(format!("build/exit_ledger/rust.tsv.{process}"))
        };
        found.extend(first_differences(&process, &python, &rows(&rust)));
    }
    println!("{}", found.join("\n"));
    let held: Vec<String> = fs::read_to_string(root.join(HELD))
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(str::to_string)
        .collect();
    assert_eq!(
        found, held,
        "where the Rust host parts from the frozen host moved from {HELD}"
    );
}

#[test]
fn a_difference_is_named_by_its_row_and_column() {
    let row = |s: &str| s.split('\t').map(str::to_string).collect::<Vec<_>>();
    let python = vec![
        row("0\thvc 1\t10\t27\t0\t-\t-\t-\t-\t-\t-\t-\t-"),
        row("1\thvc 8\t20\t30\t0\tf\t1\t2\t3\t4\t0\t0\t-"),
    ];
    let mut rust = python.clone();
    let found = |rust: &[Vec<String>]| first_differences("main", &python, rust);
    assert!(
        found(&rust)
            .iter()
            .all(|l| l.ends_with("none") || l == "main\trows\t2\t2")
    );
    rust[0][4] = "143".into();
    rust[0][2] = "11".into();
    let lines = found(&rust);
    assert!(lines.contains(&"main\twarp\t0\t0\t143\thvc 1".to_string()), "{lines:?}");
    assert!(lines.contains(&"main\tsite\t0\t10\t11\thvc 1".to_string()), "{lines:?}");
    // the stop row's answer is not compared; a short ledger is named by its count
    let mut rust = python.clone();
    rust[1][11] = "10".into();
    assert!(found(&rust).contains(&"main\tresult\tnone".to_string()));
    rust.pop();
    assert!(found(&rust).contains(&"main\trows\t2\t1".to_string()));
}
