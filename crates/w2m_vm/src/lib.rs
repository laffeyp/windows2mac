// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The VM a `w2m-vm` process owns (Sprint 134, the walking skeleton's third link): the monitor area mapped at
//! `IPA_BASE`, stage-1 tables that put it at `MONITOR_VA`, the C guest runtime copied to where it was linked, the
//! system registers the Python host set, and vectors that hand every exception to the host. Carried from
//! `src/w2m/vm_backend.py`; each value cites its line.

/// The assembler moved to `a64_encoder` (Sprint 151), where the translator reaches it too.
pub use a64_encoder::assembler;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub mod case_inputs;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub mod cases;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub mod compile;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub mod guest;
pub mod guest_stubs;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub mod host_request;
pub mod ledger;
pub mod monitor_code;
pub mod process;
pub mod record;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub mod run;
pub mod tables;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub mod threads;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub mod monitor;

/// vm_backend 334-335: the VirtualAlloc arena, sparse (here, ungated, so the arena's replay runs on CI; Sprint 149).
pub const VIRTUAL_VA: u64 = 0x0000_0010_0000_0000;
pub const VIRTUAL_SIZE: u64 = 0x80_0000_0000;
/// A level-1 block in the 4 KB granule, 1 GB (vm_backend 84).
pub const GIGA_BLOCK: u64 = 0x4000_0000;
/// The exception vectors, 16 slots of 0x80 (vm_backend 127).
pub const VECTORS: u64 = 0x01000;
/// The dispatcher's place (vm_backend 128).
pub const DISPATCHER: u64 = 0x01800;
/// The skeleton's return stub: `hvc #1` in the free area just past the exception entry (vm_backend 130), a Rust
/// test aid the Python host did not have.
pub const RETURN_STUB: u64 = 0x02000;
/// The native helper operands (vm_backend 126): 32 bytes the skeleton's calls write their results to.
pub const NATIVE_OPERANDS: u64 = 0x300;
/// AttrIndx 0 (MAIR byte 0 = 0xFF, Normal write-back), inner shareable, the access flag (vm_backend 352).
pub const PAGE_ATTRIBUTES: u64 = (3 << 8) | (1 << 10);
/// FP/SIMD at EL1 and EL0 not trapped (vm_backend 351).
pub const CPACR_FPEN: u64 = 3 << 20;
/// ACTLR_EL1.EnTSO: x86's store order in the guest (vm_backend 350).
pub const ACTLR_ENTSO: u64 = 1 << 1;
/// M, C, I; alignment checks off (vm_backend 353).
pub const SCTLR: u64 = 0x30D0_1805;
/// T0SZ 16, write-back, inner shareable, EPD1, IPS 40 (vm_backend 354).
pub const TCR: u64 = 16 | (1 << 8) | (1 << 10) | (3 << 12) | (1 << 23) | (2 << 32);
/// MAIR_EL1 (vm_backend 4980): attribute 0 Normal, inner and outer write-back.
pub const MAIR: u64 = 0xFF;
/// EL1h with D, A, I and F masked (vm_backend 355).
pub const CPSR_EL1H: u64 = 0x3C5;

/// `hvc #imm` (Arm ARM C6.2: 0xD4000002 with the immediate at bits 20:5).
pub fn hvc(imm: u16) -> u32 {
    0xD400_0002 | (u32::from(imm) << 5)
}

/// The immediate each exception vector's `hvc` carries: 0x100 + the slot (0 to 15), so an exit names the vector.
pub const VECTOR_HVC_BASE: u16 = 0x100;
/// The immediate of the skeleton's return stub.
pub const RETURN_HVC: u16 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hvc_encodes_as_clang_assembles_it() {
        // clang --target=aarch64-linux-gnu, 2026-10-25: hvc #8 = d4000102, hvc #0 = d4000002
        assert_eq!(hvc(8), 0xd400_0102);
        assert_eq!(hvc(0), 0xd400_0002);
    }

    #[test]
    fn the_tcr_is_the_python_hosts() {
        assert_eq!(TCR, 0x2_0080_3510);
    }
}
