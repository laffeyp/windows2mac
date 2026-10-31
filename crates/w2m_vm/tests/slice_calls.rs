// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Census row D7 (Sprint 278): the slice calls over guest memory, `as_slice()` and `as_mut_slice()`, left in each file
//! of `crates/w2m_vm/src/`, each sound only while every vCPU that may write the memory is stopped
//! (`docs/design-guest-memory-on-several-vcpus-2026-10-31.md`); and (Sprint 279, reviewer-3's Low 1 at 278) the views
//! still built from such a slice, `ConcurrentGuestMemory::new(`, outside the view's own module. A file's count may fall,
//! never rise; when a card moves a module to the view, it lowers that module's figures here and in the census together.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Each file's slice calls, as census row D7 states them; a file not named holds none.
const SLICES: [(&str, usize); 18] = [
    ("batch_stop.rs", 1),
    ("bin/class_judge.rs", 1),
    ("bin/ledger_parting.rs", 1),
    ("bin/w2m-vm.rs", 4),
    ("block_judge.rs", 1),
    ("cache_file.rs", 1),
    ("cases.rs", 15),
    ("compile.rs", 12),
    ("guest.rs", 20),
    ("host_request.rs", 22),
    ("kernel.rs", 1),
    ("model_step.rs", 4),
    ("modules.rs", 5),
    ("monitor.rs", 6),
    ("run.rs", 23),
    ("shared.rs", 1),
    ("threads.rs", 6),
    ("turns.rs", 2),
];

/// Each file's views built from a slice, as census row D7 states them; the view's module is not counted.
const SLICE_BUILT_VIEWS: [(&str, usize); 4] = [("cases.rs", 1), ("guest.rs", 1), ("monitor_code.rs", 1), ("run.rs", 7)];

/// Each `.rs` file under `dir` holding any of `patterns`, by its path from `root`, with the count.
fn counts(dir: &Path, root: &Path, patterns: &[&str], out: &mut BTreeMap<String, usize>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            counts(&path, root, patterns, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = fs::read_to_string(&path).unwrap();
            let n: usize = patterns.iter().map(|p| text.matches(p).count()).sum();
            if n > 0 {
                let name = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                out.insert(name, n);
            }
        }
    }
}

/// The files whose count of `patterns` is above `most`, named with both figures.
fn over(patterns: &[&str], most: &[(&str, usize)], skip: &str, what: &str) -> Vec<String> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = BTreeMap::new();
    counts(&src, &src, patterns, &mut found);
    found.remove(skip);
    let most: BTreeMap<&str, usize> = most.iter().copied().collect();
    found
        .iter()
        .filter(|(f, n)| **n > most.get(f.as_str()).copied().unwrap_or(0))
        .map(|(f, n)| {
            format!(
                "{f}: {n} {what}, census D7 allows {}",
                most.get(f.as_str()).copied().unwrap_or(0)
            )
        })
        .collect()
}

#[test]
fn no_file_holds_more_slice_calls_than_the_census_counts() {
    let over = over(&["as_slice()", "as_mut_slice()"], &SLICES, "", "slice calls");
    assert!(over.is_empty(), "{}", over.join("; "));
}

#[test]
fn no_file_builds_more_views_from_a_slice_than_the_census_counts() {
    let over = over(
        &["ConcurrentGuestMemory::new("],
        &SLICE_BUILT_VIEWS,
        "concurrent_memory.rs",
        "views built from a slice",
    );
    assert!(over.is_empty(), "{}", over.join("; "));
}
