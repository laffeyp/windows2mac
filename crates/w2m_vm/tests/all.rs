// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The crate's integration tests as one binary (Sprint 300). Each file under `tests/` was its own binary, and macOS's
//! `syspolicyd` assesses each newly linked executable on its first run, nextest running each twice to list it: at
//! Sprint 298's gates three listings waited 1 min 28 s each. Each file stays where it was, a module here
//! (`autotests = false` in the manifest); nextest still runs each test in its own process.

// the VM tests' shared setup, macOS-only as its two users are
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
mod common;

/// One VM at a time in this binary (Sprint 306): Hypervisor.framework allows one VM a process, and `cargo test` runs a
/// binary's tests as threads of one; every test that makes a VM holds this for its life. nextest, a process a test,
/// takes it uncontended. A test that panicked holding it leaves it poisoned, which the next taker takes regardless.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
static ONE_VM: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn one_vm() -> std::sync::MutexGuard<'static, ()> {
    ONE_VM.lock().unwrap_or_else(|e| e.into_inner())
}

mod batch_record;
mod batch_run;
mod block_judge;
mod bridges;
mod cache_file;
mod cache_layout;
mod case_runner;
mod class_judge;
mod cpuid_in_vm;
mod dispatch_hash_planted;
mod exit_ledger;
mod family_runs;
mod first_decline;
mod flush_stub;
mod guest_parts;
mod guest_stubs;
mod host_requests;
mod layout_constants;
mod ledger;
mod litmus;
mod loader_lists;
mod monitor_code_in_vm;
mod process_area;
mod run_loop;
mod skeleton_gate;
mod slice_calls;
mod unmapped_image_faults;
// the `virtual_memory` crate's name, which `use virtual_memory::…` in the file means
#[path = "virtual_memory.rs"]
mod virtual_memory_arena;

/// A file under `tests/` that no `mod` here declares compiles to nothing, and its tests vanish without a failure.
#[test]
fn every_file_is_a_module() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let root = std::fs::read_to_string(dir.join("all.rs")).unwrap();
    let mut undeclared = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().to_string();
        let Some(stem) = name.strip_suffix(".rs") else { continue };
        if stem != "all"
            && !root.lines().any(|l| l == format!("mod {stem};"))
            && !root.contains(&format!("#[path = \"{name}\"]"))
        {
            undeclared.push(name);
        }
    }
    assert!(
        undeclared.is_empty(),
        "files under tests/ no module declares: {undeclared:?}"
    );
}
