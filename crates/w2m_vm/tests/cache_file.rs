// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The translation cache's file (Sprint 254; the design's card 3), judged on every target: a cache of each image under
//! `store/binaries/`, made with stand-in exits (the words embed them; liveness and the page index do not), has every
//! block's live flags covering its successors' reads, every page a block's bytes touch listing the block, and its
//! file read back equal; and the stored figures of UnityPlayer.dll's cache add up to the walk's starts within the
//! regions.

use std::fs;
use std::path::{Path, PathBuf};

use guest_runtime::layout::{CACHE_CODE_SIZE, CACHE_DATA_SIZE, CODE_CACHE, MONITOR_VA, PAGE, PROCESS_VA, STATE_OFF};
use w2m_vm::cache_file::{self, Exits};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn exits() -> Exits {
    Exits {
        exit_va: MONITOR_VA + w2m_vm::DISPATCHER,
        slice_va: MONITOR_VA + CODE_CACHE,
        state_va: PROCESS_VA + STATE_OFF,
        runtime: translator::lower::RuntimeCalls::default(),
    }
}

#[test]
fn every_fixture_cache_covers_its_reads_indexes_its_pages_and_reads_back() {
    let mut blocks = 0;
    let mut wrong = Vec::new();
    for entry in fs::read_dir(root().join("store/binaries")).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let data = fs::read(&path).unwrap();
        let img = pe_image::parse(&data, false).unwrap();
        let (cache, _) = cache_file::make(&img, &data, img.image_base, &exits(), "0".repeat(64));
        blocks += cache.blocks.len();
        let uncovered = cache_file::uncovered_reads(&img, &data, &cache);
        if !uncovered.is_empty() {
            wrong.push(format!("{name}: live flags miss a successor's read at {uncovered:x?}"));
        }
        for (i, b) in cache.blocks.iter().enumerate() {
            let mut page = u64::from(b.start) & !(PAGE - 1);
            while page < u64::from(b.end) {
                if !cache.pages.get(&(page as u32)).is_some_and(|l| l.contains(&(i as u32))) {
                    wrong.push(format!("{name}: block 0x{:x} not in page 0x{page:x}'s list", b.start));
                }
                page += PAGE;
            }
        }
        // the relocation (Sprint 256): every block moved into the code area equals its assembly there
        let to = MONITOR_VA + CODE_CACHE + 0x10_0000;
        let (_, unmovable, differ) = cache_file::relocation_census(&img, &data, &cache, &exits(), to);
        if !unmovable.is_empty() || !differ.is_empty() {
            wrong.push(format!(
                "{name}: relocation refused {unmovable:?}, differing at {differ:x?}"
            ));
        }
        if cache_file::from_bytes(&cache_file::to_bytes(&cache)).as_ref() != Ok(&cache) {
            wrong.push(format!("{name}: the file does not read back equal"));
        }
    }
    assert!(blocks > 100_000, "{blocks} blocks over the fixtures");
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_stored_cache_figures_add_up_within_their_regions() {
    let text = fs::read_to_string(root().join("store/translation-cache/cache-unityplayer-2026-10-30.json")).unwrap();
    let f: serde_json::Value = serde_json::from_str(&text).unwrap();
    let walk: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root().join("store/translation-cache/walk-unityplayer-2026-10-30.json")).unwrap(),
    )
    .unwrap();
    let refused: u64 = f["refused"]
        .as_object()
        .unwrap()
        .values()
        .filter_map(|v| v.as_u64())
        .sum();
    assert_eq!(
        f["blocks"].as_u64().unwrap() + refused,
        walk["block_starts"].as_u64().unwrap()
    );
    assert!(f["code_bytes"].as_u64().unwrap() <= CACHE_CODE_SIZE, "the code region");
    assert!(f["data_bytes"].as_u64().unwrap() <= CACHE_DATA_SIZE, "the data region");
    assert_eq!(
        f["uncovered_reads"], 0,
        "blocks whose live flags miss a successor's read"
    );
}
