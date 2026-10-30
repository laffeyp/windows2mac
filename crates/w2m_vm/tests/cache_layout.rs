// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The translation cache's regions (Sprint 254; `docs/design-translation-cache-2026-10-30.md`, B and B'): just below
//! the monitor area, each other's neighbours, overlapping no area the guest or the loader maps, and the code region
//! within a branch's reach of every exit its words take. `MONITOR_SIZE` and the runtime image's addresses stay as
//! `guest_runtime::layout`'s own test holds them, since the regions lie outside the monitor area.

use guest_runtime::layout::{
    BRANCH_REACH, CACHE_CODE_SIZE, CACHE_CODE_VA, CACHE_DATA_SIZE, CACHE_DATA_VA, CODE_CACHE, HEAP_SIZE, HEAP_VA,
    MONITOR_SIZE, MONITOR_VA, PROCESS_VA,
};

/// The slice stub is the code area's first stub after the bridges (`guest.rs` 926); the bridges take far less than
/// this, which `w2m-vm --make-cache` checks against the run's own `slice_va`.
const SLICE_STUB_BOUND: u64 = MONITOR_VA + CODE_CACHE + 0x10_0000;

fn overlaps(a: (u64, u64), b: (u64, u64)) -> bool {
    a.0 < b.0 + b.1 && b.0 < a.0 + a.1
}

#[test]
fn the_cache_regions_lie_below_the_monitor_area_and_overlap_nothing() {
    assert_eq!(
        CACHE_CODE_VA + CACHE_CODE_SIZE,
        MONITOR_VA,
        "the code region ends at the monitor area"
    );
    assert_eq!(
        CACHE_DATA_VA + CACHE_DATA_SIZE,
        CACHE_CODE_VA,
        "the data region ends at the code region"
    );
    assert_eq!(
        MONITOR_SIZE, 0xAA0_0000,
        "the monitor area's size, which the runtime image links against"
    );
    let regions = [(CACHE_CODE_VA, CACHE_CODE_SIZE), (CACHE_DATA_VA, CACHE_DATA_SIZE)];
    let others = [
        ("the monitor area", (MONITOR_VA, MONITOR_SIZE)),
        (
            "the process area, from PROCESS_VA up",
            (PROCESS_VA, u64::MAX - PROCESS_VA),
        ),
        ("the heap", (HEAP_VA, HEAP_SIZE)),
        ("the VirtualAlloc arena", (w2m_vm::VIRTUAL_VA, w2m_vm::VIRTUAL_SIZE)),
        (
            "the DLLs the loader places, from DLL_BASE up",
            (pe_loader::DLL_BASE, u64::MAX - pe_loader::DLL_BASE),
        ),
    ];
    for r in regions {
        for (name, area) in others {
            assert!(
                !overlaps(r, area),
                "a cache region 0x{:x}+0x{:x} overlaps {name}",
                r.0,
                r.1
            );
        }
    }
}

#[test]
fn every_exit_is_within_a_branchs_reach_of_the_cache_code() {
    for (name, target) in [
        ("the dispatcher", MONITOR_VA + w2m_vm::DISPATCHER),
        ("the slice stub's bound", SLICE_STUB_BOUND),
    ] {
        assert!(target - CACHE_CODE_VA < BRANCH_REACH, "{name} at 0x{target:x}");
    }
}
