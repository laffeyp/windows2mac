// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 132: a vCPU spinning in the guest is stopped from another thread by `hv_vcpus_exit`, the kick the host
//! design gives a wake that reaches a vCPU running guest code.
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use std::time::{Duration, Instant};

use hypervisor::{Exit, GuestMemory, Reg, Vcpu, Vm, memory};

const CODE_IPA: u64 = 0x1_0000;

#[test]
fn a_spinning_vcpu_stopped_from_another_thread_returns_canceled() {
    let mut mem = GuestMemory::new(64 * 1024).unwrap();
    mem.write_words(0, &[0x1400_0000]); // b . (clang, 2026-10-25)
    let vm = Vm::new(40).unwrap();
    vm.map(&mem, CODE_IPA, memory::READ | memory::EXEC).unwrap();
    let vcpu = Vcpu::new(&vm).unwrap();
    vcpu.set_reg(Reg::Pc, CODE_IPA).unwrap();
    vcpu.set_reg(Reg::Cpsr, 0x3c5).unwrap();
    let handle = vcpu.handle();
    let started = Instant::now();
    let exit = std::thread::scope(|s| {
        s.spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            handle.stop().unwrap();
        });
        loop {
            match vcpu.run().unwrap() {
                Exit::VtimerActivated => vcpu.set_vtimer_mask(true).unwrap(),
                other => break other,
            }
        }
    });
    assert_eq!(exit, Exit::Canceled);
    assert!(
        started.elapsed() >= Duration::from_millis(50),
        "returned before the stop: {:?}",
        started.elapsed()
    );
    assert_eq!(vcpu.reg(Reg::Pc).unwrap(), CODE_IPA, "the vCPU was still in its loop");
}
