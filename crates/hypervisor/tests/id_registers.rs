// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The CPU's level of FEAT_BBM as a vCPU reads it (Sprint 303): `ID_AA64MMFR2_EL1.BBM`, bits 55:52, 0b0010 for level 2,
//! under which a translation's change of block size alone needs no break-before-make, the old and new entries both
//! allowed in a TLB with no conflict abort, a TLB invalidation following (the Arm ARM, DDI 0487, FEAT_BBM). Whether a
//! block split in the guest's stage-1 tables needs the design's two steps rests on it
//! (`docs/design-run-loop-on-several-vcpus-2026-10-31.md`, precondition 4).
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use hypervisor::{Vcpu, Vm, sys_reg};

#[test]
fn the_vcpu_reports_its_bbm_level() {
    let vm = Vm::new(40).unwrap();
    let vcpu = Vcpu::new(&vm).unwrap();
    let mmfr2 = vcpu.sys_reg(sys_reg::ID_AA64MMFR2_EL1).unwrap();
    let bbm = (mmfr2 >> 52) & 0xF;
    println!("id_aa64mmfr2_el1: {mmfr2:#018x}, BBM {bbm}");
    assert!(bbm <= 2, "BBM {bbm}: a value the Arm ARM does not define");
    // the design's precondition 4 rests on this level: a machine that reports another fails here, naming it
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let design = std::fs::read_to_string(root.join("docs/design-run-loop-on-several-vcpus-2026-10-31.md")).unwrap();
    assert!(
        design.contains(&format!("BBM {bbm},")),
        "this vCPU reports BBM {bbm}, which the design's precondition 4 does not state"
    );
}
