// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 132's expected values: a few arm64 instructions run on one vCPU from Rust, with the MMU off, at EL1. One VM
//! per process, so each scenario is its own test file.
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use hypervisor::{GuestMemory, Reg, Vcpu, Vm, memory};

const CODE_IPA: u64 = 0x1_0000;
/// EL1h with D, A, I and F masked.
const EL1H_MASKED: u64 = 0x3c5;

#[test]
fn an_hvc_exits_with_its_immediate_and_the_resumed_run_reaches_the_next() {
    // assembled by clang (--target=aarch64-linux-gnu) and read back with llvm-objdump, 2026-10-25
    let mut mem = GuestMemory::new(64 * 1024).unwrap();
    mem.write_words(
        0,
        &[
            0xd280_0540, // mov x0, #42
            0xd400_0102, // hvc #8
            0x9100_0400, // add x0, x0, #1
            0xd400_0122, // hvc #9
        ],
    );
    let vm = Vm::new(40).unwrap();
    vm.map(&mem, CODE_IPA, memory::READ | memory::WRITE | memory::EXEC)
        .unwrap();
    let vcpu = Vcpu::new(&vm).unwrap();
    vcpu.set_reg(Reg::Pc, CODE_IPA).unwrap();
    vcpu.set_reg(Reg::Cpsr, EL1H_MASKED).unwrap();

    let exit = vcpu.run().unwrap();
    assert_eq!(
        (exit.exception_class(), exit.hvc_immediate()),
        (Some(0x16), Some(8)),
        "{exit:?}"
    );
    assert_eq!(vcpu.reg(Reg::X(0)).unwrap(), 42);
    assert_eq!(
        vcpu.reg(Reg::Pc).unwrap(),
        CODE_IPA + 8,
        "the PC after an hvc exit is the address after the hvc"
    );

    let exit = vcpu.run().unwrap();
    assert_eq!(exit.hvc_immediate(), Some(9), "{exit:?}");
    assert_eq!(vcpu.reg(Reg::X(0)).unwrap(), 43);
    assert_eq!(vcpu.reg(Reg::Pc).unwrap(), CODE_IPA + 16);
}
