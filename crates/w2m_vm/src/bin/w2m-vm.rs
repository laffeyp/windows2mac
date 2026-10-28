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
//!
//! Step 3's measurements (Sprint 163; `run_measurements` runs these as its children):
//! - `w2m-vm <guest.exe> --output <file> --serve-times <json>` also writes each served exit's host time by the exit's
//!   kind, with the run's own exit tally, as JSON: `{"kinds": {"hvc 8": [ns, ...]}, "exits": {"8": n}}`.
//! - `w2m-vm --wfe-probe` runs a guest of 1,000 WFE and then, separately, 1,000 WFI, each ending in `hvc #0`, and
//!   prints `{"wfe": {"exits": n, "ended": bool}, "wfi": ...}`: the WFx traps (ESR EC 0x01, ISS bit 0 1 for WFE) the
//!   framework gave, and whether the run reached its `hvc #0` before a one-second watchdog stopped it (a WFE or WFI that
//!   does not exit and is not woken waits until then).

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

    /// The WFx probe's two runs (Sprint 163): each instruction 1,000 times, then `hvc #0`.
    const WFX_COUNT: usize = 1000;
    const WFE: u32 = 0xd503_205f; // clang, 2026-10-27
    const WFI: u32 = 0xd503_207f;
    const HVC_0: u32 = 0xd400_0002;
    /// EC 0x01: a trapped WFI or WFE (Arm ARM, ESR_ELx); ISS bit 0 (TI) 1 for WFE.
    const EC_WFX: u8 = 0x01;

    /// `--wfe-probe`: how many of each WFx the framework traps, and whether each run reached its `hvc #0`.
    fn wfe_probe() -> Result<(), String> {
        use hypervisor::{Exit, GuestMemory, Reg};
        use std::sync::mpsc;
        use std::time::Duration;
        const CODE_IPA: u64 = 0x1_0000;
        const WFI_AT: usize = 0x2000;
        let mut mem = GuestMemory::new(64 * 1024).map_err(|e| e.to_string())?;
        let program = |word: u32| {
            let mut words = vec![word; WFX_COUNT];
            words.push(HVC_0);
            words
        };
        mem.write_words(0, &program(WFE));
        mem.write_words(WFI_AT, &program(WFI));
        let vm = Vm::new(40).map_err(|e| format!("{e:?}"))?;
        vm.map(&mem, CODE_IPA, memory::READ | memory::EXEC)
            .map_err(|e| format!("{e:?}"))?;
        let vcpu = Vcpu::new(&vm).map_err(|e| format!("{e:?}"))?;
        let mut result = serde_json::Map::new();
        for (name, at, is_wfe) in [("wfe", 0usize, true), ("wfi", WFI_AT, false)] {
            vcpu.set_reg(Reg::Pc, CODE_IPA + at as u64)
                .map_err(|e| format!("{e:?}"))?;
            vcpu.set_reg(Reg::Cpsr, 0x3c5).map_err(|e| format!("{e:?}"))?; // EL1h, D A I F masked
            let (done, wait) = mpsc::channel::<()>();
            let handle = vcpu.handle();
            let (mut exits, mut ended) = (0u64, false);
            std::thread::scope(|s| -> Result<(), String> {
                // the watchdog: a WFx that neither exits nor wakes waits until it stops the vCPU
                s.spawn(move || {
                    if wait.recv_timeout(Duration::from_secs(1)).is_err() {
                        let _ = handle.stop();
                    }
                });
                loop {
                    let exit = vcpu.run().map_err(|e| format!("{e:?}"))?;
                    match exit {
                        Exit::VtimerActivated => vcpu.set_vtimer_mask(true).map_err(|e| format!("{e:?}"))?,
                        Exit::Canceled => break,
                        Exit::Exception { syndrome, .. } if exit.exception_class() == Some(EC_WFX) => {
                            if (syndrome & 1 == 1) != is_wfe {
                                return Err(format!("{name}: a trap of the other kind, ESR 0x{syndrome:x}"));
                            }
                            exits += 1;
                            let pc = vcpu.reg(Reg::Pc).map_err(|e| format!("{e:?}"))?;
                            vcpu.set_reg(Reg::Pc, pc + 4).map_err(|e| format!("{e:?}"))?; // the trapped WFx done
                        }
                        _ if exit.hvc_immediate() == Some(0) => {
                            ended = true;
                            break;
                        }
                        other => return Err(format!("{name}: an exit the probe does not expect, {other:?}")),
                    }
                }
                let _ = done.send(());
                Ok(())
            })?;
            let mut entry = serde_json::Map::new();
            entry.insert("exits".into(), exits.into());
            entry.insert("ended".into(), ended.into());
            result.insert(name.into(), entry.into());
        }
        println!("{}", serde_json::Value::Object(result));
        Ok(())
    }

    pub fn main() -> Result<(), String> {
        let started = Instant::now();
        let args: Vec<String> = std::env::args().skip(1).collect();
        if args == ["--wfe-probe"] {
            return wfe_probe();
        }
        let usage = "usage: w2m-vm <guest.exe> --output <file> [--serve-times <json>] | w2m-vm --wfe-probe";
        let (guest_path, output, serve_times) = match args.as_slice() {
            [guest, flag, output] if flag == "--output" => (guest, output, None),
            [guest, flag, output, times, file] if flag == "--output" && times == "--serve-times" => {
                (guest, output, Some(file))
            }
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
        if let Some(file) = serve_times {
            let kinds: serde_json::Map<String, serde_json::Value> = out
                .serve_by_kind
                .iter()
                .map(|(k, v)| (k.clone(), v.clone().into()))
                .collect();
            let exits: serde_json::Map<String, serde_json::Value> =
                out.exits.iter().map(|(k, v)| (k.to_string(), (*v).into())).collect();
            let json = serde_json::json!({ "kinds": kinds, "exits": exits });
            fs::write(file, json.to_string()).map_err(|e| format!("{file}: {e}"))?;
        }
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
