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
/// and lockfile, the pinned toolchain and cargo's config, as `build.rs` stages them; and `specs/`, whose
/// `x86_decode.json` a build script reads (Sprint 194) and whose `approximations.json` `approximate.rs` compiles in
/// (Sprint 238; reviewer-3's Low 1 of its review at 237).
pub const INPUTS: [&str; 6] = [
    "crates",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".cargo",
    "specs",
];

/// The work tree's tree id of `INPUTS`, taken as `build.rs` takes it.
pub fn crates_tree(root: &Path) -> Result<String, String> {
    tree_of(root, &INPUTS)
}

/// The work tree's tree id of `inputs`, paths from the root, through a temporary index (Sprint 265: the translation
/// cache's key takes it of its own inputs, `cache_file::KEY_INPUTS`).
pub fn tree_of(root: &Path, inputs: &[&str]) -> Result<String, String> {
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
    add.extend(inputs);
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

    /// The `.rs` files under `dir`, `tests/` directories left out: their includes are not in a record-writing binary.
    fn sources(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if !["tests", "target"].iter().any(|n| path.ends_with(n)) {
                    sources(&path, out);
                }
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    /// Every file an `include_str!` or `include_bytes!` outside `tests/` compiles in is under one of `INPUTS` (Sprint
    /// 238; reviewer-3's Low 1 of its review at 237: `approximate.rs` compiled in `specs/approximations.json`, which the
    /// digest left out, so a changed table reached a record unstamped).
    #[test]
    fn every_file_a_source_compiles_in_is_an_input() {
        let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").canonicalize().unwrap();
        let root = crates.parent().unwrap();
        let mut files = Vec::new();
        sources(&crates, &mut files);
        let mut includes = 0;
        for file in files {
            let text = std::fs::read_to_string(&file).unwrap();
            for mac in ["include_str!(\"", "include_bytes!(\""] {
                for piece in text.split(mac).skip(1) {
                    let named = &piece[..piece.find('"').unwrap()];
                    let path = file.parent().unwrap().join(named).canonicalize();
                    let path = path.unwrap_or_else(|e| panic!("{}: {named}: {e}", file.display()));
                    let rel = path.strip_prefix(root).unwrap().to_string_lossy().to_string();
                    let covered = INPUTS.iter().any(|i| rel == *i || rel.starts_with(&format!("{i}/")));
                    assert!(
                        covered,
                        "{} compiles in {rel}, which INPUTS does not name",
                        file.display()
                    );
                    includes += 1;
                }
            }
        }
        assert!(includes >= 2, "found {includes} includes");
    }
}
