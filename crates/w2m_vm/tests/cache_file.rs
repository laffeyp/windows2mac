// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The translation cache's file (Sprint 254; the design's card 3), judged on every target: a cache of each image under
//! `store/binaries/`, made with stand-in exits (the words embed them; liveness and the page index do not), has every
//! block's live flags covering its successors' reads, every page a block's bytes touch listing the block, and its
//! file read back equal; and the stored figures of UnityPlayer.dll's cache add up to the walk's starts within the
//! regions.

use std::fs;
use std::path::{Path, PathBuf};

use guest_runtime::layout::{CACHE_CODE_SIZE, CACHE_FILE_SIZE, CODE_CACHE, MONITOR_VA, PAGE, PROCESS_VA, STATE_OFF};
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

/// Sprint 265: the key hashes every crate the cached words are made from: the maker's four and each crate their
/// manifests reach by `path =`, and the maker's own file.
#[test]
fn the_key_covers_every_crate_the_cache_is_made_from() {
    let root = root();
    let mut reached: Vec<String> = ["guest_runtime", "pe_image", "translator", "image_walk"]
        .map(String::from)
        .to_vec();
    let mut i = 0;
    while i < reached.len() {
        let manifest = fs::read_to_string(root.join("crates").join(&reached[i]).join("Cargo.toml")).unwrap();
        for line in manifest.lines() {
            if let Some((_, rest)) = line.split_once("path = \"../")
                && let Some((name, _)) = rest.split_once('"')
                && !reached.iter().any(|r| r == name)
            {
                reached.push(name.to_string());
            }
        }
        i += 1;
    }
    for name in &reached {
        let path = format!("crates/{name}");
        assert!(
            cache_file::KEY_INPUTS.contains(&path.as_str()),
            "{path}, which the cache is made from, is not among the key's inputs"
        );
    }
    assert!(cache_file::KEY_INPUTS.contains(&"crates/w2m_vm/src/cache_file.rs"));
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
        // the fix-ups (Sprint 261, card 5's II): each block's list is the words `relocate` changes in a move
        for b in &cache.blocks {
            let from = guest_runtime::layout::CACHE_CODE_VA + 4 * u64::from(b.word);
            let words = &cache.words[b.word as usize..(b.word + b.nwords) as usize];
            let Ok(moved) = cache_file::relocate(words, from, to) else {
                continue;
            };
            let changed: Vec<u32> = (0..words.len() as u32)
                .filter(|&k| moved[k as usize] != words[k as usize])
                .collect();
            let listed = &cache.fixups[b.fixup as usize..(b.fixup + b.nfixups) as usize];
            if listed != changed.as_slice() {
                wrong.push(format!(
                    "{name}: block 0x{:x}'s fix-ups {listed:?}, where the move changes {changed:?}",
                    b.start
                ));
            }
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
    assert!(
        f["file_bytes"].as_u64().unwrap() <= CACHE_FILE_SIZE,
        "the file's region"
    );
    assert_eq!(
        f["uncovered_reads"], 0,
        "blocks whose live flags miss a successor's read"
    );
}
