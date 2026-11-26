// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 348's built-image judge: in the runtime image clang builds, TlsAlloc's and TlsFree's load of the run table's
//! count (`thread_storage.c` `RUN_COUNT`, at `W2M_RUN_TABLE_VA`) is a load-acquire, pairing with the host's `Release`
//! store of the count in `threads::add_thread` after a new entry's words. The two services take the slots' lock, not
//! the runtime word the host's writer holds, so only the acquire orders the entry's TEB and status after the count.
//! `__ATOMIC_ACQUIRE` builds to `ldapr`, the RCpc load-acquire, on this target (`arm64-apple-macos14`, whose apple-m1
//! has FEAT_LRCPC; LLVM D126250 selects LDAPR for an acquire load with +rcpc), and to `ldar` without it; either orders
//! every later load after the count, which is all the reader needs. Xcode's clang and `ld` build the image, so the test
//! runs on macOS.
#![cfg(target_os = "macos")]

use std::path::{Path, PathBuf};

use guest_runtime::build;
use guest_runtime::layout::{MONITOR_VA, RUN_TABLE, RUNTIME_IMAGE_VA, runtime_defines};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// A load of a 64-bit register from the address a register holds, as the decoder below finds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Load {
    /// `ldar xT, [xN]`: load-acquire
    Acquire,
    /// `ldapr xT, [xN]` or `ldapur xT, [xN, #imm]`: load-acquire RCpc
    AcquirePc,
    /// `ldr xT, [xN, #imm]` or `ldur xT, [xN, #imm]` at offset 0: a plain load
    Plain,
}

/// Each 64-bit load from `target` in `words` (the function's instructions from `at`): its address and kind. The base
/// register's value is followed through `movz` and `movk` (the form clang builds a 48-bit constant in), and forgotten
/// at any other instruction that writes the register: a data-processing instruction's or a load's destination.
fn loads_of(words: &[u32], at: u64, target: u64) -> Vec<(u64, Load, u32)> {
    let mut held: [Option<u64>; 32] = [None; 32];
    let mut out = Vec::new();
    for (i, &w) in words.iter().enumerate() {
        let (rd, rn) = ((w & 31) as usize, ((w >> 5) & 31) as usize);
        let hw = u64::from((w >> 21) & 3) * 16;
        let imm16 = u64::from((w >> 5) & 0xFFFF);
        let pc = at + 4 * i as u64;
        if w & 0xFF80_0000 == 0xD280_0000 {
            held[rd] = Some(imm16 << hw); // movz xD, #imm16, lsl #hw
            continue;
        }
        if w & 0xFF80_0000 == 0xF280_0000 {
            held[rd] = held[rd].map(|v| v & !(0xFFFF << hw) | imm16 << hw); // movk xD, #imm16, lsl #hw
            continue;
        }
        let imm9 = (((w >> 12) & 0x1FF) as i64).wrapping_shl(55).wrapping_shr(55) as u64;
        let kind = if w & 0xFFFF_FC00 == 0xC8DF_FC00 {
            Some((Load::Acquire, 0)) // ldar xT, [xN]
        } else if w & 0xFFFF_FC00 == 0xF8BF_C000 {
            Some((Load::AcquirePc, 0)) // ldapr xT, [xN]
        } else if w & 0xFFE0_0C00 == 0xD940_0000 {
            Some((Load::AcquirePc, imm9)) // ldapur xT, [xN, #imm9]
        } else if w & 0xFFC0_0000 == 0xF940_0000 {
            Some((Load::Plain, u64::from((w >> 10) & 0xFFF) * 8)) // ldr xT, [xN, #imm12 * 8]
        } else if w & 0xFFE0_0C00 == 0xF840_0000 {
            Some((Load::Plain, imm9)) // ldur xT, [xN, #imm9]
        } else {
            None
        };
        if let Some((load, offset)) = kind
            && held[rn].is_some_and(|base| base.wrapping_add(offset) == target)
        {
            out.push((pc, load, w));
        }
        // the destination forgotten: data processing (op0 100x, x101) and loads (op0 x1x0 with L, bit 22, set)
        let op0 = (w >> 25) & 0xF;
        let writes = op0 & 0b1110 == 0b1000 || op0 & 0b0111 == 0b0101 || (op0 & 0b0101 == 0b0100 && w >> 22 & 1 == 1);
        if writes && rd != 31 {
            held[rd] = None;
        }
    }
    out
}

/// TlsAlloc and TlsFree, `tls_clear_everywhere` inlined in each or called by each: every load of the count is a
/// load-acquire, `ldar` or `ldapr`, and there is one at least in each service or the function it calls.
#[test]
fn tls_alloc_and_tls_free_read_the_run_tables_count_with_a_load_acquire() {
    let root = root();
    let (key, image) = build(&root, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    let count = MONITOR_VA + RUN_TABLE;
    let mut starts: Vec<u64> = image.symbols.values().copied().collect();
    starts.sort_unstable();
    let body = |name: &str| -> (u64, Vec<u32>) {
        let at = image.symbols[name];
        let end = *starts.iter().find(|&&s| s > at).expect("a symbol after the function");
        let (base, bytes) = image
            .segments
            .iter()
            .find(|(va, b)| *va <= at && end <= va + b.len() as u64)
            .expect("the function's segment");
        let from = (at - base) as usize;
        let words = bytes[from..from + (end - at) as usize]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| u32::from_le_bytes(*c))
            .collect();
        (at, words)
    };
    let mut quoted = Vec::new();
    for service in ["k32_TlsAlloc", "k32_TlsFree"] {
        let mut found = Vec::new();
        for name in [service, "tls_clear_everywhere"] {
            if image.symbols.contains_key(name) {
                let (at, words) = body(name);
                found.extend(loads_of(&words, at, count).into_iter().map(|l| (name, l)));
            }
        }
        assert!(
            !found.is_empty(),
            "{service}: no load of the run table's count at 0x{count:x} (image {key})"
        );
        for (name, (pc, load, word)) in &found {
            assert_ne!(
                *load,
                Load::Plain,
                "{service}: {name}'s load of the run table's count at 0x{pc:x} is {word:#010x}, a plain load, not a \
                 load-acquire (ldar or ldapr) (image {key})"
            );
            quoted.push(format!("_{name} 0x{pc:x} {word:#010x} {load:?}"));
        }
    }
    println!(
        "run_count_load: image {key}; the count at 0x{count:x} read by a load-acquire at {}",
        quoted.join(", ")
    );
}

/// The decoder's own reach: a plain `ldr` from the count is found and named plain, an `ldar` or `ldapr` acquire, a load from
/// another address or through a register rewritten after its `movk` not at all (the words of today's `_k32_TlsAlloc`,
/// `mov x14, #0x9000; movk x14, #0x9db, lsl #16; movk x14, #0x7f00, lsl #32`, then the load).
#[test]
fn the_decoder_finds_the_counts_load_and_tells_ldar_from_ldr() {
    let count = MONITOR_VA + RUN_TABLE;
    let build = [0xd292_000e, 0xf2a1_3b6e, 0xf2cf_e00e];
    let with = |load: u32| [&build[..], &[load]].concat();
    assert_eq!(
        loads_of(&with(0xf940_01cc), 0, count),
        vec![(12, Load::Plain, 0xf940_01cc)],
        "ldr x12, [x14]"
    );
    assert_eq!(
        loads_of(&with(0xc8df_fdcc), 0, count),
        vec![(12, Load::Acquire, 0xc8df_fdcc)],
        "ldar x12, [x14]"
    );
    assert_eq!(
        loads_of(&with(0xf8bf_c1cc), 0, count),
        vec![(12, Load::AcquirePc, 0xf8bf_c1cc)],
        "ldapr x12, [x14]"
    );
    assert_eq!(
        loads_of(&with(0xf940_05cc), 0, count),
        vec![],
        "ldr x12, [x14, #8]: another word"
    );
    let moved = [&build[..], &[0x9104_01ce, 0xf940_01cc]].concat(); // add x14, x14, #0x100; ldr x12, [x14]
    assert_eq!(loads_of(&moved, 0, count), vec![], "the base moved by add");
}
