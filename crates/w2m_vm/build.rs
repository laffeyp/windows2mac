// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The `crates/` tree this build compiles, stamped into the crate as `W2M_CRATES_TREE` (Sprint 187; reviewer-2's
//! Medium 3 of its review of 182-184: a relinked planted `case_runs` wrote four records). The tree id is taken as
//! `scripts/gates.sh` takes its trees: the work tree's `crates/` staged into a temporary index, then `git write-tree`,
//! so an untracked file counts as much as a changed one. `w2m_vm::built_from` takes the same id at run time.

use std::path::Path;
use std::process::Command;

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    println!("cargo:rerun-if-changed=../../crates");
    let index = Path::new(&std::env::var("OUT_DIR").unwrap()).join("crates-index");
    let _ = std::fs::remove_file(&index);
    let git = |args: &[&str]| {
        Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(args)
            .env("GIT_INDEX_FILE", &index)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let tree = git(&["add", "-A", "--", "crates"])
        .and_then(|_| git(&["write-tree", "--prefix=crates/"]))
        .unwrap_or_else(|| "unknown".to_string());
    let _ = std::fs::remove_file(&index);
    println!("cargo:rustc-env=W2M_CRATES_TREE={tree}");
}
