// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The walk's rules judged on every target (Sprint 254; Deferred (Sprint 253)): each of the 288 images under
//! `store/binaries/` walks to its counts in `store/translation-cache/walk-fixtures-2026-10-30.json`, 644,877
//! instructions and 116 switch tables in all, so a change to the walk fails CI as well as the depot's test. A binary
//! added to `store/binaries/` adds its entry, the `image_walk` binary's counts (Sprint 331: `probes/locked_race`'s,
//! the 281st; 280 and 641,041 before; Sprint 344: `probes/thread_values_race`'s, the 282nd; 281 and 641,343 before;
//! Sprint 345: `probes/thread_slots_race`'s, the 283rd; 282 and 641,894 before; Sprint 347: `probes/tls_free_reach`'s,
//! the 284th; 283 and 642,646 before; and `probes/tls_expansion_first`'s, the 285th; 284 and 643,087 before; Sprint
//! 350: `probes/wait_closed_handle`'s, the 286th; 285 and 643,344 before; Sprint 352: `probes/wait_closed_kinds`'s, the
//! 287th; 286 and 643,637 before; Sprint 356: `probes/mutex_abandoned`'s, the 288th; 287 and 644,291 before).

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
