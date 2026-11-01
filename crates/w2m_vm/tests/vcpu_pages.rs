// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The host reads a vCPU's own pages only through that vCPU's base (Sprint 313; the per-vCPU pages design,
//! `docs/design-each-vcpus-own-pages-2026-10-31.md`): no code under `src/` reaches the monitor data page at
//! `MONITOR_DATA` or the state block at `MONITOR_SIZE + STATE_OFF` as a memory offset, save `VcpuPages::FIRST`, which
//! names vCPU 0's. A guest address (`MONITOR_VA + MONITOR_DATA`, `PROCESS_VA + STATE_OFF`) is the same on every vCPU
//! and stays.

use std::fs;
use std::path::{Path, PathBuf};

/// Every `.rs` file under `dir`, in order.
fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            sources(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

/// Whether `line` reaches a vCPU's page at a fixed memory offset: `MONITOR_DATA` used as a value (followed by `+`,
/// `)`, `;`, `]` or ` as`) and not as a guest address after `MONITOR_VA + `, or `MONITOR_SIZE + STATE_OFF`; a name
/// spelt with its `guest_runtime::layout::` path counts as the bare name.
fn fixed(line: &str) -> bool {
    let code = line
        .split("//")
        .next()
        .unwrap_or("")
        .replace("guest_runtime::layout::", "");
    let monitor = code.match_indices("MONITOR_DATA").any(|(i, m)| {
        let after = code[i + m.len()..].trim_start();
        let used = ["+", ")", ";", "]", "as "].iter().any(|t| after.starts_with(t));
        used && !code[..i].trim_end().ends_with("MONITOR_VA +")
    });
    monitor || code.contains("MONITOR_SIZE + STATE_OFF")
}

#[test]
fn no_vcpu_page_is_read_at_a_fixed_offset() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&src, &mut files);
    let mut found = Vec::new();
    for f in &files {
        let text = fs::read_to_string(f).unwrap();
        for (n, line) in text.lines().enumerate() {
            // `VcpuPages::FIRST`'s state block, vCPU 0's, the one fixed offset the rule allows
            let first = f.ends_with("lib.rs")
                && line.trim() == "state: guest_runtime::layout::MONITOR_SIZE + guest_runtime::layout::STATE_OFF,";
            if fixed(line) && !first {
                found.push(format!("{}:{}", f.strip_prefix(&src).unwrap().display(), n + 1));
            }
        }
    }
    assert!(found.is_empty(), "a vCPU's page read at a fixed offset: {found:?}");
}

#[test]
fn the_rule_names_each_form() {
    assert!(fixed("    let md = MONITOR_DATA;"));
    assert!(fixed("read_u64(mem, MONITOR_DATA + CLOCK_MODE)"));
    assert!(fixed("let st = MONITOR_SIZE + STATE_OFF;"));
    assert!(fixed(
        "let st = guest_runtime::layout::MONITOR_SIZE + guest_runtime::layout::STATE_OFF;"
    ));
    assert!(!fixed("x[28] = MONITOR_VA + MONITOR_DATA;"));
    assert!(!fixed("    MONITOR_DATA, MONITOR_SIZE, MONITOR_VA,"));
    assert!(!fixed("// MONITOR_DATA + LOCK_OWNER"));
}
