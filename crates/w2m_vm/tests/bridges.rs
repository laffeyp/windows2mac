// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 141's judge: in one Rust VM holding `tests/data/timed_waits/qpc.exe`, an import's bridge reaches its guest
//! runtime service and returns through the dispatcher as the Microsoft x64 convention says; an import with no service
//! stops at `hvc #9`.
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use guest_runtime::layout::{
    HEAP_VA, IPA_BASE, MONITOR_DATA, MONITOR_VA, RUN_TABLE, RUNTIME_IMAGE_VA, runtime_defines,
};
use hypervisor::{Exit, Reg, Vcpu, Vm, memory};
use pe_loader::{THUNK_BASE, load};
use w2m_vm::CPSR_EL1H;
use w2m_vm::guest::Guest;
use w2m_vm::monitor::set_system_registers;
use w2m_vm::monitor_code::RUNTIME_LOCK;
use w2m_vm::process::Start;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn system_modules() -> BTreeSet<String> {
    let v: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root().join("specs/windows_exports.json")).unwrap()).unwrap();
    v["modules"].as_object().unwrap().keys().cloned().collect()
}

const NO_SUCH: &str = "KERNEL32.dll!W2mNoSuch";

fn guest() -> (Guest, u64) {
    let root = root();
    let data = fs::read(root.join("tests/data/timed_waits/qpc.exe")).unwrap();
    let mut loaded = load(&data, None, "probe.exe").unwrap();
    let no_such = THUNK_BASE + 16 * loaded.thunks.len() as u64;
    loaded.thunks.insert(no_such, NO_SUCH.to_string());
    let (_, image) = guest_runtime::build(&root, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    let services = guest_runtime::services::services(&guest_runtime::texts(&root).unwrap()).unwrap();
    let table = fs::read_to_string(root.join("tools/vm_compiler/stencil_table.h")).unwrap();
    let symbols = w2m_vm::guest::vm_compiler_symbols(&table).unwrap();
    let g = Guest::new(
        &loaded,
        &image,
        &services,
        &Start::console("probe.exe"),
        &system_modules(),
        &symbols,
    )
    .unwrap();
    (g, no_such)
}

fn thunk(g: &Guest, name: &str, loaded_names: &[(u64, String)]) -> u64 {
    let (t, _) = loaded_names.iter().find(|(_, n)| n == name).unwrap();
    g.bridges[t]
}

fn run_to_hvc(vcpu: &Vcpu<'_>) -> Result<u16, Exit> {
    loop {
        match vcpu.run().unwrap() {
            Exit::VtimerActivated => vcpu.set_vtimer_mask(true).unwrap(),
            e => return e.hvc_immediate().ok_or(e),
        }
    }
}

#[test]
fn a_bridged_call_returns_through_the_dispatcher() {
    let (mut g, no_such) = guest();
    let data = fs::read(root().join("tests/data/timed_waits/qpc.exe")).unwrap();
    let names: Vec<(u64, String)> = load(&data, None, "probe.exe")
        .unwrap()
        .thunks
        .into_iter()
        .chain([(no_such, NO_SUCH.to_string())])
        .collect();
    let stack_top = 0x1_0000_0000u64; // pe_loader::STACK_TOP
    let rsp = stack_top - 0x100;
    let ret = 0x1_4000_1234u64;
    let at = |g: &Guest, va: u64| g.layout.offset(va).unwrap() as usize;
    let rsp_off = at(&g, rsp);
    g.memory.as_mut_slice()[rsp_off..rsp_off + 8].copy_from_slice(&ret.to_le_bytes());
    let std_output = {
        // the parameters block's StandardOutput (RTL_USER_PROCESS_PARAMETERS 0x28; Sprint 140)
        let p = at(&g, HEAP_VA + 0x1000);
        u64::from_le_bytes(g.memory.as_slice()[p + 0x28..p + 0x30].try_into().unwrap())
    };

    let vm = Vm::new(40).unwrap();
    vm.map(&g.memory, IPA_BASE, memory::READ | memory::WRITE | memory::EXEC)
        .unwrap();
    let vcpu = Vcpu::new(&vm).unwrap();
    set_system_registers(&vcpu, g.ttbr).unwrap();
    let x = |r: u8| vcpu.reg(Reg::X(r)).unwrap();
    let start = |va: u64, regs: &[(u8, u64)]| {
        for r in 0..16u8 {
            vcpu.set_reg(Reg::X(r), 0x1000 + u64::from(r)).unwrap();
        }
        vcpu.set_reg(Reg::X(4), rsp).unwrap();
        for (r, v) in regs {
            vcpu.set_reg(Reg::X(*r), *v).unwrap();
        }
        vcpu.set_reg(Reg::X(23), 1_000_000).unwrap(); // the budget
        vcpu.set_reg(Reg::X(28), MONITOR_VA + MONITOR_DATA).unwrap();
        vcpu.set_reg(Reg::Cpsr, CPSR_EL1H).unwrap();
        vcpu.set_reg(Reg::Pc, va).unwrap();
    };
    let lock = |g: &Guest| {
        let o = (RUN_TABLE + RUNTIME_LOCK) as usize;
        u32::from_le_bytes(g.memory.as_slice()[o..o + 4].try_into().unwrap())
    };

    // GetStdHandle(STD_OUTPUT_HANDLE): RAX the block's StandardOutput (parent.standard_handles, output_is_std)
    start(thunk(&g, "KERNEL32.dll!GetStdHandle", &names), &[(1, (-11i64) as u64)]);
    assert_eq!(
        run_to_hvc(&vcpu),
        Ok(1),
        "GetStdHandle's bridge ends at the dispatcher's miss"
    );
    assert_eq!(
        (x(17), x(4)),
        (ret, rsp + 8),
        "RET's pop: x17 the return address, RSP raised by 8"
    );
    assert_eq!(x(0), std_output, "RAX is the parameters block's StandardOutput");
    assert_eq!(std_output, 0x74);
    assert_eq!(
        [x(3), x(5), x(6), x(7), x(10), x(15)],
        [0x1003, 0x1005, 0x1006, 0x1007, 0x100A, 0x100F],
        "the other x86 registers kept"
    );
    assert_eq!(
        (x(23), lock(&g)),
        (1_000_000, 0),
        "the budget kept and the runtime lock free"
    );

    // QueryPerformanceCounter(&c): a count at [RCX], RAX 1
    let slot = rsp + 0x40;
    start(thunk(&g, "KERNEL32.dll!QueryPerformanceCounter", &names), &[(1, slot)]);
    assert_eq!(run_to_hvc(&vcpu), Ok(1));
    let o = at(&g, slot);
    let count = u64::from_le_bytes(g.memory.as_slice()[o..o + 8].try_into().unwrap());
    assert_eq!((x(0), x(17)), (1, ret));
    assert_ne!(count, 0, "a count at [RCX]");

    // an import with no service
    start(g.bridges[&no_such], &[]);
    assert_eq!(run_to_hvc(&vcpu), Ok(9), "{NO_SUCH} stops at hvc #9");
    println!("bridges: GetStdHandle answered 0x{std_output:x}, QueryPerformanceCounter {count}, {NO_SUCH} hvc #9");
}
