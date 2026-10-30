// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Every request the guest can ask has an arm on the Rust host (Sprint 219). HOST_FILETIME stopped Sprint 218's probes
//! only after a run reached it, though the code named it all along; a comparison of the request lists then found
//! HOST_CLOCK, HOST_UNIMPLEMENTED and THREAD_WAKE the same way. The lists are known from the code, so they are read
//! from it here, not met in a run (KIT_DIARY #87).
//!
//! Two sources (reviewer-3's Medium 1 of its review of Sprint 219): the requests the guest can ask, the enums of
//! tools/guest_runtime/runtime.h (`W2M_HOST_*`, `W2M_FILE_*`, `W2M_KERNEL_*`, `W2M_EXCEPTION_*`, `W2M_THREAD_*`); and
//! the frozen host's answer to them, its `if`/`elif` dispatch lines that test `op`, `sub` or `a0`, read once from its
//! Python and stored in `store/frozen_python/host-dispatch-2026-10-29.tsv` before the Python left the tree (Sprint 226;
//! the tag frozen-python-2026-10-29). A name the Rust host
//! serves is matched as a whole word (`NAME =>`, `| NAME`, `NAME |`, `== NAME`) outside comments, in the file that
//! holds those requests. `REFUSED` names the requests the Rust host refuses on purpose. An arm that matches the name
//! and then refuses passes here; the record of a run names such a refusal.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Requests the guest's enums still name that no code asks: thread request 8, retired at Sprint 069.
const REFUSED: [&str; 1] = ["THREAD_SIGNAL_RETIRED"];

/// The frozen host's dispatch names by file, stored from its Python (Sprint 226).
const DISPATCH: &str = "store/frozen_python/host-dispatch-2026-10-29.tsv";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The names in `runtime.h`'s `enum { ... }` blocks with the prefix `W2M_<group>_`, the prefix taken off as the Rust
/// names them (`W2M_HOST_CLOCK` is `HOST_CLOCK`; `W2M_KERNEL_CLOSE` is `CLOSE`).
fn enumerated(header: &str, group: &str, keep_group: bool) -> BTreeSet<String> {
    let prefix = format!("W2M_{group}_");
    let mut out = BTreeSet::new();
    for block in header.split("enum {").skip(1) {
        let body = block.split('}').next().unwrap_or("");
        for token in body.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
            if let Some(rest) = token.strip_prefix(&prefix) {
                out.insert(if keep_group {
                    format!("{group}_{rest}")
                } else {
                    rest.to_string()
                });
            }
        }
    }
    out
}

/// The Rust text without its `//` comments.
fn code(rust: &str) -> String {
    rust.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether `code` matches `name` as a request: the name as a whole word before `=>` or `|`, or after `|` or `==`.
fn served(code: &str, name: &str) -> bool {
    let word = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
    let mut from = 0;
    while let Some(i) = code[from..].find(name) {
        let at = from + i;
        let (before, after) = (code[..at].chars().next_back(), code[at + name.len()..].chars().next());
        if !word(before) && !word(after) {
            let left = code[..at].trim_end();
            let right = code[at + name.len()..].trim_start();
            if right.starts_with("=>") || right.starts_with('|') || left.ends_with('|') || left.ends_with("==") {
                return true;
            }
        }
        from = at + name.len();
    }
    false
}

#[test]
fn every_request_the_guest_can_ask_has_a_rust_arm() {
    let root = root();
    let read = |p: &str| fs::read_to_string(root.join(p)).unwrap();
    let header = read("tools/guest_runtime/runtime.h");
    let host_request = code(&read("crates/w2m_vm/src/host_request.rs"));
    let kernel = code(&read("crates/w2m_vm/src/kernel.rs"));
    let files = code(&read("crates/w2m_vm/src/file_bridge.rs"));
    // (the names, from where, the Rust code that serves them)
    let mut sources: Vec<(BTreeSet<String>, String, &str)> = vec![
        (
            enumerated(&header, "HOST", true),
            "runtime.h W2M_HOST_".into(),
            &host_request,
        ),
        (
            enumerated(&header, "EXCEPTION", true),
            "runtime.h W2M_EXCEPTION_".into(),
            &host_request,
        ),
        (
            enumerated(&header, "THREAD", true),
            "runtime.h W2M_THREAD_".into(),
            &host_request,
        ),
        (
            enumerated(&header, "KERNEL", false),
            "runtime.h W2M_KERNEL_".into(),
            &kernel,
        ),
        (enumerated(&header, "FILE", true), "runtime.h W2M_FILE_".into(), &files),
    ];
    // the frozen host's dispatch names, stored once from its Python by `dispatched` before the Python left the tree
    // (Sprint 226; the tag frozen-python-2026-10-29)
    let extract = read(DISPATCH);
    for (python, rust) in [
        ("src/w2m/vm_backend.py", &host_request),
        ("src/w2m/kernel.py", &kernel),
        ("src/w2m/file_bridge.py", &files),
    ] {
        let names: BTreeSet<String> = extract
            .lines()
            .filter(|l| !l.starts_with('#'))
            .filter_map(|l| l.split_once('\t'))
            .filter(|(file, _)| *file == python)
            .map(|(_, name)| name.to_string())
            .collect();
        sources.push((names, format!("{python} ({DISPATCH})"), rust));
    }
    let mut missing = Vec::new();
    let mut checked = BTreeSet::new();
    for (names, from, rust) in &sources {
        assert!(!names.is_empty(), "{from}: no request read");
        for name in names {
            if REFUSED.contains(&name.as_str()) {
                continue;
            }
            checked.insert(name.clone());
            if !served(rust, name) {
                missing.push(format!("{name} ({from})"));
            }
        }
    }
    println!("{} requests the guest can ask, each with a Rust arm", checked.len());
    assert!(
        missing.is_empty(),
        "requests the Rust host has no arm for: {missing:#?}"
    );
}

/// The readers: an enum's names by group; a whole word in code, not a
/// longer name's tail and not a comment.
#[test]
fn the_readers_take_whole_names_only() {
    let header = "enum { W2M_HOST_A = 1, W2M_HOST_B,\n W2M_FILE_C };\nenum { W2M_KERNEL_CLOSE = 1 };";
    assert_eq!(
        enumerated(header, "HOST", true).into_iter().collect::<Vec<_>>(),
        ["HOST_A", "HOST_B"]
    );
    assert_eq!(
        enumerated(header, "KERNEL", false).into_iter().collect::<Vec<_>>(),
        ["CLOSE"]
    );
    assert!(served("    CLOSE => 0,", "CLOSE"));
    assert!(!served("    FILE_CLOSE => 0,", "CLOSE"), "a longer name's tail");
    assert!(!served(&code("    x => 0, // CLOSE =>"), "CLOSE"), "a comment");
    assert!(served("    EVENT_STATE | CLOSE => 0,", "CLOSE"));
}
