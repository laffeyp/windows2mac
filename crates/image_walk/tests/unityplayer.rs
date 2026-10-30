// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! UnityPlayer.dll walked (Sprint 253): the walk's counts are the stored ones, `store/translation-cache/walk-
//! unityplayer-2026-10-30.json`, and against the frozen census's (`store/census/static/instructions-hail-to-the-
//! rainbow.json`) its tables and their entries and the sweep's instructions are equal, and its instructions fewer by
//! the walk's stops at bytes w2m's decoder refuses, each of which LLVM's `objdump` also refuses (2026-10-30). The depot
//! is on the machine that runs the gates; elsewhere the test says so and holds the stored file alone.

use std::fs;
use std::path::{Path, PathBuf};

const STORED: &str = "store/translation-cache/walk-unityplayer-2026-10-30.json";
const CENSUS: &str = "store/census/static/instructions-hail-to-the-rainbow.json";
/// The census's image: UnityPlayer.dll of build 24247624.
const SHA256: &str = "6f2a1adb75d3f85449441307f51ef746cf6d293923be36611e0e764c1c919cc2";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn json(path: &str) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(root().join(path)).unwrap()).unwrap()
}

/// The census's figures for UnityPlayer.dll.
fn census() -> serde_json::Value {
    json(CENSUS)["images"]
        .as_object()
        .and_then(|images| images.values().find(|i| i["file"] == "UnityPlayer.dll"))
        .cloned()
        .expect("the census holds UnityPlayer.dll")
}

/// The stored walk against the census: the same image, the same tables and sweep, and fewer instructions by what the
/// walk's refusals cut, each a refusal (48 of them).
#[test]
fn the_stored_walk_is_the_censuss_less_its_refused_paths() {
    let (walk, census) = (json(STORED), census());
    assert_eq!(census["sha256"], SHA256);
    for key in ["jump_tables", "jump_table_entries"] {
        assert_eq!(walk[key], census[key], "{key}");
    }
    let census_swept = census["reachable_instructions"].as_u64().unwrap() - census["walked"].as_u64().unwrap();
    assert_eq!(
        walk["swept_only"].as_u64(),
        Some(census_swept),
        "the sweep's instructions"
    );
    let ours = walk["reachable_instructions"].as_u64().unwrap();
    let theirs = census["reachable_instructions"].as_u64().unwrap();
    // fewer, by the paths the walk's refusals cut; equal would need no refusal
    assert!(ours < theirs, "{ours} instructions against the census's {theirs}");
    assert_eq!(walk["undecodable"].as_u64(), Some(48), "the walk's refusals");
}

/// The walk of the depot's image gives the stored counts.
#[test]
fn the_depot_image_walks_to_the_stored_counts() {
    let home = std::env::var("HOME").unwrap_or_default();
    let path = Path::new(&home).join("w2m-depots/hail-to-the-rainbow/UnityPlayer.dll");
    let Ok(data) = fs::read(&path) else {
        eprintln!(
            "{} is not here: the stored walk is held to the census alone",
            path.display()
        );
        return;
    };
    let img = pe_image::parse(&data, false).unwrap();
    let w = image_walk::walk(&img, &data);
    let stored = json(STORED);
    let ours = serde_json::json!({
        "reachable_instructions": w.insns.len(),
        "walked": w.insns.len() - w.swept_only.len(),
        "swept_only": w.swept_only.len(),
        "jump_tables": w.tables.len(),
        "jump_table_entries": w.tables.values().map(Vec::len).sum::<usize>(),
        "block_starts": w.block_starts.len(),
        "undecodable": w.undecodable.len(),
    });
    for (key, value) in ours.as_object().unwrap() {
        assert_eq!(&stored[key], value, "{key}");
    }
}
