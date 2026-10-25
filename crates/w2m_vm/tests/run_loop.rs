// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 155's judge: `tests/data/timed_waits/qpc.exe` runs in a Rust VM with the host compiling, in Rust, the blocks
//! the VM's C compiler declines (`w2m_vm::compile`), past its first decline to a named stop; the blocks the host
//! compiled, in order, are the frozen Python host's compiles of `qpc` (`build/run_loop/qpc-python.json`, made by
//! running it, its sha256 in `store/run_loop/qpc-python.sha256`; its generator is named in WORKING_AGREEMENT).
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use cpu_cases::sha256;
use guest_runtime::layout::{IPA_BASE, RUNTIME_IMAGE_VA, runtime_defines};
use hypervisor::{Vcpu, Vm, memory};
use pe_loader::load;
use w2m_vm::compile::HostCompiler;
use w2m_vm::guest::{Guest, vm_compiler_symbols};
use w2m_vm::process::Start;
use w2m_vm::run::{Stop, run_compiling};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// A missing input: a failure under `W2M_REQUIRE_CASE_RUNS=1` (the gates), else a skip said on stderr.
fn missing(what: &str) {
    if std::env::var("W2M_REQUIRE_CASE_RUNS").as_deref() == Ok("1") {
        panic!("{what}, and W2M_REQUIRE_CASE_RUNS=1 requires it");
    }
    eprintln!("{what}: the run loop's judge is not run here");
}

#[test]
fn qpc_runs_past_its_first_decline_compiling_as_the_python_host() {
    let root = root();
    let Ok(text) = fs::read_to_string(root.join("build/run_loop/qpc-python.json")) else {
        return missing("no Python host compiles of qpc (build/run_loop/qpc-python.json)");
    };
    let pinned = fs::read_to_string(root.join("store/run_loop/qpc-python.sha256")).unwrap();
    assert_eq!(
        sha256(text.as_bytes()),
        pinned.trim(),
        "the fixture is not the one committed by its sha256"
    );
    let python: serde_json::Value = serde_json::from_str(&text).unwrap();
    let python_compiles: Vec<(u64, u64)> = python["translations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| (t[0].as_u64().unwrap(), t[1].as_u64().unwrap()))
        .collect();

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
    let mut host = HostCompiler::default();
    let before = monotonic_count();
    let out = run_compiling(&mut guest, &loaded, &vcpu, 1 << 40, &mut host).unwrap();
    let after = monotonic_count();

    println!("stop: {:?}", out.stop);
    println!("blocks the C compiled: {:x?}", out.blocks.keys().collect::<Vec<_>>());
    println!("blocks the host compiled: {:x?}", host.translations);
    println!("their code: {:x?}", host.blocks);
    println!("the Python host's compiles: {:x?}", python_compiles);
    println!("bridged: {:x?}", out.bridged);
    println!("exits: {:?}", out.exits);
    println!("stdout: {:?}", String::from_utf8_lossy(&guest.stdout));

    // `tests/test_timed_waits.py` 218-223, carried: the process ends, and its counter lies between the host's monotonic
    // count before the run and after it
    assert_eq!(out.stop, Stop::ExitProcess { code: 0 });
    let text = String::from_utf8(guest.stdout.clone()).unwrap();
    let counter = u64::from_str_radix(text.trim_end(), 16).unwrap();
    assert!(
        before <= counter && counter <= after,
        "{before} <= {counter} <= {after}"
    );
    // the blocks: every start the C or the host compiled is a Python host compile's, each host compile's end is the
    // Python's, and together they are every block the Python host compiled
    let python_starts: BTreeSet<u64> = python_compiles.iter().map(|(s, _)| *s).collect();
    let ours: BTreeSet<u64> = out
        .blocks
        .keys()
        .copied()
        .chain(host.translations.iter().map(|(s, _)| *s))
        .collect();
    for compile in &host.translations {
        assert!(
            python_compiles.contains(compile),
            "the host compiled {compile:x?}, which the Python did not"
        );
    }
    assert_eq!(ours, python_starts, "the blocks compiled differ from the Python host's");
    assert!(!host.translations.is_empty(), "the host compiled nothing");
    // and each host compile's words, at the same code address, are the Python host's words for that block
    for (start, va) in &host.blocks {
        let want = &python["blocks"][format!("0x{start:x}")];
        assert_eq!(
            want[0].as_str().unwrap(),
            format!("0x{va:x}"),
            "block 0x{start:x}'s code address"
        );
        let hex = want[1].as_str().unwrap();
        let at = guest.layout.offset(*va).unwrap() as usize;
        let ours: String = guest.memory.as_slice()[at..at + hex.len() / 2]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(ours, hex, "block 0x{start:x}'s words differ from the Python host's");
    }
}

/// `kernel32.monotonic_count`: QueryPerformanceCounter's count, the host's monotonic clock in 100 ns units. CPython's
/// `time.monotonic_ns` on macOS is `mach_absolute_time`, which `CLOCK_UPTIME_RAW` reads.
fn monotonic_count() -> u64 {
    unsafe extern "C" {
        fn clock_gettime_nsec_np(clock: u32) -> u64;
    }
    const CLOCK_UPTIME_RAW: u32 = 8;
    unsafe { clock_gettime_nsec_np(CLOCK_UPTIME_RAW) / 100 }
}
