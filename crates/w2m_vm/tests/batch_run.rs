// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The batch run on the Rust host to its first stop, and each tests/data guest's stop (Sprint 186; reviewer-2's High 1
//! of its review of Sprints 182-184: no card had run the Rust host on the batch run). `scripts/gates.sh` runs `w2m-vm
//! --game` on the batch run's depot and `w2m-vm` on each guest, writing build/batch_run/first-stop.tsv and guests.tsv;
//! this test holds them to `store/batch_run/first-stop-2026-10-28.tsv`, so a change that moves the first stop, or a
//! guest's, fails here until the held file is rewritten with the card that moved it. The run's seconds are not held,
//! and the guests' Python column is the frozen host's, for the reader.

use std::fs;
use std::path::{Path, PathBuf};

const HELD: &str = "store/batch_run/first-stop-2026-10-28.tsv";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// A file `gates.sh` writes under build/batch_run/; None when it has not run here (and the gates do not require it).
fn written(root: &Path, name: &str) -> Option<String> {
    match fs::read_to_string(root.join("build/batch_run").join(name)) {
        Ok(text) => Some(text),
        Err(_) => {
            let what = format!("no build/batch_run/{name} (scripts/gates.sh writes it)");
            if std::env::var("W2M_REQUIRE_CASE_RUNS").as_deref() == Ok("1") {
                panic!("{what}, and W2M_REQUIRE_CASE_RUNS=1 requires it");
            }
            eprintln!("{what}: the batch run's stop is not held here");
            None
        }
    }
}

#[test]
fn the_batch_run_and_the_guests_stop_where_held() {
    let root = root();
    let (Some(game), Some(guests)) = (written(&root, "first-stop.tsv"), written(&root, "guests.tsv")) else {
        return;
    };
    let mut ours: Vec<String> = game
        .lines()
        .filter(|l| !l.starts_with("seconds\t"))
        .map(str::to_string)
        .collect();
    ours.extend(guests.lines().map(|l| format!("guest\t{l}")));
    let held: Vec<String> = fs::read_to_string(root.join(HELD))
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| {
            // a guest's line without its fourth column, the Python host's stop and output
            if l.starts_with("guest\t") {
                l.split('\t').take(3).collect::<Vec<_>>().join("\t")
            } else {
                l.to_string()
            }
        })
        .collect();
    println!("{}", ours.join("\n"));
    assert_eq!(
        ours, held,
        "the batch run's first stop or a guest's stop moved from {HELD}"
    );
}
