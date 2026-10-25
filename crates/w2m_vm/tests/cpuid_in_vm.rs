// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 134's judge, carried from `tests/test_vm_state.py` 193-240: the guest runtime's CPUID answer, from a Rust VM
//! with the MMU on, equals `cpuid.lookup`'s for every leaf of `specs/guest_cpuid.json` and six inputs past its edges.
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use std::fs;
use std::path::{Path, PathBuf};

use guest_runtime::layout::{MONITOR_VA, RUNTIME_IMAGE_VA, runtime_defines};
use hypervisor::{Vcpu, Vm};
use serde_json::Value;
use w2m_vm::NATIVE_OPERANDS;
use w2m_vm::monitor::{MonitorMemory, call, map, set_system_registers};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

type Row = ((u64, Option<u64>), [u64; 4]);

fn spec(root: &Path) -> Vec<Row> {
    let v: Value = serde_json::from_str(&fs::read_to_string(root.join("specs/guest_cpuid.json")).unwrap()).unwrap();
    v["leaves"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            let r = |k: &str| e[k].as_u64().unwrap();
            (
                (r("leaf"), e["subleaf"].as_u64()),
                [r("eax"), r("ebx"), r("ecx"), r("edx")],
            )
        })
        .collect()
}

/// `src/w2m/cpuid.py` `lookup` (30-41), the oracle the Python test compared the VM with.
fn lookup(table: &[Row], eax: u64, ecx: u64) -> [u64; 4] {
    let get = |k: (u64, Option<u64>)| table.iter().find(|(key, _)| *key == k).map(|(_, r)| *r);
    let max_basic = get((0, None)).unwrap()[0];
    let max_ext = get((0x8000_0000, None)).unwrap()[0];
    let mut leaf = eax & 0xFFFF_FFFF;
    if leaf > max_ext || (max_basic < leaf && leaf < 0x8000_0000) {
        leaf = max_basic;
    }
    if let Some(r) = get((leaf, Some(ecx & 0xFFFF_FFFF))) {
        return r;
    }
    if let Some(r) = get((leaf, None)) {
        return r;
    }
    if table.iter().any(|((l, _), _)| *l == leaf) {
        return [0; 4];
    }
    get((max_basic, None)).unwrap_or([0; 4])
}

#[test]
fn the_runtimes_cpuid_answers_equal_the_oracles_from_a_rust_vm() {
    let root = root();
    let (_, image) = guest_runtime::build(&root, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    let query = image.symbols["w2m_cpuid_query"];
    let area = MonitorMemory::new(&image, &[]).unwrap();
    let table = spec(&root);
    // tests/test_vm_state.py 204-205
    let mut inputs: Vec<(u64, u64)> = table.iter().map(|((l, s), _)| (*l, s.unwrap_or(0))).collect();
    inputs.extend([
        (1, 5),
        (7, 1),
        (8, 0),
        (0x4000_0000, 0),
        (0x8000_0009, 0),
        (0xFFFF_FFFF, 0),
    ]);

    let vm = Vm::new(40).unwrap();
    map(&vm, &area).unwrap();
    let vcpu = Vcpu::new(&vm).unwrap();
    set_system_registers(&vcpu, area.ttbr).unwrap();
    for (eax, ecx) in &inputs {
        // the Python test set EAX's upper half to 0xDEAD...: the runtime takes the low 32 bits
        call(
            &vcpu,
            query,
            &[eax | 0xDEAD_0000_0000_0000, *ecx, MONITOR_VA + NATIVE_OPERANDS],
        )
        .unwrap_or_else(|e| panic!("CPUID({eax:#x}, {ecx:#x}): {e:?}"));
        let got: [u64; 4] = std::array::from_fn(|i| u64::from(area.read_u32(NATIVE_OPERANDS + 4 * i as u64)));
        assert_eq!(got, lookup(&table, *eax, *ecx), "CPUID({eax:#x}, {ecx:#x})");
    }
    println!("cpuid_in_vm: {} inputs answered as the oracle answers", inputs.len());
}
