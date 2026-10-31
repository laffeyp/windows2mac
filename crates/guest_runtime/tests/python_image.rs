// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 133's judge: the runtime image Rust builds equals, byte for byte, the one the frozen Python host's build
//! cached under the same key in `build/guest_runtime/`. Xcode's clang and `ld` build it, so the test runs on macOS.
//! From Sprint 271 it builds the C as it stood at `FROZEN`, exported from git, so the working tree's C may change
//! (plan step 6, "in the C runtime, changed only there") and the judge still proves the Rust build reproduces the
//! Python build.
#![cfg(target_os = "macos")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use guest_runtime::layout::{RUNTIME_IMAGE_VA, runtime_defines};
use guest_runtime::{build, cache_key, services, texts};

/// The tag whose C the frozen Python host built (the Decision "No Python runs").
const FROZEN: &str = "frozen-python-2026-10-29";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// A root holding `FROZEN`'s `tools/`, `specs/` and the code pages' probe run (`headers.rs` 70), every input `build`
/// reads, exported by `git archive`; made once and kept under the build directory.
fn frozen_root(root: &Path) -> PathBuf {
    let out = root.join(format!("build/guest_runtime_frozen/{FROZEN}"));
    if !out.join("tools/guest_runtime/runtime.h").exists() {
        fs::create_dir_all(&out).unwrap();
        let archive = Command::new("git")
            .arg("-C")
            .arg(root)
            .args([
                "archive",
                "--format=tar",
                FROZEN,
                "tools",
                "specs",
                "store/probe-runs/38044229348",
            ])
            .output()
            .unwrap();
        assert!(
            archive.status.success(),
            "git archive {FROZEN}: {}",
            String::from_utf8_lossy(&archive.stderr)
        );
        let mut tar = Command::new("tar")
            .arg("-x")
            .arg("-C")
            .arg(&out)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        std::io::Write::write_all(tar.stdin.as_mut().unwrap(), &archive.stdout).unwrap();
        assert!(tar.wait().unwrap().success(), "tar -x into {}", out.display());
    }
    out
}

/// The frozen sources' key, and the root to build them at: the repository's while its C keys the same, so its build
/// cache serves, else the export.
fn frozen_build_root(root: &Path) -> (String, PathBuf) {
    let frozen = frozen_root(root);
    let key = cache_key(&frozen, &texts(&frozen).unwrap(), &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    let here = cache_key(root, &texts(root).unwrap(), &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    (key.clone(), if here == key { root.to_path_buf() } else { frozen })
}

#[test]
fn the_image_rust_builds_is_the_python_builds_byte_for_byte() {
    let root = root();
    let (frozen_key, at) = frozen_build_root(&root);
    let (key, image) = build(&at, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    assert_eq!(key, frozen_key);
    let python = root.join(format!("build/guest_runtime/{key}.bin"));
    assert!(
        python.exists(),
        "no Python image under key {key}: a define, a header or a source differs from the Python build's"
    );
    let ours = fs::read(&image.file).unwrap();
    let theirs = fs::read(&python).unwrap();
    assert_eq!(ours.len(), theirs.len(), "sizes differ");
    assert!(
        ours == theirs,
        "the images differ at byte {:?}",
        ours.iter().zip(&theirs).position(|(a, b)| a != b)
    );
    let texts = texts(&at).unwrap();
    let missing: Vec<String> = services::services(&texts)
        .unwrap()
        .into_iter()
        .filter(|s| !image.symbols.contains_key(&s.symbol))
        .map(|s| s.symbol)
        .collect();
    assert!(missing.is_empty(), "the image does not define {missing:?}");
    println!(
        "python_image: key {key}, {} bytes, {} segments, {} symbols",
        ours.len(),
        image.segments.len(),
        image.symbols.len()
    );
}

/// The C files the working tree changes from `FROZEN`'s, each with the sprint and the reason; the image the VM runs
/// is the frozen one with these changes, which their own tests judge (Sprint 272).
const DEPARTURES: [(&str, &str); 3] = [
    (
        "tools/vm_compiler/vm_compiler.c",
        "Sprint 272: a memory XCHG declined to the host's atomic lowering",
    ),
    ("tools/vm_compiler/x86_decode.c", "Sprint 272: x86_is_xchg"),
    ("tools/vm_compiler/x86_decode.h", "Sprint 272: x86_is_xchg"),
];

/// Sprint 271, from Sprint 272 a list: the working tree's C differs from `FROZEN`'s in the files `DEPARTURES` names
/// and no others, so a change to the C is named with its reason, the frozen judge unmoved.
#[test]
fn the_working_trees_c_departs_from_the_frozen_c_only_where_named() {
    let root = root();
    let out = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args([
            "diff",
            "--name-only",
            FROZEN,
            "--",
            "tools/guest_runtime",
            "tools/vm_compiler",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "git diff {FROZEN}");
    let changed: Vec<&str> = std::str::from_utf8(&out.stdout).unwrap().lines().collect();
    let named: Vec<&str> = DEPARTURES.iter().map(|(f, _)| *f).collect();
    assert_eq!(
        changed, named,
        "the C files changed from {FROZEN}'s, against those named with a reason"
    );
    let (frozen_key, _) = frozen_build_root(&root);
    let here = cache_key(&root, &texts(&root).unwrap(), &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    assert_ne!(
        here, frozen_key,
        "the named departures leave the image's key the frozen one's"
    );
}

#[test]
fn a_define_changed_by_one_names_an_image_python_never_built() {
    let root = root();
    let mut defines = runtime_defines();
    for (k, v) in &mut defines {
        if k == "W2M_TIME_SLICE" {
            *v += 1;
        }
    }
    let key = cache_key(&root, &texts(&root).unwrap(), &defines, RUNTIME_IMAGE_VA).unwrap();
    assert!(!root.join(format!("build/guest_runtime/{key}.bin")).exists());
    let right = cache_key(&root, &texts(&root).unwrap(), &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    assert_ne!(key, right);
}

#[test]
fn a_source_byte_changed_names_another_key() {
    let root = root();
    let mut t = texts(&root).unwrap();
    let right = cache_key(&root, &t, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    t.get_mut("runtime.c").unwrap().push(' ');
    let key = cache_key(&root, &t, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    assert_ne!(key, right);
    assert!(!root.join(format!("build/guest_runtime/{key}.bin")).exists());
}
