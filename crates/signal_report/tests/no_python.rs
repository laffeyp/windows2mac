// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! No Python in the tree but the read-only kit's (Sprint 226; BLACKBOARD Decision 2026-10-29, "No Python runs", and the
//! user: "the 22% of python into 0"). The frozen Python host, its tests and scripts are at the tag
//! frozen-python-2026-10-29; `sdd-kit-3/` is the kit, never edited, its 13 example and library files marked
//! `linguist-vendored` in `.gitattributes`.

use std::path::Path;
use std::process::Command;

/// The tracked paths ending in `.py`, from `git ls-files` at the repository root.
fn tracked_python(root: &Path) -> Vec<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--", "*.py"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn no_python_is_tracked_outside_the_kit() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let outside: Vec<String> = tracked_python(&root)
        .into_iter()
        .filter(|p| !p.starts_with("sdd-kit-3/"))
        .collect();
    assert!(outside.is_empty(), "Python tracked outside sdd-kit-3/: {outside:#?}");
}
