// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 132: the time of one exit and its resume, from Rust, against the C monitor's 0.72 µs
//! (`scripts/measure_exit.py`, 2026-10-14, quoted in `tools/vm_monitor/vm_monitor.c`).
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use std::time::Instant;

use hypervisor::{Exit, GuestMemory, Reg, Vcpu, Vm, memory};

const CODE_IPA: u64 = 0x1_0000;
const EXITS: u32 = 100_000;

#[test]
fn an_exit_and_its_resume_take_about_a_microsecond() {
    let mut mem = GuestMemory::new(64 * 1024).unwrap();
    mem.write_words(
        0,
        &[
            0xd400_0002, // hvc #0
            0x17ff_ffff, // b .-4
        ],
    );
    let vm = Vm::new(40).unwrap();
    vm.map(&mem, CODE_IPA, memory::READ | memory::EXEC).unwrap();
    let vcpu = Vcpu::new(&vm).unwrap();
    vcpu.set_reg(Reg::Pc, CODE_IPA).unwrap();
    vcpu.set_reg(Reg::Cpsr, 0x3c5).unwrap();
    let mut hvcs = 0;
    let started = Instant::now();
    while hvcs < EXITS {
        match vcpu.run().unwrap() {
            e if e.hvc_immediate() == Some(0) => hvcs += 1,
            Exit::VtimerActivated => vcpu.set_vtimer_mask(true).unwrap(),
            other => panic!("unexpected exit {other:?}"),
        }
    }
    let ns = started.elapsed().as_nanos() as f64 / f64::from(EXITS);
    println!("exit_time: {EXITS} exits, {ns:.0} ns each (the C monitor's: 720 ns)");
    assert!(ns < 5_000.0, "{ns:.0} ns an exit");
}
