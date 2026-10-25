// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 136's judge: the monitor code written from Rust, run in a Rust VM, dispatches, misses and takes faults as
//! the Python host's did (`vm_backend.py` 4217-4258, 4343-4400, 5297-5299).
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

mod common;

use common::*;
use guest_runtime::layout::{MONITOR_DATA, MONITOR_VA, SAVE_AREA};
use hypervisor::{Reg, Vcpu, Vm};
use w2m_vm::monitor::{map, set_system_registers};
use w2m_vm::monitor_code::{CODE_WRITE_ESR, CODE_WRITE_ESR_MASK, HVC_CODE_WRITE, HVC_FAULT_GIVE_BACK, insert};

/// Two targets whose pairs differ, one in its pair's first slot, one in its second, and one never inserted.
const TARGET_FIRST: u64 = 0x0000_7FF3_4567_89A0;
const TARGET_SECOND: u64 = 0x0000_0001_4000_2000;
const TARGET_MISSING: u64 = 0x0040_1000;
/// An address no map covers.
const UNMAPPED: u64 = 0x20_0000_0000;

const SAVE: u64 = MONITOR_DATA + SAVE_AREA;

#[test]
fn the_monitor_code_dispatches_misses_and_takes_faults() {
    let (mut area, code) = area();
    insert(area.memory.as_mut_slice(), TARGET_FIRST, MONITOR_VA + CODE_A, false);
    insert(area.memory.as_mut_slice(), TARGET_SECOND, MONITOR_VA + CODE_B, true);

    let vm = Vm::new(40).unwrap();
    map(&vm, &area).unwrap();
    let vcpu = Vcpu::new(&vm).unwrap();
    set_system_registers(&vcpu, area.ttbr).unwrap();
    let x = |r: u8| vcpu.reg(Reg::X(r)).unwrap();

    // a hit in the first slot, then in the second
    start(&vcpu, code.dispatch_va, &[(17, TARGET_FIRST)]);
    assert_eq!(
        run_to_hvc(&vcpu),
        Ok(HVC_A),
        "{TARGET_FIRST:#x} from its pair's first slot"
    );
    start(&vcpu, code.dispatch_va, &[(17, TARGET_SECOND)]);
    assert_eq!(
        run_to_hvc(&vcpu),
        Ok(HVC_B),
        "{TARGET_SECOND:#x} from its pair's second slot"
    );

    // a miss with the in-VM compiler off: the host's hvc #1, x17 the target
    start(&vcpu, code.dispatch_va, &[(17, TARGET_MISSING)]);
    assert_eq!(run_to_hvc(&vcpu), Ok(1), "{TARGET_MISSING:#x} misses");
    assert_eq!(x(17), TARGET_MISSING);
    assert_eq!(
        vcpu.reg(Reg::Pc).unwrap(),
        code.miss_va + 4,
        "the miss's hvc is the miss stub's first word"
    );

    // a write to a read-only page: hvc #13 with x17 the address; answered "no blocks dropped", the fault goes on to
    // the save and hvc #14
    start(
        &vcpu,
        MONITOR_VA + STORE,
        &[(0, 0x5A), (1, READ_ONLY_VA), (5, 0x55), (17, 0x1717), (30, 0x3030)],
    );
    assert_eq!(
        run_to_hvc(&vcpu),
        Ok(HVC_CODE_WRITE as u16),
        "a write to a read-only page"
    );
    assert_eq!(x(17), READ_ONLY_VA);
    vcpu.set_reg(Reg::X(17), 0).unwrap();
    assert_eq!(run_to_hvc(&vcpu), Ok(HVC_FAULT_GIVE_BACK as u16));
    let saved = |r: u64| area.read_u64(SAVE + 8 * r);
    assert_eq!(
        [saved(1), saved(5), saved(17), saved(30)],
        [READ_ONLY_VA, 0x55, 0x1717, 0x3030]
    );
    let (esr, far, elr) = (
        area.read_u64(SAVE + 0x100),
        area.read_u64(SAVE + 0x108),
        area.read_u64(SAVE + 0x110),
    );
    assert_eq!(esr & CODE_WRITE_ESR_MASK, CODE_WRITE_ESR, "ESR {esr:#x}");
    assert_eq!((far, elr), (READ_ONLY_VA, MONITOR_VA + STORE));
    assert_eq!(area.read_u64(0x3000), 0, "the read-only page was written");

    // any other fault: the save, then hvc #14
    start(&vcpu, MONITOR_VA + LOAD, &[(1, UNMAPPED), (7, 0x77), (17, 0x1818)]);
    assert_eq!(
        run_to_hvc(&vcpu),
        Ok(HVC_FAULT_GIVE_BACK as u16),
        "a load from {UNMAPPED:#x}"
    );
    assert_eq!([saved(1), saved(7), saved(17)], [UNMAPPED, 0x77, 0x1818]);
    let (esr, far, elr) = (
        area.read_u64(SAVE + 0x100),
        area.read_u64(SAVE + 0x108),
        area.read_u64(SAVE + 0x110),
    );
    assert_eq!(esr >> 26, 0x25, "a data abort from the current EL: ESR {esr:#x}");
    assert_ne!(esr & CODE_WRITE_ESR_MASK, CODE_WRITE_ESR);
    assert_eq!((far, elr), (UNMAPPED, MONITOR_VA + LOAD));
    println!(
        "monitor_code_in_vm: two hits, a miss, a code write and a fault, as the Python host's monitor code ran them"
    );
}
