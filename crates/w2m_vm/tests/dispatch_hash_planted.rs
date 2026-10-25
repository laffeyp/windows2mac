// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 136's planted fault: the dispatcher's hash with its second shift changed from `2 * DISPATCH_BITS` to
//! `2 * DISPATCH_BITS + 1`. A target inserted where `dispatch_slot` puts it then misses, and the judge names it.
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

mod common;

use a64_encoder as a64;
use common::*;
use guest_runtime::layout::{DISPATCH_BITS, MONITOR_VA};
use hypervisor::{Vcpu, Vm};
use w2m_vm::DISPATCHER;
use w2m_vm::monitor::{map, set_system_registers};
use w2m_vm::monitor_code::{dispatch_slot, insert};

const TARGET: u64 = 0x0000_7FF3_4567_89A0;

/// `dispatch_slot` with the planted shift.
fn planted_slot(t: u64) -> u64 {
    (t ^ t >> DISPATCH_BITS ^ t >> (2 * DISPATCH_BITS + 1)) & ((1 << DISPATCH_BITS) - 2)
}

#[test]
fn a_changed_shift_misses_the_inserted_target() {
    assert_ne!(
        dispatch_slot(TARGET),
        planted_slot(TARGET),
        "the target does not tell the two hashes apart"
    );
    let (mut area, code) = area();
    insert(area.memory.as_mut_slice(), TARGET, MONITOR_VA + CODE_A, false);
    // the dispatcher's third word: eor x18, x18, x17, lsr #(2 * DISPATCH_BITS)
    let at = (DISPATCHER + 8) as usize;
    let word = |bytes: &[u8]| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    assert_eq!(
        word(area.memory.as_slice()),
        a64::eor(18, 18, 17, a64::LSR, 2 * DISPATCH_BITS as u32)
    );
    let planted = a64::eor(18, 18, 17, a64::LSR, 2 * DISPATCH_BITS as u32 + 1);
    area.memory.as_mut_slice()[at..at + 4].copy_from_slice(&planted.to_le_bytes());

    let vm = Vm::new(40).unwrap();
    map(&vm, &area).unwrap();
    let vcpu = Vcpu::new(&vm).unwrap();
    set_system_registers(&vcpu, area.ttbr).unwrap();
    start(&vcpu, code.dispatch_va, &[(17, TARGET)]);
    let got = run_to_hvc(&vcpu);
    assert_eq!(got, Ok(1), "the planted hash still reached {TARGET:#x}'s code");
    println!(
        "dispatch_hash_planted: {TARGET:#x} inserted at slot {:#x} missed under the planted hash",
        dispatch_slot(TARGET)
    );
}
