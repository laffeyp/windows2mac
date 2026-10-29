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
    use w2m_vm::run::{Outcome, Stop, run_compiling};

    /// The run's limit, as `tests/test_timed_waits.py`'s Machine sets it.
    const MAX_STEPS: u64 = 1 << 40;
    /// a guest run's limit, family_run.py `run_in_vm`'s `timeout_ms=60_000` (Sprint 197)
    const GUEST_SECONDS: u64 = 60;

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
        let mut args: Vec<String> = std::env::args().skip(1).collect();
        // a guest on the instruction clock, `family_run.INSTRUCTION_CLOCK` as `run_guest` passes it (Sprint 199)
        let instruction_clock = args.last().is_some_and(|a| a == "--instruction-clock");
        if instruction_clock {
            args.pop();
        }
        // a file beside probe.exe under its name, as `run_in_vm` writes a family's DLL when `dll_on_disk` (Sprint 204)
        let beside = match args.iter().position(|a| a == "--beside") {
            Some(i) if args.len() >= i + 3 => {
                let file: Vec<String> = args.drain(i..i + 3).skip(1).collect();
                Some((file[0].clone(), file[1].clone()))
            }
            Some(_) => return Err("--beside <name> <file>".into()),
            None => None,
        };
        if args == ["--wfe-probe"] {
            return wfe_probe();
        }
        let usage = "usage: w2m-vm <guest.exe> --output <file> [--serve-times <json> | --runtime-word <json> | --stop \
                     <tsv>] [--beside <name> <file>] [--instruction-clock] \
                     | w2m-vm --game <directory> <program.exe> --output <file> --stop <tsv> | w2m-vm --wfe-probe";
        if let [flag, page_fd, memory_fd, slot] = args.as_slice()
            && flag == "--child"
        {
            let n = |s: &String| s.parse::<u64>().map_err(|e| format!("--child {s}: {e}"));
            return child(n(page_fd)? as i32, n(memory_fd)? as i32, n(slot)? as usize);
        }
        if let [flag, directory, program, out_flag, output, stop_flag, stop_file] = args.as_slice()
            && flag == "--game"
            && out_flag == "--output"
            && stop_flag == "--stop"
        {
            return game(Path::new(directory), program, output, stop_file);
        }
        // (guest, output, the measure asked for and its file); every path relative to the repository root
        let (guest_path, output, measure) = match args.as_slice() {
            [guest, flag, output] if flag == "--output" => (guest, output, None),
            [guest, flag, output, which, file]
                if flag == "--output"
                    && (which == "--serve-times" || which == "--runtime-word" || which == "--stop") =>
            {
                (guest, output, Some((which.as_str(), file)))
            }
            _ => return Err(usage.into()),
        };
        let mut start = Start::console("probe.exe");
        start.time_runtime_word = matches!(measure, Some(("--runtime-word", _)));
        if instruction_clock {
            start.instruction_clock = Some((CLOCK_RATE, REFERENCE_START));
        }
        let root = root();
        start.program_path = Some(root.join(guest_path));
        // the Producer's input arrives as one JSON line; this run takes none, but the line is read so the pipe closes
        let mut input = String::new();
        io::stdin().lock().read_line(&mut input).map_err(|e| e.to_string())?;

        // the guest, as `tests/test_timed_waits.py` `run_guest` loads it: the program's own name probe.exe
        let data = fs::read(root.join(guest_path)).map_err(|e| format!("{guest_path}: {e}"))?;
        let loaded = pe_loader::load(&data, None, "probe.exe").map_err(|e| format!("{guest_path}: {e:?}"))?;
        // C:\w2m\app: a fresh directory holding the program as probe.exe, as `run_guest` and `run_in_vm` make it
        // (Sprint 204)
        let app = std::env::temp_dir().join(format!("w2m-app-{}", std::process::id()));
        let _ = fs::remove_dir_all(&app);
        fs::create_dir_all(&app).map_err(|e| format!("{}: {e}", app.display()))?;
        fs::write(app.join("probe.exe"), &data).map_err(|e| format!("{}: {e}", app.display()))?;
        if let Some((name, file)) = &beside {
            fs::copy(root.join(file), app.join(name)).map_err(|e| format!("--beside {file}: {e}"))?;
        }
        start.app_root = Some(app.clone());
        let mut guest = guest_of(&root, &loaded, &start)?;
        guest.files = Some(w2m_vm::file_bridge::FileBridge::new(&app, false, None)?);
        let out = on_one_vcpu(&mut guest, &loaded, true);
        drop(guest.files.take());
        let _ = fs::remove_dir_all(&app);
        let out = out?;

        fs::write(root.join(output), &guest.stdout).map_err(|e| format!("{output}: {e}"))?;
        if let Some(("--stop", file)) = measure {
            // the guest's stop by name, as the game's is written (Sprint 191; reviewer-2's Medium 4 of its review of
            // 185-189: `gates.sh` had read it from stderr with `sed`)
            fs::write(root.join(file), format!("{}\n", stop_name(&out.stop))).map_err(|e| format!("{file}: {e}"))?;
        } else if let Some((which, file)) = measure {
            let json = if which == "--serve-times" {
                let kinds: serde_json::Map<String, serde_json::Value> = out
                    .serve_by_kind
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone().into()))
                    .collect();
                let exits: serde_json::Map<String, serde_json::Value> =
                    out.exits.iter().map(|(k, v)| (k.to_string(), (*v).into())).collect();
                serde_json::json!({ "kinds": kinds, "exits": exits })
            } else {
                // the runtime word's takings (Sprint 164): each as [site, wait start, taken, released, the exit count
                // at each of the three], counter ticks
                let w = &out.runtime_word;
                let takings: Vec<[u64; 7]> = w
                    .takings
                    .iter()
                    .map(|t| {
                        [
                            t.site,
                            t.wait_start,
                            t.taken,
                            t.released,
                            t.exits_at_wait_start,
                            t.exits_at_taken,
                            t.exits_at_released,
                        ]
                    })
                    .collect();
                serde_json::json!({ "counted": w.counted, "takings": takings })
            };
            // relative to the repository root, as `--output` is (the review of 2026-10-27 found it relative to the
            // working directory)
            fs::write(root.join(file), json.to_string()).map_err(|e| format!("{file}: {e}"))?;
        }
        if !matches!(out.stop, Stop::ExitProcess { .. }) {
            return Err(format!(
                "{guest_path} stopped at {}, not at ExitProcess",
                stop_name(&out.stop)
            ));
        }
        let blocks: Vec<(u64, u64, u64)> = out.translations.iter().map(|t| (t.start, t.end, t.code)).collect();
        let mut stdout = io::stdout().lock();
        for line in translation_lines(&sha256(&data), loaded.entry, &blocks, started.elapsed()) {
            writeln!(stdout, "{line}").map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// The guest for `loaded` and `start`: the runtime image the build makes, its services, the exported modules and
    /// the VM compiler's symbols, as `tests/test_timed_waits.py` builds a VM.
    fn guest_of(root: &Path, loaded: &pe_loader::Loaded, start: &Start) -> Result<Guest, String> {
        let (_, image) = guest_runtime::build(root, &runtime_defines(), RUNTIME_IMAGE_VA)
            .map_err(|e| format!("the guest runtime's build: {e:?}"))?;
        let services = guest_runtime::services::services(
            &guest_runtime::texts(root).map_err(|e| format!("the guest runtime's sources: {e:?}"))?,
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
        Guest::new(loaded, &image, &services, start, &modules, &symbols).map_err(|e| format!("the guest: {e:?}"))
    }

    /// The guest run on one vCPU, the host compiling the blocks the VM's C declines, to its stop.
    fn on_one_vcpu(guest: &mut Guest, loaded: &pe_loader::Loaded, limited: bool) -> Result<Outcome, String> {
        let vm = Vm::new(40).map_err(|e| format!("{e:?}"))?;
        vm.map(&guest.memory, IPA_BASE, memory::READ | memory::WRITE | memory::EXEC)
            .map_err(|e| format!("{e:?}"))?;
        let vcpu = Vcpu::new(&vm).map_err(|e| format!("{e:?}"))?;
        let mut host = HostCompiler::default();
        // a guest's run stops at GUEST_SECONDS, as family_run.py's `run_in_vm` stops a probe at 60 s (`timeout_ms`), so
        // a guest that spins fails by name rather than hanging the gates (Sprint 197); the batch run has no limit
        guest.limited = limited;
        if limited {
            let handle = vcpu.handle();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(GUEST_SECONDS));
                let _ = handle.stop();
            });
        }
        run_compiling(guest, loaded, &vcpu, MAX_STEPS, &mut host).map_err(|e| format!("{e:?}"))
    }

    /// A stop by its kind and reason, without host addresses, which move when the runtime image is rebuilt (Sprint
    /// 186): what `store/batch_run/first-stop-2026-10-28.tsv` holds.
    fn stop_name(stop: &Stop) -> String {
        match stop {
            Stop::HostRequest { op, why, .. } => format!("HostRequest op {op}: {why}"),
            Stop::Hvc { number, .. } => format!("Hvc {number}"),
            Stop::Handover { why, .. } => format!("Handover: {why}"),
            Stop::Model { rip, why } => format!("Model at 0x{rip:x}: {why}"),
            other => format!("{other:?}"),
        }
    }

    /// `USER_ENVIRONMENT` (vm_backend 342-343): the environment the batch run's game starts with.
    const USER_ENVIRONMENT: [&str; 3] = [
        "TEMP=C:\\Users\\w2m\\AppData\\Local\\Temp",
        "TMP=C:\\Users\\w2m\\AppData\\Local\\Temp",
        "USERPROFILE=C:\\Users\\runneradmin",
    ];
    /// `BATCH_ARGUMENTS` (scripts/run_frontier.py 64)
    const BATCH_ARGUMENTS: &str = "-batchmode -nographics -logFile \"C:\\w2m\\unity.log\"";
    /// `CLOCK_RATE` and `REFERENCE_START` (scripts/run_frontier.py 68-69): instructions a second, and game run
    /// 38216081440's start as a FILETIME
    const CLOCK_RATE: u64 = 1_400_000_000;
    const REFERENCE_START: u64 = 134_362_624_920_000_000;

    /// The batch run on the Rust host (Sprint 186; reviewer-2's High 1 of its review of 182-184): `program` in
    /// `directory`, its DLLs supplied from the directory, started as `scripts/run_frontier.py` 266-273 starts it (a GUI
    /// program, foreground held, `USER_ENVIRONMENT`, `BATCH_ARGUMENTS`, the instruction clock), run on one vCPU to its
    /// stop; the guest's output to `output` and the stop to `stop_file`, each relative to the repository root.
    /// `w2m-vm --child <turn page fd> <memory fd> <slot>` (Sprint 197): a child process a supervisor `w2m-vm` started
    /// for a PROCESS_CREATE (`spawner`, kernel.py 437-468), its start one JSON line on standard input. Its program is
    /// the command line's first token, quoted or not, found in C:\w2m\app on the host or, for a run with no program
    /// directory, the parent's own program when the name is its name; its DLLs from the program's directory. It runs
    /// in its turns and tells the supervisor its end or its stop.
    fn child(page_fd: i32, memory_fd: i32, slot: usize) -> Result<(), String> {
        let mut line = String::new();
        io::stdin().lock().read_line(&mut line).map_err(|e| e.to_string())?;
        let s: serde_json::Value = serde_json::from_str(&line).map_err(|e| format!("a child's start: {e}"))?;
        let text = |k: &str| s[k].as_str().map(str::to_string);
        let number = |k: &str| s[k].as_u64().ok_or_else(|| format!("a child's start lacks {k}"));
        let pid = number("pid")?;
        let link = w2m_vm::turns::ChildLink::open(page_fd, slot, pid)?;
        w2m_vm::turns::end_with_parent()?;
        link.first_turn();
        let run = || -> Result<Outcome, String> {
            let command_line = text("command_line").ok_or("a child's start lacks its command line")?;
            let first = if let Some(rest) = command_line.strip_prefix('"') {
                rest.split('"').next().unwrap_or("").to_string()
            } else {
                command_line.split(' ').next().unwrap_or("").to_string()
            };
            let leaf = first.rsplit(['\\', '/']).next().unwrap_or("").to_string();
            let path = match (text("app_root"), text("program_path"), text("parent_image")) {
                (Some(root), _, _) => w2m_vm::modules::host_file(&PathBuf::from(root), &first)?,
                (None, Some(program), Some(parent)) if leaf.eq_ignore_ascii_case(&parent) => {
                    Some(PathBuf::from(program))
                }
                _ => None,
            }
            .ok_or_else(|| {
                format!("CreateProcessW of {first:?}: no such file (ERROR_FILE_NOT_FOUND is not in the rows)")
            })?;
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let data = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let here = path.parent().map(Path::to_path_buf).unwrap_or_default();
            let supply = |dll: &str| fs::read(here.join(dll)).ok();
            let loaded = pe_loader::load(&data, Some(&supply), &name).map_err(|e| format!("{name}: {e:?}"))?;
            let mut start = Start::console(&name);
            start.gui = loaded.image.as_ref().is_some_and(|i| i.subsystem == 2);
            start.command_line = Some(command_line.clone());
            start.environment = s["environment"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|e| e.as_str().map(str::to_string))
                .collect();
            start.process_id = pid;
            start.tid = number("tid")?;
            start.inherited = Some(
                s["inherited"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|e| Some((e[0].as_u64()?, e[1].as_u64()?, e[2].as_u64()?)))
                    .collect(),
            );
            start.directory = text("directory");
            start.process_group = number("process_group")? as u32;
            start.instruction_clock = s["instruction_clock"]
                .as_array()
                .and_then(|c| Some((c.first()?.as_u64()?, c.get(1)?.as_u64()?)));
            start.app_root = text("app_root").map(PathBuf::from);
            start.program_path = Some(path.clone());
            start.memory_fd = Some(memory_fd);
            let root = root();
            let mut guest = guest_of(&root, &loaded, &start)?;
            // a child's bridge: the parent's root, read-only setting and drive (kernel.py 460; Sprint 204)
            if let (Some(app), Some(drive)) = (&start.app_root, text("drive")) {
                let read_only = s["read_only"].as_bool().unwrap_or(false);
                guest.files = Some(w2m_vm::file_bridge::FileBridge::new(
                    app,
                    read_only,
                    Some(Path::new(&drive)),
                )?);
            }
            guest.link = Some(w2m_vm::turns::Link::Child(w2m_vm::turns::ChildLink::open(
                page_fd, slot, pid,
            )?));
            // the child starts at the machine's clock, as `VM.steps` catches every process up at its start (Sprint 199;
            // reviewer-2's High 1 of its review of 196-198: tests/test_timed_waits.py 141-152's plant gives "clock behind")
            w2m_vm::threads::catch_up(&mut guest, link.machine(), 2);
            on_one_vcpu(&mut guest, &loaded, s["limited"].as_bool().unwrap_or(true))
        };
        match run() {
            Ok(out) => match out.stop {
                Stop::ExitProcess { code } => link.end(u64::from(code)),
                other => link.stop(&stop_name(&other)),
            },
            Err(why) => link.stop(&why),
        }
        Ok(())
    }

    fn game(directory: &Path, program: &str, output: &str, stop_file: &str) -> Result<(), String> {
        let root = root();
        let started = Instant::now();
        let data = fs::read(directory.join(program)).map_err(|e| format!("{program}: {e}"))?;
        let supply = |name: &str| fs::read(directory.join(name)).ok();
        let loaded = pe_loader::load(&data, Some(&supply), program).map_err(|e| format!("{program}: {e:?}"))?;
        let mut start = Start::console(program);
        start.gui = true;
        start.foreground_held = true;
        start.environment = USER_ENVIRONMENT.iter().map(|s| s.to_string()).collect();
        start.command_line = Some(format!("\"C:\\w2m\\app\\{program}\" {BATCH_ARGUMENTS}"));
        start.instruction_clock = Some((CLOCK_RATE, REFERENCE_START));
        // the call trace as `run_frontier.py` 271-272 asks it (Sprint 191): UnityPlayer.dll's and mono's calls, from
        // UnityMain
        start.call_trace = vec!["UnityPlayer.dll".into(), "mono-2.0-bdwgc.dll".into()];
        start.call_trace_from = Some("UnityPlayer.dll!UnityMain".into());
        // the depot is C:\w2m\app, the Python's file bridge root (`host_file`, `in_app_directory`; Sprint 196)
        start.app_root = Some(directory.to_path_buf());
        start.program_path = Some(directory.join(program));
        let mut guest = guest_of(&root, &loaded, &start)?;
        // the depot mounted read-only, as `run_frontier.py` 268 makes it (Sprint 204)
        guest.files = Some(w2m_vm::file_bridge::FileBridge::new(directory, true, None)?);
        let out = on_one_vcpu(&mut guest, &loaded, false)?;
        fs::write(root.join(output), &guest.stdout).map_err(|e| format!("{output}: {e}"))?;
        let mut tsv = format!(
            "stop\t{}\nseconds\t{:.1}\n",
            stop_name(&out.stop),
            started.elapsed().as_secs_f64()
        );
        if let Stop::HostRequest { op, .. } = out.stop {
            // the request's arguments on the transport page (vm_backend 3808-3812); HOST_MODULE's name is UTF-16 at a0,
            // a1 characters (3892-3893)
            use guest_runtime::layout::{MONITOR_SIZE, TRANSPORT_OFF};
            let mem = guest.memory.as_slice();
            let word = |k: u64| {
                let at = (MONITOR_SIZE + TRANSPORT_OFF + 8 * k) as usize;
                u64::from_le_bytes(mem[at..at + 8].try_into().unwrap())
            };
            let (a0, a1, a2) = (word(1), word(2), word(3));
            tsv += &format!("arguments\t0x{a0:x} 0x{a1:x} 0x{a2:x}\n");
            if op == 15
                && let Some(at) = guest.layout.offset(a0)
            {
                let units: Vec<u16> = mem[at as usize..at as usize + 2 * a1 as usize]
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|c| u16::from_le_bytes(*c))
                    .collect();
                tsv += &format!(
                    "module asked for\t{} (loading {a2})\n",
                    String::from_utf16_lossy(&units)
                );
            }
        }
        tsv += &format!(
            "images\t{}\n",
            loaded
                .images
                .iter()
                .map(|i| i.name.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        );
        for (number, n) in &out.exits {
            tsv += &format!("hvc {number}\t{n}\n");
        }
        for (name, n) in &out.counts {
            tsv += &format!("{name}\t{n}\n");
        }
        tsv += &format!("blocks the host compiled\t{}\n", out.translations.len());
        // the call stream at the stop, in `scripts/call_compare.py` `write_tsv`'s form, beside the stop file (Sprint 191)
        let (calls, in_flight, dropped) = guest.traced_calls()?;
        let calls_file = format!("{}.calls.tsv", stop_file.trim_end_matches(".tsv"));
        fs::write(root.join(&calls_file), &calls).map_err(|e| format!("{calls_file}: {e}"))?;
        // the calls not yet returned at the stop (Sprint 194), without their return and error
        let in_flight_file = format!("{}.in-flight.tsv", stop_file.trim_end_matches(".tsv"));
        fs::write(root.join(&in_flight_file), &in_flight).map_err(|e| format!("{in_flight_file}: {e}"))?;
        tsv += &format!(
            "calls traced\t{}\ncalls in flight\t{}\ncalls dropped\t{dropped}\n",
            calls.lines().count(),
            in_flight.lines().count()
        );
        fs::write(root.join(stop_file), &tsv).map_err(|e| format!("{stop_file}: {e}"))?;
        print!("{tsv}");
        Ok(())
    }
}
