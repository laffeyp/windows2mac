// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The crate's integration tests as one binary (Sprint 300), as `w2m_vm`'s `tests/all.rs` says why: one link and one
//! assessment by macOS's `syspolicyd` a rebuild, not one a file. Each file stays where it was, a module here
//! (`autotests = false` in the manifest). Hypervisor.framework allows one VM a process, so on macOS these run under
//! nextest, a process a test, as the gates run them; `cargo test` would run them as threads of one process (Sprint
//! 303, a consequence of the merge Sprint 300 did not name).

mod exit_time;
mod first_exit;
mod id_registers;
mod no_wall_clock_bound;
mod stop_from_another_thread;
mod vector_register;

/// A file under `tests/` that no `mod` here declares compiles to nothing, and its tests vanish without a failure.
#[test]
fn every_file_is_a_module() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let root = std::fs::read_to_string(dir.join("all.rs")).unwrap();
    let mut undeclared = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().to_string();
        let Some(stem) = name.strip_suffix(".rs") else { continue };
        if stem != "all" && !root.lines().any(|l| l == format!("mod {stem};")) {
            undeclared.push(name);
        }
    }
    assert!(
        undeclared.is_empty(),
        "files under tests/ no module declares: {undeclared:?}"
    );
}
