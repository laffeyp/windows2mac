// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 157's expected value for `Vcpu::set_simd`: a vector the host sets is the one the guest reads. The guest copies
//! Q1 to Q0, and the host reads Q0 back. One VM per process, so this is its own test file.
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use hypervisor::{GuestMemory, Reg, Vcpu, Vm, memory, sys_reg};

const CODE_IPA: u64 = 0x1_0000;
/// EL1h with D, A, I and F masked.
const EL1H_MASKED: u64 = 0x3c5;
/// CPACR_EL1.FPEN: SIMD and floating point at EL1 do not trap.
const CPACR_FPEN: u64 = 3 << 20;

#[test]
fn a_vector_the_host_sets_is_the_one_the_guest_reads() {
    // assembled by clang (--target=aarch64-linux-gnu) and read back with llvm-objdump, 2026-10-25
    let mut mem = GuestMemory::new(64 * 1024).unwrap();
    mem.write_words(
        0,
        &[
            0x4ea1_1c20, // mov v0.16b, v1.16b
            0xd400_0102, // hvc #8
        ],
    );
    let vm = Vm::new(40).unwrap();
    vm.map(&mem, CODE_IPA, memory::READ | memory::WRITE | memory::EXEC)
        .unwrap();
    let vcpu = Vcpu::new(&vm).unwrap();
    vcpu.set_reg(Reg::Pc, CODE_IPA).unwrap();
    vcpu.set_reg(Reg::Cpsr, EL1H_MASKED).unwrap();
    vcpu.set_sys_reg(sys_reg::CPACR_EL1, CPACR_FPEN).unwrap();
    let value = 0x0011_2233_4455_6677_8899_aabb_ccdd_eeffu128;
    vcpu.set_simd(1, value).unwrap();
    assert_eq!(vcpu.simd(1).unwrap(), value, "Q1 read back before the run");

    let exit = vcpu.run().unwrap();
    assert_eq!(exit.hvc_immediate(), Some(8), "{exit:?}");
    assert_eq!(vcpu.simd(0).unwrap(), value, "Q0, the guest's copy of Q1");
}
