// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The skeleton's gate (Sprint 158; link 6 of `docs/design-host-compile-path-2026-10-24.md`): the Rust host's record
//! and output of `tests/data/timed_waits/sleeper.exe` equal the stored ones, the frozen Python host's. The Rust record
//! is `w2m-vm`'s run under `harness/w2m-vm-sleeper.toml`; the Python's is `record_translations` of the same guest, its
//! generator in WORKING_AGREEMENT. Both records are committed, so the record's check runs everywhere; the output file
//! is the run's, under `build/`, which only this Mac has, and the gates require it (`W2M_REQUIRE_CASE_RUNS=1`).

use std::fs;
use std::path::{Path, PathBuf};

use w2m_vm::record::{compare, w2m_events};

/// The Rust host's record (`harness/w2m-vm-sleeper.toml`) and the frozen Python host's.
const RUST_RECORD: &str = "store/runs/w2m-vm-sleeper-1793149355000000000";
const PYTHON_RECORD: &str = "store/runs/translations-1793149239949363000";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

#[test]
fn the_rust_host_s_record_of_sleeper_is_the_python_host_s() {
    let root = root();
    let ours = w2m_events(&root.join(RUST_RECORD));
    let stored = w2m_events(&root.join(PYTHON_RECORD));
    // SESSION_INIT, 19 blocks of two events each, SESSION_COMPLETED
    assert_eq!(stored.len(), 40, "the stored record's events");
    assert_eq!(compare(&ours, &stored), Ok(()));
}

#[test]
fn the_rust_host_s_output_of_sleeper_is_the_stored_output() {
    let root = root();
    let stored = fs::read(root.join("store/skeleton/sleeper-stdout.txt")).unwrap();
    assert_eq!(stored, b"woke\ndone\n");
    let Ok(ours) = fs::read(root.join("build/skeleton/sleeper-stdout.txt")) else {
        let what = "no output of w2m-vm's run of sleeper (build/skeleton/sleeper-stdout.txt)";
        if std::env::var("W2M_REQUIRE_CASE_RUNS").as_deref() == Ok("1") {
            panic!("{what}, and W2M_REQUIRE_CASE_RUNS=1 requires it");
        }
        eprintln!("{what}: the output's check is not run here");
        return;
    };
    assert_eq!(
        String::from_utf8_lossy(&ours),
        String::from_utf8_lossy(&stored),
        "the output"
    );
}
