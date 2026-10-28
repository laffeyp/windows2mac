// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! `w2m-vm`, the Rust host as a program (Sprint 158; step 3 of `docs/rust-port-plan-2026-10-24.md`): it loads a guest,
//! runs it on one vCPU with the host compiling the blocks the VM's C declines (`w2m_vm::run::run_compiling`), writes the
//! guest's standard output to a file, and writes the run's record, `record_translations`' events
//! (`w2m_vm::record`), to its own standard output in substrate's line protocol.
//!
//! Substrate runs it as a Producer (`harness/w2m-vm-sleeper.toml`). Usage, paths relative to the repository root:
//! `w2m-vm <guest.exe> --output <file>`. A run that does not end at ExitProcess fails the Producer, naming its stop;
//! its record then holds no SESSION_COMPLETED.

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
fn main() {
    eprintln!("w2m-vm needs Hypervisor.framework on Apple silicon");
    std::process::exit(2);
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn main() -> std::process::ExitCode {
    match run::main() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("w2m-vm: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
mod run {
    use std::collections::BTreeSet;
    use std::fs;
    use std::io::{self, BufRead, Write};
    use std::path::{Path, PathBuf};
    use std::time::Instant;

    use cpu_cases::sha256;
    use guest_runtime::layout::{IPA_BASE, RUNTIME_IMAGE_VA, runtime_defines};
    use hypervisor::{Vcpu, Vm, memory};
    use w2m_vm::compile::HostCompiler;
    use w2m_vm::guest::{Guest, vm_compiler_symbols};
    use w2m_vm::process::Start;
    use w2m_vm::record::translation_lines;
    use w2m_vm::run::{Stop, run_compiling};

    /// The run's limit, as `tests/test_timed_waits.py`'s Machine sets it.
    const MAX_STEPS: u64 = 1 << 40;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap()
    }

    fn text(path: &Path) -> Result<String, String> {
        fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn main() -> Result<(), String> {
        let started = Instant::now();
        let args: Vec<String> = std::env::args().skip(1).collect();
        let usage = "usage: w2m-vm <guest.exe> --output <file>";
        let (guest_path, output) = match args.as_slice() {
            [guest, flag, output] if flag == "--output" => (guest, output),
            _ => return Err(usage.into()),
        };
        let root = root();
        // the Producer's input arrives as one JSON line; this run takes none, but the line is read so the pipe closes
        let mut input = String::new();
        io::stdin().lock().read_line(&mut input).map_err(|e| e.to_string())?;

        // the guest, as `tests/test_timed_waits.py` `run_guest` loads it: the program's own name probe.exe
        let data = fs::read(root.join(guest_path)).map_err(|e| format!("{guest_path}: {e}"))?;
        let loaded = pe_loader::load(&data, None, "probe.exe").map_err(|e| format!("{guest_path}: {e:?}"))?;
        let (_, image) = guest_runtime::build(&root, &runtime_defines(), RUNTIME_IMAGE_VA)
            .map_err(|e| format!("the guest runtime's build: {e:?}"))?;
        let services = guest_runtime::services::services(
            &guest_runtime::texts(&root).map_err(|e| format!("the guest runtime's sources: {e:?}"))?,
        )
        .map_err(|e| format!("the guest runtime's services: {e:?}"))?;
        let exports: serde_json::Value = serde_json::from_str(&text(&root.join("specs/windows_exports.json"))?)
            .map_err(|e| format!("specs/windows_exports.json: {e}"))?;
        let modules: BTreeSet<String> = exports["modules"]
            .as_object()
            .ok_or("specs/windows_exports.json holds no modules")?
            .keys()
            .cloned()
            .collect();
        let symbols = vm_compiler_symbols(&text(&root.join("tools/vm_compiler/stencil_table.h"))?)
            .map_err(|e| format!("tools/vm_compiler/stencil_table.h: {e:?}"))?;
        let mut guest = Guest::new(
            &loaded,
            &image,
            &services,
            &Start::console("probe.exe"),
            &modules,
            &symbols,
        )
        .map_err(|e| format!("the guest: {e:?}"))?;

        let vm = Vm::new(40).map_err(|e| format!("{e:?}"))?;
        vm.map(&guest.memory, IPA_BASE, memory::READ | memory::WRITE | memory::EXEC)
            .map_err(|e| format!("{e:?}"))?;
        let vcpu = Vcpu::new(&vm).map_err(|e| format!("{e:?}"))?;
        let mut host = HostCompiler::default();
        let out = run_compiling(&mut guest, &loaded, &vcpu, MAX_STEPS, &mut host).map_err(|e| format!("{e:?}"))?;

        fs::write(root.join(output), &guest.stdout).map_err(|e| format!("{output}: {e}"))?;
        if !matches!(out.stop, Stop::ExitProcess { .. }) {
            return Err(format!("{guest_path} stopped at {:?}, not at ExitProcess", out.stop));
        }
        let blocks: Vec<(u64, u64, u64)> = out.translations.iter().map(|t| (t.start, t.end, t.code)).collect();
        let mut stdout = io::stdout().lock();
        for line in translation_lines(&sha256(&data), loaded.entry, &blocks, started.elapsed()) {
            writeln!(stdout, "{line}").map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
