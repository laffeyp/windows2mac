// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! What Sprint 136's VM tests share: the monitor area with the monitor code written, small code stubs in the free
//! area past the exception entry, and a run to the next `hvc`.

use std::path::{Path, PathBuf};

use a64_encoder as a64;
use guest_runtime::layout::{IPA_BASE, MONITOR_DATA, MONITOR_VA, PAGE, RUNTIME_IMAGE_VA, runtime_defines};
use hypervisor::{Exit, Reg, Vcpu};
use w2m_vm::monitor::MonitorMemory;
use w2m_vm::monitor_code::MonitorCode;
use w2m_vm::tables::{AP_READ_ONLY, page_descriptor};
use w2m_vm::{CPSR_EL1H, RETURN_STUB};

/// Two blocks' code: `hvc #0x20` and `hvc #0x21`.
pub const CODE_A: u64 = RETURN_STUB + 0x100;
pub const CODE_B: u64 = RETURN_STUB + 0x180;
pub const HVC_A: u16 = 0x20;
pub const HVC_B: u16 = 0x21;
/// `str x0, [x1]`, then `hvc #0x22`.
pub const STORE: u64 = RETURN_STUB + 0x200;
/// `ldr x0, [x1]`, then `hvc #0x23`.
pub const LOAD: u64 = RETURN_STUB + 0x280;
/// A read-only page: a 4 KB map onto the monitor area's free page at 0x3000.
pub const READ_ONLY_VA: u64 = 0x10_0000_0000;
const READ_ONLY_IPA: u64 = IPA_BASE + 0x3000;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn put(area: &mut MonitorMemory, off: u64, words: &[u32]) {
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    area.memory.as_mut_slice()[off as usize..off as usize + bytes.len()].copy_from_slice(&bytes);
}

/// The monitor area with the monitor code, the stubs and the read-only page.
pub fn area() -> (MonitorMemory, MonitorCode) {
    let (_, image) = guest_runtime::build(&root(), &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    let (mut area, code) = MonitorMemory::with_monitor_code(&image, &[(READ_ONLY_VA, READ_ONLY_IPA, PAGE)]).unwrap();
    put(&mut area, CODE_A, &[a64::hvc(HVC_A.into())]);
    put(&mut area, CODE_B, &[a64::hvc(HVC_B.into())]);
    put(&mut area, STORE, &[a64::str(8, 0, 1, 0), a64::hvc(0x22)]);
    put(&mut area, LOAD, &[a64::ldr(8, 0, 1, 0), a64::hvc(0x23)]);
    let at = page_descriptor(area.memory.as_slice(), READ_ONLY_VA).expect("the read-only page has a level-3 descriptor")
        as usize;
    let desc = u64::from_le_bytes(area.memory.as_slice()[at..at + 8].try_into().unwrap()) | AP_READ_ONLY;
    area.memory.as_mut_slice()[at..at + 8].copy_from_slice(&desc.to_le_bytes());
    (area, code)
}

/// Starts the vCPU at `pc` at EL1h, x28 the monitor data, with `regs` set.
pub fn start(vcpu: &Vcpu<'_>, pc: u64, regs: &[(u8, u64)]) {
    vcpu.set_reg(Reg::X(28), MONITOR_VA + MONITOR_DATA).unwrap();
    for (r, v) in regs {
        vcpu.set_reg(Reg::X(*r), *v).unwrap();
    }
    vcpu.set_reg(Reg::Cpsr, CPSR_EL1H).unwrap();
    vcpu.set_reg(Reg::Pc, pc).unwrap();
}

/// Runs to the next exit other than the VTimer's; the `hvc`'s immediate, or the exit when it is not an `hvc`.
pub fn run_to_hvc(vcpu: &Vcpu<'_>) -> Result<u16, Exit> {
    loop {
        match vcpu.run().unwrap() {
            Exit::VtimerActivated => vcpu.set_vtimer_mask(true).unwrap(),
            e => return e.hvc_immediate().ok_or(e),
        }
    }
}
