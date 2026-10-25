// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 142's judge: `tests/data/timed_waits/qpc.exe` runs from its entry in a Rust VM, its code compiled by the C
//! compiler in the VM and its imports answered by the bridges, until the first miss the host's compiler must serve;
//! the run names that block, and the C's own counts say why it declined.
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use guest_runtime::layout::{IPA_BASE, RUNTIME_IMAGE_VA, runtime_defines};
use hypervisor::{Vcpu, Vm, memory};
use pe_loader::load;
use w2m_vm::guest::{Guest, vm_compiler_symbols};
use w2m_vm::process::Start;
use w2m_vm::run::{Stop, run_to_first_decline};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

#[test]
fn qpc_runs_to_its_first_declined_block() {
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
    let mut guest = Guest::new(
        &loaded,
        &image,
        &services,
        &Start::console("probe.exe"),
        &modules,
        &symbols,
    )
    .unwrap();

    let vm = Vm::new(40).unwrap();
    vm.map(&guest.memory, IPA_BASE, memory::READ | memory::WRITE | memory::EXEC)
        .unwrap();
    let vcpu = Vcpu::new(&vm).unwrap();
    let out = run_to_first_decline(&mut guest, &loaded, &vcpu, 1 << 40).unwrap();

    let text = loaded.image.sections.iter().find(|s| s.name == ".text").unwrap();
    let base = loaded.image.image_base;
    let (lo, hi) = (
        base + u64::from(text.virtual_address),
        base + u64::from(text.virtual_address + text.virtual_size),
    );
    let c = |name: &str| out.counts[name];
    println!("stop: {:?}", out.stop);
    println!("blocks the C compiled: {:x?}", out.blocks);
    println!("bridged: {:x?}", out.bridged);
    println!("exits: {:?}", out.exits);
    println!(
        "counts: {:?}",
        out.counts.iter().filter(|(_, v)| **v != 0).collect::<Vec<_>>()
    );
    let median = |v: &mut Vec<u64>| {
        v.sort_unstable();
        v.get(v.len() / 2).copied().unwrap_or(0)
    };
    let (mut serve, mut run) = (out.serve_ns.clone(), out.run_ns.clone());
    println!(
        "{} served exits: median {} ns from exit to the next run; {} runs: median {} ns",
        serve.len(),
        median(&mut serve),
        run.len(),
        median(&mut run)
    );

    let Stop::Declined { target } = out.stop else {
        panic!("the run stopped otherwise: {:?}", out.stop)
    };
    assert!(
        (lo..hi).contains(&target),
        "the declined block 0x{target:x} is in qpc's .text"
    );
    assert!(!out.blocks.contains_key(&target) && !guest.bridges.contains_key(&target));
    assert!(c("compiled") >= 1, "the C compiled at least one block");
    assert!(
        c("no stencil") + c("cut by no stencil") + c("page needs a split") > 0,
        "the C's counts say why it declined"
    );
    assert_eq!(
        c("not a code page"),
        out.bridged.values().sum::<u64>(),
        "every 'not a code page' is a thunk a bridge answered"
    );
    assert!(
        out.exits.keys().all(|k| *k == 1),
        "every exit before the stop is hvc #1"
    );
}
