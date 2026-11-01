// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! `Guest`'s parts are the run-loop design's (`docs/design-run-loop-on-several-vcpus-2026-10-31.md`, "The guest's
//! parts"; Sprint 302): the fields the table calls Served are `Served`'s, and no other, so the lock card 2b puts on
//! `served` covers what a served exit writes.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// The field names declared in `pub struct <name> { .. }` of `text`.
fn fields(text: &str, name: &str) -> BTreeSet<String> {
    let start = text.find(&format!("pub struct {name} {{")).expect("the struct");
    let body = &text[start..start + text[start..].find("\n}").expect("its end")];
    body.lines()
        .filter_map(|l| l.trim_start().strip_prefix("pub "))
        .filter_map(|l| l.split_once(':'))
        .map(|(f, _)| f.trim().to_string())
        .collect()
}

#[test]
fn the_served_part_is_the_designs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let design = fs::read_to_string(root.join("docs/design-run-loop-on-several-vcpus-2026-10-31.md")).unwrap();
    let table: BTreeSet<String> = design
        .lines()
        .filter_map(|l| {
            let cells: Vec<&str> = l.split('|').map(str::trim).collect();
            (cells.len() > 3 && cells[2] == "Served").then(|| cells[1].trim_matches('`').to_string())
        })
        .collect();
    let source = fs::read_to_string(root.join("crates/w2m_vm/src/guest.rs")).unwrap();
    let served = fields(&source, "Served");
    let only_served: Vec<&String> = served.difference(&table).collect();
    let only_table: Vec<&String> = table.difference(&served).collect();
    assert!(
        only_served.is_empty() && only_table.is_empty(),
        "in Served and not a Served row of the design: {only_served:?}; a Served row and not in Served: {only_table:?}"
    );
    assert_eq!(table.len(), 24, "the design's table counts 24 Served");
    let guest = fields(&source, "Guest");
    let both: Vec<&String> = guest.intersection(&table).collect();
    assert!(both.is_empty(), "Served fields still on Guest: {both:?}");
}
