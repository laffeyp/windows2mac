// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The `crates/` tree a binary was built from, against the tree it runs in (Sprint 187; reviewer-2's Medium 3 of its
//! review of 182-184): `build.rs` stamps the id; a program that writes records refuses to run when the work tree's
//! `crates/` is not the one it was built from.

use std::path::Path;
use std::process::Command;

/// The tree id of the build's inputs this build compiled (`build.rs`), "unknown" outside a git work tree.
pub const BUILT_FROM: &str = env!("W2M_CRATES_TREE");

/// The build's inputs (Sprint 192; reviewer-2's Medium 3 of its review of 185-189): the crates, the workspace's manifest
/// and lockfile, the pinned toolchain and cargo's config, as `build.rs` stages them.
pub const INPUTS: [&str; 5] = ["crates", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".cargo"];

/// The work tree's tree id of `INPUTS`, taken as `build.rs` takes it.
pub fn crates_tree(root: &Path) -> Result<String, String> {
    let index = std::env::temp_dir().join(format!("w2m-built-from-{}", std::process::id()));
    let _ = std::fs::remove_file(&index);
    let git = |args: &[&str]| -> Result<String, String> {
        let out = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .env("GIT_INDEX_FILE", &index)
            .output()
            .map_err(|e| format!("git: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "git {}: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    };
    let mut add = vec!["add", "-A", "--"];
    add.extend(INPUTS);
    let tree = git(&add).and_then(|_| git(&["write-tree"]));
    let _ = std::fs::remove_file(&index);
    tree
}

/// Ok when the work tree's `crates/` is the one this binary was built from; otherwise the refusal, naming both.
pub fn check(root: &Path, program: &str) -> Result<(), String> {
    let now = crates_tree(root)?;
    if now != BUILT_FROM {
        return Err(format!(
            "{program} was built from the inputs {BUILT_FROM} and the work tree's are {now}: a record is written only by a \
             binary built from the sources it stands for (`cargo run --release --bin {program}` rebuilds it)"
        ));
    }
    Ok(())
}
