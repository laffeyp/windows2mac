// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! One definition a fact (Sprints 296 and 297). A `pub const` name defined in two source files under `crates/` is a
//! fact stated twice, which a change to one copy would split: Sprint 296 found seven of the layout's names (then
//! `guest_runtime/src/layout.rs`, `guest_layout` from Sprint 298) defined again in `w2m_vm`, and reviewer-3's Low 1 at 296 found 24 more across the crates. A name in two files
//! passes only if one definition derives its value from the other's path, or a named test holds the two equal, or the
//! two are different facts that share a name, each allowed below with its reason.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Names two crates define, held equal by the test named, where no dependency joins them.
const HELD: [(&str, &str); 3] = [
    (
        "ACCESS_VIOLATION",
        "w2m_vm/tests/layout_constants.rs `access_violation_is_one_status`",
    ),
    (
        "X87_ST_OFF",
        "translator/tests/layout.rs `the_x87_offsets_are_the_guest_layouts` (Sprint 236)",
    ),
    (
        "X87_SW_OFF",
        "translator/tests/layout.rs `the_x87_offsets_are_the_guest_layouts` (Sprint 236)",
    ),
];

/// Different facts that share a name.
const DIFFERENT: [(&str, &str); 12] = [
    (
        "ALL",
        "each generated enum's own list of its values (events/src/generated.rs), and the runtime word's sites",
    ),
    (
        "STACK_SIZE",
        "cpu_cases' case stack, 0x10000; pe_loader's thread default, 1 MiB",
    ),
    (
        "ELR_EL1",
        "a64_encoder's MSR operand tuple; hypervisor's Hypervisor.framework register id",
    ),
    (
        "ESR_EL1",
        "a64_encoder's MSR operand tuple; hypervisor's Hypervisor.framework register id",
    ),
    (
        "FAR_EL1",
        "a64_encoder's MSR operand tuple; hypervisor's Hypervisor.framework register id",
    ),
    (
        "SPSR_EL1",
        "a64_encoder's MSR operand tuple; hypervisor's Hypervisor.framework register id",
    ),
    (
        "IE",
        "the FPU status word's bit (x87.rs) and MXCSR's (x86fp.rs), two registers laid out alike",
    ),
    (
        "DE",
        "the FPU status word's bit (x87.rs) and MXCSR's (x86fp.rs), two registers laid out alike",
    ),
    (
        "ZE",
        "the FPU status word's bit (x87.rs) and MXCSR's (x86fp.rs), two registers laid out alike",
    ),
    (
        "OE",
        "the FPU status word's bit (x87.rs) and MXCSR's (x86fp.rs), two registers laid out alike",
    ),
    (
        "UE",
        "the FPU status word's bit (x87.rs) and MXCSR's (x86fp.rs), two registers laid out alike",
    ),
    (
        "PE",
        "the FPU status word's bit (x87.rs) and MXCSR's (x86fp.rs), two registers laid out alike",
    ),
];

/// Each `pub const NAME` in `text`, at any indent, with the rest of its line.
fn defined(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|l| l.trim_start().strip_prefix("pub const "))
        .filter_map(|rest| rest.split_once(':'))
        .map(|(name, value)| (name.trim().to_string(), value.to_string()))
        .collect()
}

/// Whether `value` names a path ending `::name`, the whole identifier: `layout::HEAP_VA` names `HEAP_VA`, not `HEAP`.
fn names(value: &str, name: &str) -> bool {
    let tail = format!("::{name}");
    value.match_indices(&tail).any(|(at, _)| {
        !value[at + tail.len()..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

#[test]
fn a_derived_value_names_the_whole_identifier() {
    assert!(names(" u32 = crate::kernel::STILL_ACTIVE as u32;", "STILL_ACTIVE"));
    assert!(!names(" u64 = layout::HEAP_VA;", "HEAP"));
    assert!(names(" u16 = translator::lower::HVC_SLOW_SSE as u16;", "HVC_SLOW_SSE"));
}

/// Each `.rs` file under a crate's `src/`, with its text.
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
fn every_fact_has_one_definition_or_a_named_reason() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for entry in fs::read_dir(&crates).unwrap() {
        let src = entry.unwrap().path().join("src");
        if src.is_dir() {
            sources(&src, &mut files);
        }
    }
    // name -> (file, the rest of its definition's line)
    let mut by_name: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for (file, text) in &files {
        for (name, value) in defined(text) {
            by_name.entry(name).or_default().push((file.clone(), value));
        }
    }
    assert!(
        by_name.contains_key("LOCK_OWNER"),
        "the crates read: {} names",
        by_name.len()
    );
    let allowed = |name: &str| HELD.iter().chain(DIFFERENT.iter()).any(|(n, _)| *n == name);
    let twice: Vec<String> = by_name
        .iter()
        .filter(|(_, defs)| {
            let files: std::collections::BTreeSet<&String> = defs.iter().map(|(f, _)| f).collect();
            files.len() > 1
        })
        .filter(|(name, defs)| {
            let derived = defs.iter().any(|(_, v)| names(v, name));
            !derived && !allowed(name)
        })
        .map(|(name, defs)| {
            let files: Vec<&str> = defs.iter().map(|(f, _)| f.as_str()).collect();
            format!("{name} in {}", files.join(", "))
        })
        .collect();
    assert!(
        twice.is_empty(),
        "defined in two files, neither derived from the other nor held nor allowed: {}",
        twice.join("; ")
    );
    // an allowed name no longer defined twice is struck from the lists, so they say only what is true
    let stale: Vec<&str> = HELD
        .iter()
        .chain(DIFFERENT.iter())
        .map(|(n, _)| *n)
        .filter(|n| by_name.get(*n).is_none_or(|d| d.len() < 2))
        .collect();
    assert!(stale.is_empty(), "allowed but defined once: {stale:?}");
}

/// The loader and the arena use the layout alone, so they reach it in `guest_layout`, a crate of facts, and not through
/// `guest_runtime`, which builds the C runtime's image (reviewer-3's Low 1 at 297; Sprint 298). A manifest's lines are
/// read whole, less comments, so every form Cargo takes is caught: `name = {..}`, `name.workspace = true`, and the
/// table forms `[dependencies.name]` and `[target.'cfg(..)'.dependencies]`.
#[test]
fn the_loader_and_the_arena_reach_the_layout_without_the_image_builder() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let lines = |name: &str| -> Vec<String> {
        fs::read_to_string(crates.join(name).join("Cargo.toml"))
            .unwrap()
            .lines()
            .map(|l| l.split('#').next().unwrap_or("").to_string())
            .collect()
    };
    let through_builder: Vec<&str> = ["pe_loader", "virtual_memory"]
        .into_iter()
        .filter(|name| lines(name).iter().any(|l| l.contains("guest_runtime")))
        .collect();
    assert!(
        through_builder.is_empty(),
        "depending on guest_runtime for the layout: {through_builder:?}"
    );
    let tables: Vec<String> = lines("guest_layout")
        .into_iter()
        .filter(|l| l.contains("dependencies"))
        .collect();
    assert!(tables.is_empty(), "guest_layout has dependencies: {tables:?}");
}

/// `STATUS_ACCESS_VIOLATION`, stated by `cpu_cases` (the case runner's faults) and `pe_loader` (the loader's), which
/// no dependency joins (Sprint 297).
#[test]
fn access_violation_is_one_status() {
    assert_eq!(
        u64::from(pe_loader::ACCESS_VIOLATION),
        cpu_cases::ACCESS_VIOLATION,
        "pe_loader's and cpu_cases' ACCESS_VIOLATION"
    );
}
