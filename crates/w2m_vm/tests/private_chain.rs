// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Each vCPU's own pages at the shared addresses (Sprint 315; the per-vCPU pages design, form B, and its card 2; the
//! frozen host's Sprint 104 judge): two vCPUs of one VM, vCPU 1 on its private chain, each read the monitor data page's
//! first word at one address and see their own; each `TTBR0_EL1` has CnP 0; a page the guest maps after vCPU 1's chain is
//! made reads the same on both; and a split asked in the process area's first gigabyte, whose tables the chain copies,
//! is refused by name.
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use a64_encoder as a64;
use guest_runtime::layout::{IPA_BASE, MONITOR_DATA, MONITOR_VA, PAGE, PROCESS_VA, RUNTIME_IMAGE_VA, runtime_defines};
use hypervisor::{Exit, Reg, Vcpu, Vm, memory, sys_reg};
use pe_loader::load;
use w2m_vm::guest::{Guest, vm_compiler_symbols};
use w2m_vm::monitor::set_system_registers;
use w2m_vm::process::Start;
use w2m_vm::{CPSR_EL1H, RETURN_STUB, VIRTUAL_VA, VcpuPages};

/// `ldr x0, [x1]`, then `hvc #0x23`, in the monitor area's free words past the exception entry.
const LOAD: u64 = RETURN_STUB + 0x280;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The qpc guest, built as the run-loop tests build theirs, with the load stub written.
fn guest() -> Guest {
    let root = root();
    let data = fs::read(root.join("tests/data/timed_waits/qpc.exe")).unwrap();
    let loaded = load(&data, None, "probe.exe").unwrap();
    let (_, image) = guest_runtime::build(&root, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    let services = guest_runtime::services::services(&guest_runtime::texts(&root).unwrap()).unwrap();
    let exports: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("specs/windows_exports.json")).unwrap()).unwrap();
    let modules: BTreeSet<String> = exports["modules"].as_object().unwrap().keys().cloned().collect();
    let symbols =
        vm_compiler_symbols(&fs::read_to_string(root.join("tools/vm_compiler/stencil_table.h")).unwrap()).unwrap();
    let guest = Guest::new(&loaded, &image, &services, &Start::console("probe.exe"), &modules, &symbols).unwrap();
    let words: Vec<u8> = [a64::ldr(8, 0, 1, 0), a64::hvc(0x23)]
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .collect();
    guest.memory_view().write(LOAD, &words);
    guest
}

/// A vCPU made on this thread with `ttbr`, which loads the word at `va`: the word, and `TTBR0_EL1` as it reads.
fn load_on_a_vcpu(vm: &Vm, ttbr: u64, va: u64) -> (u64, u64) {
    let vcpu = Vcpu::new(vm).unwrap();
    set_system_registers(&vcpu, ttbr).unwrap();
    vcpu.set_reg(Reg::X(1), va).unwrap();
    vcpu.set_reg(Reg::Cpsr, CPSR_EL1H).unwrap();
    vcpu.set_reg(Reg::Pc, MONITOR_VA + LOAD).unwrap();
    loop {
        match vcpu.run().unwrap() {
            Exit::VtimerActivated => vcpu.set_vtimer_mask(true).unwrap(),
            e => {
                assert_eq!(e.hvc_immediate(), Some(0x23), "the load stopped at {e:?}");
                break;
            }
        }
    }
    (vcpu.reg(Reg::X(0)).unwrap(), vcpu.sys_reg(sys_reg::TTBR0_EL1).unwrap())
}

#[test]
fn each_vcpu_reads_its_own_monitor_data_at_one_address() {
    let _one = crate::one_vm();
    let mut g = guest();
    let (ttbr1, pages) = g.private_chain(1).unwrap();
    assert_ne!(pages, VcpuPages::FIRST);
    let mem = g.memory_view();
    mem.write_u64(MONITOR_DATA, 0xA0A0);
    mem.write_u64(pages.monitor, 0xB1B1);
    // a page the guest maps after vCPU 1's chain is made, in the arena, which every chain shares
    let late = VIRTUAL_VA + 0x40_0000;
    g.map_pages(late, late + PAGE, true, &mut || Ok(())).unwrap();
    let at = g.layout.offset(late).expect("the arena's page in the layout");
    g.memory_view().write_u64(at, 0x1A7E);
    let refused = g
        .page_descriptor(PROCESS_VA + 0x20_0000, &mut || Ok(()))
        .expect_err("a split in the process area's first gigabyte");
    assert!(refused.contains("process area's first gigabyte"), "{refused}");

    let vm = Vm::new(40).unwrap();
    vm.map(&g.memory, IPA_BASE, memory::READ | memory::WRITE | memory::EXEC)
        .unwrap();
    let ttbr0 = g.ttbr;
    let monitor = MONITOR_VA + MONITOR_DATA;
    let (on0, on1) = std::thread::scope(|s| {
        let a = s.spawn(|| (load_on_a_vcpu(&vm, ttbr0, monitor), load_on_a_vcpu(&vm, ttbr0, late)));
        let b = s.spawn(|| (load_on_a_vcpu(&vm, ttbr1, monitor), load_on_a_vcpu(&vm, ttbr1, late)));
        (a.join().unwrap(), b.join().unwrap())
    });
    assert_eq!((on0.0.0, on1.0.0), (0xA0A0, 0xB1B1), "each vCPU's monitor data at one address");
    assert_eq!((on0.1.0, on1.1.0), (0x1A7E, 0x1A7E), "a page mapped after the chain, on both");
    assert_eq!((on0.0.1 & 1, on1.0.1 & 1), (0, 0), "TTBR0_EL1.CnP");
}
