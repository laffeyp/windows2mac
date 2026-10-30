// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The walk's rules judged on every target (Sprint 254; Deferred (Sprint 253)): each of the 280 images under
//! `store/binaries/` walks to its counts in `store/translation-cache/walk-fixtures-2026-10-30.json`, 641,041
//! instructions and 116 switch tables in all, so a change to the walk fails CI as well as the depot's test.

use std::fs;
use std::path::{Path, PathBuf};

const STORED: &str = "store/translation-cache/walk-fixtures-2026-10-30.json";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

#[test]
fn every_fixture_image_walks_to_its_stored_counts() {
    let root = root();
    let stored: serde_json::Value = serde_json::from_str(&fs::read_to_string(root.join(STORED)).unwrap()).unwrap();
    let stored = stored.as_object().unwrap();
    let mut walked = 0;
    let mut wrong = Vec::new();
    for entry in fs::read_dir(root.join("store/binaries")).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let data = fs::read(&path).unwrap();
        let img = pe_image::parse(&data, false).unwrap();
        let w = image_walk::walk(&img, &data);
        let ours = serde_json::json!({
            "reachable_instructions": w.insns.len(),
            "walked": w.insns.len() - w.swept_only.len(),
            "swept_only": w.swept_only.len(),
            "jump_tables": w.tables.len(),
            "jump_table_entries": w.tables.values().map(Vec::len).sum::<usize>(),
            "block_starts": w.block_starts.len(),
            "undecodable": w.undecodable.len(),
        });
        if stored.get(&name) != Some(&ours) {
            wrong.push(format!("{name}: {ours} where {STORED} holds {:?}", stored.get(&name)));
        }
        walked += 1;
    }
    assert_eq!(walked, stored.len(), "images walked against images stored");
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
