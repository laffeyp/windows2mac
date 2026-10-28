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
pub const INPUTS: [&str; 6] = [
    "crates",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".cargo",
    "specs/x86_decode.json",
];

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

#[cfg(test)]
mod tests {
    use super::INPUTS;
    use std::path::Path;

    /// The quoted strings between `INPUTS` and the `];` that closes its list.
    fn listed(text: &str) -> Vec<String> {
        let start = text.find("const INPUTS").unwrap();
        let body = &text[start..start + text[start..].find("];").unwrap()];
        let body = &body[body.find("= [").unwrap()..];
        body.split('"').skip(1).step_by(2).map(str::to_string).collect()
    }

    /// Every path outside `crates/` a build script names (`../../<path>`) is one of `INPUTS`, and `build.rs`'s list is
    /// this one (Sprint 194; reviewer-2's Medium 3 of its review of 190-192: `crates/x86_decoder/build.rs` read
    /// `specs/x86_decode.json`, which the digest left out).
    #[test]
    fn every_file_a_build_script_reads_is_an_input() {
        let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let ours = std::fs::read_to_string(crates.join("w2m_vm/build.rs")).unwrap();
        assert_eq!(listed(&ours), INPUTS, "build.rs's INPUTS is not built_from's");
        let mut scripts = 0;
        for entry in std::fs::read_dir(&crates).unwrap() {
            let Ok(text) = std::fs::read_to_string(entry.unwrap().path().join("build.rs")) else {
                continue;
            };
            scripts += 1;
            for piece in text.split("\"../../").skip(1) {
                let path = &piece[..piece.find('"').unwrap()];
                if path.starts_with('{') {
                    continue;
                }
                let covered = INPUTS.iter().any(|i| path == *i || path.starts_with(&format!("{i}/")));
                assert!(covered, "a build script reads ../../{path}, which INPUTS does not name");
            }
        }
        assert!(scripts >= 2, "found {scripts} build scripts");
    }
}
