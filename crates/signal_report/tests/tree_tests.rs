// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Every test that reads beyond its own crate runs in every gates run (Sprint 322; Deferred, Sprint 321). The gates
//! select tests by the crates a change reaches (`rdeps()`), which skips a test of another crate that reads the changed
//! one's source: Sprint 320's two upper time bounds in `w2m_vm` passed its gates and failed CI on `hypervisor`'s
//! `no_wall_clock_bound`. `scripts/tree-tests.txt` lists such tests and `scripts/test-filter.sh` adds them to every
//! selection; this test finds each test file that reaches the crates directory (`join("..")` on its crate's own) or the
//! whole repository (`git ls-files`), and fails naming any the list does not select.

use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every `.rs` file under `dir`, in order.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for p in paths {
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Whether a test file's text reads beyond its crate: the crates directory, or every tracked file.
fn reaches_beyond(text: &str) -> bool {
    let crates_dir = ["join(", "\"..\")"].concat();
    let tracked = ["\"ls-files", "\""].concat();
    text.contains(&crates_dir) || text.contains(&tracked)
}

/// The list's filter lines, comments and blanks left out.
fn listed(root: &Path) -> Vec<String> {
    fs::read_to_string(root.join("scripts/tree-tests.txt"))
        .unwrap()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(String::from)
        .collect()
}

/// Whether the list selects test file `file` of crate `krate`: its own binary, or its module in the crate's merged
/// `tests/all.rs` binary.
fn selected(lines: &[String], krate: &str, file: &str) -> bool {
    let own = format!("binary_id({krate}::{file})");
    let merged = format!("binary_id({krate}::all) & test(/^{file}::/)");
    lines.iter().any(|l| *l == own || *l == merged)
}

#[test]
fn every_tree_test_runs_on_every_change() {
    let root = root();
    let lines = listed(&root);
    let mut missing = Vec::new();
    let mut crates: Vec<PathBuf> = fs::read_dir(root.join("crates"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    crates.sort();
    for krate in crates {
        let name = krate.file_name().unwrap().to_string_lossy().to_string();
        let mut files = Vec::new();
        rust_files(&krate.join("tests"), &mut files);
        for f in files {
            let stem = f.file_stem().unwrap().to_string_lossy().to_string();
            // this file names the forms it looks for; it reads only the list and the tests' text
            if name == "signal_report" && stem == "tree_tests" {
                continue;
            }
            if reaches_beyond(&fs::read_to_string(&f).unwrap()) && !selected(&lines, &name, &stem) {
                missing.push(format!("crates/{name}/tests/{stem}.rs"));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "tests that read beyond their crate and are not in scripts/tree-tests.txt: {missing:?}"
    );
}

#[test]
fn the_forms_are_read_and_a_merged_module_is_selected() {
    assert!(reaches_beyond(&["let c = here.", "join(", "\"..\");"].concat()));
    assert!(reaches_beyond(&["git(root, &[", "\"ls-files", "\", \"--\"])"].concat()));
    assert!(!reaches_beyond("let r = here.join(\"../..\").join(\"store\");"));
    let lines = vec!["binary_id(w2m_vm::all) & test(/^layout_constants::/)".to_string()];
    assert!(selected(&lines, "w2m_vm", "layout_constants"));
    assert!(!selected(&lines, "w2m_vm", "vcpu_pages"));
}
