// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The guest's layout is defined once (Sprint 296): `guest_runtime/src/layout.rs` holds each constant the runtime's C
//! takes as a define (`runtime_defines`), and `w2m_vm` re-exports what it uses. Seven names had a second definition in
//! `w2m_vm/src/`, equal today: `LOCK_OWNER`, `MXCSR_OFF`, `FCW_OFF`, `RUN_END`, `CLOCK_WARP`, `AP_READ_ONLY` and
//! `PER_THREAD_MONITOR`; a change to one copy would leave the host reading a word the runtime does not write.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// The names `text` defines as `pub const NAME`.
fn defined(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|l| l.strip_prefix("pub const "))
        .filter_map(|rest| rest.split(':').next())
        .map(|name| name.trim().to_string())
        .collect()
}

/// Each `.rs` file under `dir`, with its text.
fn sources(dir: &Path, out: &mut Vec<(String, String)>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push((path.display().to_string(), fs::read_to_string(&path).unwrap()));
        }
    }
}

#[test]
fn no_layout_constant_is_defined_again_in_w2m_vm() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let layout = defined(&fs::read_to_string(crates.join("guest_runtime/src/layout.rs")).unwrap());
    assert!(layout.contains("LOCK_OWNER"), "layout.rs read: {} names", layout.len());
    let mut files = Vec::new();
    sources(&crates.join("w2m_vm/src"), &mut files);
    let again: Vec<String> = files
        .iter()
        .flat_map(|(file, text)| defined(text).into_iter().map(move |name| (file.clone(), name)))
        .filter(|(_, name)| layout.contains(name))
        .map(|(file, name)| format!("{name} in {file}"))
        .collect();
    assert!(
        again.is_empty(),
        "defined again outside layout.rs: {}",
        again.join("; ")
    );
}
