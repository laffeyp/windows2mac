// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The Rust judges' ledger by defect (Sprint 223; reviewer-3's Medium 1 of its review at Sprint 222's close): the stored
//! ledger, `store/case_runner/discrepancy-ledger.json`, holds one entry per distinct defect the records report, each
//! report in exactly one entry. `case_runs --check-ledger` holds the file to `ledger::rebuild`; this test holds
//! `rebuild`'s entries to the records' reports, read here without it.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use w2m_vm::ledger::{defect, first_frame, frames};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

#[test]
fn the_ledger_holds_one_entry_per_defect_and_every_report_in_one() {
    let root = root();
    // the reports of the records whose first frame's baseline names a judge: id -> defect
    let mut reported: BTreeMap<String, String> = BTreeMap::new();
    for entry in fs::read_dir(root.join("store/runs")).unwrap().flatten() {
        // the first frame alone decides, so the other records are not read whole
        let judged = first_frame(&entry.path())
            .and_then(|f| f.pointer("/payload/baseline/judge").cloned())
            .is_some();
        if !judged {
            continue;
        }
        for e in frames(&entry.path())
            .iter()
            .filter(|e| e["kind"] == "DISCREPANCY_REPORTED")
        {
            let p = &e["payload"];
            reported.insert(
                p["discrepancy_id"].as_str().unwrap().to_string(),
                defect(p["first_divergence"].as_str().unwrap()),
            );
        }
    }
    let text = fs::read_to_string(root.join("store/case_runner/discrepancy-ledger.json")).unwrap();
    let ledger: Value = serde_json::from_str(&text).unwrap();
    let ledger = ledger.as_object().unwrap();
    let defects: BTreeSet<&String> = reported.values().collect();
    let entries: BTreeSet<&String> = ledger.keys().collect();
    assert_eq!(
        entries, defects,
        "the ledger's entries are not the defects the records report"
    );
    let mut placed: BTreeMap<&str, &str> = BTreeMap::new();
    for (key, entry) in ledger {
        for r in entry["reports"].as_array().unwrap() {
            let id = r["discrepancy_id"].as_str().unwrap();
            assert!(placed.insert(id, key).is_none(), "report {id} is in two entries");
            assert_eq!(
                reported.get(id).map(String::as_str),
                Some(key.as_str()),
                "report {id} is in {key}"
            );
        }
    }
    assert_eq!(placed.len(), reported.len(), "a report is in no entry");
    // every key is "<unit> <field>": a first divergence worded so `defect` finds no field falls back to the whole
    // text, which would hide a split defect; the form is held here, as `defect` cannot hold itself (reviewer-3's Low 2
    // of its review at Sprint 223)
    for (key, entry) in ledger {
        let unit = entry["unit"].as_str().unwrap();
        let field = key.strip_prefix(&format!("{unit} "));
        assert!(
            field.is_some_and(|f| !f.is_empty() && !f.contains(':')),
            "ledger entry {key:?} is not \"{unit} <field>\""
        );
    }
    let open: Vec<&String> = ledger
        .iter()
        .filter(|(_, v)| v["resolved_in"].is_null())
        .map(|(k, _)| k)
        .collect();
    println!(
        "{} reports, {} defects, {} open: {open:?}",
        reported.len(),
        ledger.len(),
        open.len()
    );
}
