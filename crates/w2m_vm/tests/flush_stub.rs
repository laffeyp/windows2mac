// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The flush a block split runs (Sprint 307; reviewer-3's Low 1 at 306): `run::flush_through_stub` gives the vCPU back
//! every general register, CPSR and the PC as they were, and absorbs a cancel that is pending when it runs, which
//! `hv_vcpu.h` 377-378 states makes the next `hv_vcpu_run` return at once. Neither needs a second thread.
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use crate::common::*;
use hypervisor::{Reg, Vcpu, Vm};
use w2m_vm::CPSR_EL1H;
use w2m_vm::monitor::{map, set_system_registers};
use w2m_vm::run::{FlushCounts, flush_through_stub};

/// x0 to x30 each a value of its own, so a register restored from another's slot is told apart (reviewer-3, by
/// message at 306); CPSR EL1h with N, Z, C and V set; the PC one of the area's stubs.
fn set(vcpu: &Vcpu<'_>) -> Vec<u64> {
    let mut state = Vec::new();
    for i in 0..31u8 {
        let v = 0x5EED_0000_0000_0000 | (u64::from(i) << 8) | u64::from(i);
        vcpu.set_reg(Reg::X(i), v).unwrap();
        state.push(v);
    }
    let cpsr = CPSR_EL1H | 0xF000_0000;
    vcpu.set_reg(Reg::Cpsr, cpsr).unwrap();
    state.push(cpsr);
    let pc = guest_runtime::layout::MONITOR_VA + CODE_A;
    vcpu.set_reg(Reg::Pc, pc).unwrap();
    state.push(pc);
    state
}

fn read(vcpu: &Vcpu<'_>) -> Vec<u64> {
    let mut state: Vec<u64> = (0..31u8).map(|i| vcpu.reg(Reg::X(i)).unwrap()).collect();
    state.push(vcpu.reg(Reg::Cpsr).unwrap());
    state.push(vcpu.reg(Reg::Pc).unwrap());
    state
}

/// The flush on a stopped vCPU, a cancel sent first when `cancel`: the state before, the state after, the counts.
fn flushed(cancel: bool) -> (Vec<u64>, Vec<u64>, FlushCounts) {
    let (area, _) = area();
    let vm = Vm::new(40).unwrap();
    map(&vm, &area).unwrap();
    let vcpu = Vcpu::new(&vm).unwrap();
    set_system_registers(&vcpu, area.ttbr).unwrap();
    let before = set(&vcpu);
    if cancel {
        vcpu.handle().stop().unwrap();
    }
    let counts = FlushCounts::default();
    flush_through_stub(&vcpu, &counts).unwrap();
    (before, read(&vcpu), counts)
}

/// The name of the first of the 33 values that differs, or None.
fn first_wrong(before: &[u64], after: &[u64]) -> Option<String> {
    let name = |i: usize| match i {
        31 => "CPSR".to_string(),
        32 => "PC".to_string(),
        _ => format!("x{i}"),
    };
    before
        .iter()
        .zip(after)
        .position(|(b, a)| b != a)
        .map(|i| format!("{}: 0x{:x} set, 0x{:x} back", name(i), before[i], after[i]))
}

#[test]
fn the_flush_gives_back_every_register_cpsr_and_the_pc() {
    let _one = crate::one_vm();
    let (before, after, counts) = flushed(false);
    assert_eq!(first_wrong(&before, &after), None, "the vCPU's state after the flush");
    assert_eq!((counts.flushes.get(), counts.cancelled.get()), (1, false));
}

#[test]
fn a_pending_cancel_is_absorbed_and_reported() {
    let _one = crate::one_vm();
    let (before, after, counts) = flushed(true);
    assert_eq!(
        first_wrong(&before, &after),
        None,
        "the vCPU's state after the cancelled flush"
    );
    assert_eq!(
        (counts.flushes.get(), counts.cancelled.get()),
        (1, true),
        "the flush run once and the cancel noted for after the split"
    );
}
