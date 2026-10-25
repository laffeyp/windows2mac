// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 134's planted fault: with the runtime image's blocks left out of the tables, a call into it takes the
//! instruction abort vector, whose `hvc` reaches the host with the abort's syndrome, in place of a hang or a wrong
//! answer.
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use std::path::Path;

use guest_runtime::layout::{MONITOR_VA, RUNTIME_IMAGE, RUNTIME_IMAGE_SIZE, RUNTIME_IMAGE_VA, runtime_defines};
use hypervisor::{Vcpu, Vm};
use w2m_vm::monitor::{MonitorError, MonitorMemory, call, map, set_system_registers};
use w2m_vm::{NATIVE_OPERANDS, VECTOR_HVC_BASE};

#[test]
fn a_call_into_an_unmapped_image_takes_the_abort_vector() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let (_, image) = guest_runtime::build(&root, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    let query = image.symbols["w2m_cpuid_query"];
    let image_range = (
        MONITOR_VA + RUNTIME_IMAGE,
        MONITOR_VA + RUNTIME_IMAGE + RUNTIME_IMAGE_SIZE,
    );
    let area = MonitorMemory::new(&image, &[image_range]).unwrap();
    let vm = Vm::new(40).unwrap();
    map(&vm, &area).unwrap();
    let vcpu = Vcpu::new(&vm).unwrap();
    set_system_registers(&vcpu, area.ttbr).unwrap();
    match call(&vcpu, query, &[0, 0, MONITOR_VA + NATIVE_OPERANDS]) {
        Err(MonitorError::Stopped { exit, esr, elr, .. }) => {
            // the synchronous exception from the current EL with SP_EL1: the vectors' fifth slot (VBAR + 0x200)
            assert_eq!(exit.hvc_immediate(), Some(VECTOR_HVC_BASE + 4), "{exit:?}");
            assert_eq!(
                esr >> 26,
                0x21,
                "an instruction abort from the current EL (ESR {esr:#x})"
            );
            assert_eq!(elr, query, "the abort's address is the function's");
        }
        other => panic!("expected the abort vector, got {other:?}"),
    }
}
