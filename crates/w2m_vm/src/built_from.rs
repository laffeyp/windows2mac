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
/// cache's key takes it of its own inputs, `cache_file::KEY_INPUTS`). Each call's index is its own, by the process and
/// a count (Sprint 288: a test binary's tests, threads of one process under `cargo test`, call it at once through
/// `fresh`, and one index named by the process alone was removed and rewritten under another call).
pub fn tree_of(root: &Path, inputs: &[&str]) -> Result<String, String> {
    static CALLS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let call = CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let index = std::env::temp_dir().join(format!("w2m-built-from-{}-{call}", std::process::id()));
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

/// What a run reads from the work tree beyond its build (Sprint 288; reviewer-3's Low 1 at 287): the guest runtime's C,
/// which `guest_runtime` compiles when a run starts.
pub const RUN_INPUTS: [&str; 1] = ["tools"];

/// A run's output file's stamp, beside it: the file's name and this (reviewer-3's Low 1 at 288: one stamp a directory
/// vouched for a file an earlier run left there, a timed run stamping `build/batch_run/` beside an older
/// `whole-blocks.tsv`).
pub const STAMP_SUFFIX: &str = ".tree";

/// The stamp beside `file`.
pub fn stamp_of(file: &Path) -> std::path::PathBuf {
    let mut name = file.as_os_str().to_owned();
    name.push(STAMP_SUFFIX);
    name.into()
}

/// The sources a run's outputs stand for, one line: the build's tree (`BUILT_FROM`) and `RUN_INPUTS`' tree as the run
/// found them.
pub fn run_stamp(root: &Path) -> Result<String, String> {
    Ok(format!("{BUILT_FROM} {}", tree_of(root, &RUN_INPUTS)?))
}

/// The work tree's sources in a stamp's form: `INPUTS`' tree and `RUN_INPUTS`'.
pub fn work_tree_stamp(root: &Path) -> Result<String, String> {
    Ok(format!("{} {}", crates_tree(root)?, tree_of(root, &RUN_INPUTS)?))
}

/// Writes `stamp` beside `file`, which a run has written.
pub fn stamp(file: &Path, stamp: &str) -> Result<(), String> {
    let at = stamp_of(file);
    std::fs::write(&at, format!("{stamp}\n")).map_err(|e| format!("{}: {e}", at.display()))
}

/// Removes `file`'s stamp, as a run told to write `file` starts: a run that ends before writing it leaves no stamp.
pub fn unstamp(file: &Path) -> Result<(), String> {
    match std::fs::remove_file(stamp_of(file)) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(format!("{}: {e}", stamp_of(file).display())),
        _ => Ok(()),
    }
}

/// None when `file` was written by a run of the sources `now` names (`work_tree_stamp`); otherwise why not, so a
/// test judges no output of other sources.
pub fn stale(now: &str, file: &Path) -> Option<String> {
    match std::fs::read_to_string(stamp_of(file)) {
        Err(e) => Some(format!("{}: no stamp ({e})", stamp_of(file).display())),
        Ok(made) if made.trim() == now => None,
        Ok(made) => Some(format!(
            "{} was made from the sources {} and the work tree's are {now} (the build's tree, then {RUN_INPUTS:?}'s)",
            file.display(),
            made.trim()
        )),
    }
}

/// For a test judging `file`, a run's output or one the same run wrote beside it: true when it was made from the work
/// tree's sources; false otherwise, after a panic naming why when `W2M_REQUIRE_CASE_RUNS=1` (the gates set it) or the
/// reason on stderr, as a test treats a missing file. The work tree's stamp is taken once a process: a test process
/// judges one work tree.
pub fn fresh(root: &Path, file: &Path) -> bool {
    static NOW: std::sync::OnceLock<Result<String, String>> = std::sync::OnceLock::new();
    let why = match NOW.get_or_init(|| work_tree_stamp(root)) {
        Ok(now) => stale(now, file),
        Err(e) => Some(e.clone()),
    };
    let Some(why) = why else {
        return true;
    };
    if std::env::var("W2M_REQUIRE_CASE_RUNS").as_deref() == Ok("1") {
        panic!("{why}, and W2M_REQUIRE_CASE_RUNS=1 requires outputs of the work tree's sources");
    }
    eprintln!("{why}: not judged here");
    false
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
    use super::{INPUTS, RUN_INPUTS, crates_tree, stale, stamp, tree_of, unstamp, work_tree_stamp};
    use std::path::Path;
    use std::process::Command;

    /// Threads of one process taking the same tree at once each get it whole (Sprint 288: under `cargo test` the
    /// readers' tests call `fresh` together).
    #[test]
    fn threads_taking_a_tree_at_once_agree() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let once = tree_of(&root, &RUN_INPUTS).unwrap();
        let trees: Vec<Result<String, String>> = std::thread::scope(|s| {
            let calls: Vec<_> = (0..16)
                .map(|_| s.spawn(|| (0..4).map(|_| tree_of(&root, &RUN_INPUTS)).collect::<Vec<_>>()))
                .collect();
            calls.into_iter().flat_map(|c| c.join().unwrap()).collect()
        });
        assert_eq!(trees.len(), 64);
        for tree in trees {
            assert_eq!(tree, Ok(once.clone()));
        }
    }

    /// A file stamped with a repository's trees is fresh, and stale once a file under `tools/` changes or its stamp is
    /// gone, the reason naming the file (Sprint 288: Sprint 286's record, made before a C change, was judged); a file
    /// beside it stamped by an earlier run stays stale when a later run stamps its own (reviewer-3's Low 1 at 288). The
    /// repository holds a file under each of `INPUTS`, which `crates_tree` stages by name.
    #[test]
    fn a_stamp_goes_stale_when_tools_changes() {
        let repo = std::env::temp_dir().join(format!("w2m-stale-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&repo);
        for file in [
            "crates/a.rs",
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain.toml",
            ".cargo/config.toml",
            "specs/a",
        ]
        .into_iter()
        .chain(["tools/a.c", "build/out/judged.tsv", "build/out/stop.tsv"])
        {
            std::fs::create_dir_all(repo.join(file).parent().unwrap()).unwrap();
            std::fs::write(repo.join(file), "1\n").unwrap();
        }
        assert!(
            Command::new("git")
                .arg("init")
                .arg("-q")
                .arg(&repo)
                .status()
                .unwrap()
                .success()
        );
        let (judged, stop) = (repo.join("build/out/judged.tsv"), repo.join("build/out/stop.tsv"));
        let made = work_tree_stamp(&repo).unwrap();
        assert_eq!(
            made,
            format!(
                "{} {}",
                crates_tree(&repo).unwrap(),
                tree_of(&repo, &RUN_INPUTS).unwrap()
            )
        );
        stamp(&judged, &made).unwrap();
        stamp(&stop, &made).unwrap();
        assert_eq!(stale(&made, &judged), None);
        std::fs::write(repo.join("tools/a.c"), "2\n").unwrap();
        let now = work_tree_stamp(&repo).unwrap();
        let why = stale(&now, &judged).expect("a changed tools/ left the stamp fresh");
        assert!(why.contains("judged.tsv") && why.contains(&made), "{why}");
        // a later run writes stop.tsv alone: judged.tsv, beside it, is still the earlier run's
        unstamp(&stop).unwrap();
        stamp(&stop, &now).unwrap();
        assert_eq!(stale(&now, &stop), None);
        assert!(
            stale(&now, &judged).is_some(),
            "a later run's stamp vouched for a file beside it"
        );
        unstamp(&judged).unwrap();
        assert!(stale(&now, &judged).unwrap().contains("no stamp"));
        let _ = std::fs::remove_dir_all(&repo);
    }

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
