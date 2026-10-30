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
    /// the batch run's limit, `run_frontier.py`'s `--timeout-seconds` default (Sprint 210), and the grace its record
    /// states, `KILL_GRACE_SECONDS` (run_frontier.py 73)
    const GAME_SECONDS: u64 = 10_800;
    const KILL_GRACE_SECONDS: u64 = 60;

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

    /// A family's start options into `start`, as `vm_inputs` gives them (`family_run.py` 2889-2902 at the tag; Sprint
    /// 231): the command line `"C:\w2m\app\probe.exe" <arguments>`, the environment block, the GUI start, the
    /// foreground held, the instruction clock. Answers the output file the rows are read from. A key `vm_inputs` does
    /// not give is an error: a run must start as its family starts, or not at all.
    fn family_options(start: &mut Start, options: &serde_json::Value) -> Result<Option<String>, String> {
        let options = options.as_object().ok_or("--start: not a JSON object")?;
        let text = |v: &serde_json::Value, k: &str| {
            v.as_str()
                .map(str::to_string)
                .ok_or(format!("--start: {k} is not a string"))
        };
        let flag = |v: &serde_json::Value, k: &str| v.as_bool().ok_or(format!("--start: {k} is not true or false"));
        let mut output_file = None;
        for (k, v) in options {
            match k.as_str() {
                "arguments" => start.command_line = Some(format!("\"C:\\w2m\\app\\probe.exe\" {}", text(v, k)?)),
                "environment" => {
                    let block = v.as_array().ok_or("--start: environment is not a list")?;
                    start.environment = block.iter().map(|e| text(e, k)).collect::<Result<_, _>>()?;
                }
                "gui" => start.gui = flag(v, k)?,
                "foreground_held" => start.foreground_held = flag(v, k)?,
                "instruction_clock" => {
                    if flag(v, k)? {
                        start.instruction_clock = Some((CLOCK_RATE, REFERENCE_START));
                    }
                }
                "output_file" => output_file = Some(text(v, k)?),
                other => return Err(format!("--start: {other} is not a start option vm_inputs gives")),
            }
        }
        Ok(output_file)
    }

    pub fn main() -> Result<(), String> {
        let started = Instant::now();
        let mut args: Vec<String> = std::env::args().skip(1).collect();
        // the QoS class this process runs at, for a measurement's baseline (Sprint 240): what the run that follows
        // from the same shell inherits
        if args.first().is_some_and(|a| a == "--qos-class") {
            println!("{}", w2m_vm::host_state::qos_class());
            return Ok(());
        }
        // a guest on the instruction clock, `family_run.INSTRUCTION_CLOCK` as `run_guest` passes it (Sprint 199); taken
        // wherever it stands, as the other flags are (Sprint 230: read only as the last argument, it was left in the
        // list behind `--max-steps`, and the run stopped at the usage)
        let instruction_clock = match args.iter().position(|a| a == "--instruction-clock") {
            Some(i) => {
                args.remove(i);
                true
            }
            None => false,
        };
        // the in-VM compiler's calls timed by the monitor code (Sprint 247), for a measurement run; the gates' run
        // leaves it off, so its monitor code is the Python's word for word
        let time_compile = match args.iter().position(|a| a == "--time-compile") {
            Some(i) => {
                args.remove(i);
                true
            }
            None => false,
        };
        // a family's start (Sprint 231): the JSON object `family_known` prints in the lookup's column 7
        let family_start = match args.iter().position(|a| a == "--start") {
            Some(i) if args.len() >= i + 2 => {
                let text = args.drain(i..i + 2).nth(1).unwrap();
                Some(serde_json::from_str::<serde_json::Value>(&text).map_err(|e| format!("--start {text}: {e}"))?)
            }
            Some(_) => return Err("--start <json>".into()),
            None => None,
        };
        // a file beside probe.exe under its name, as `run_in_vm` writes a family's DLL when `dll_on_disk` (Sprint 204)
        let beside = match args.iter().position(|a| a == "--beside") {
            Some(i) if args.len() >= i + 3 => {
                let file: Vec<String> = args.drain(i..i + 3).skip(1).collect();
                Some((file[0].clone(), file[1].clone()))
            }
            Some(_) => return Err("--beside <name> <file>".into()),
            None => None,
        };
        // the exit ledger (Sprint 206): this process's rows at the path, each child's at the path and ".<pid>"
        let ledger = match args.iter().position(|a| a == "--exit-ledger") {
            Some(i) if args.len() >= i + 2 => Some(args.drain(i..i + 2).nth(1).unwrap()),
            Some(_) => return Err("--exit-ledger <tsv>".into()),
            None => None,
        };
        // the whole-block judge (Sprint 252): each in-VM compile against the Rust translator's, written to the path
        let judge = match args.iter().position(|a| a == "--judge-blocks") {
            Some(i) if args.len() >= i + 2 => Some(args.drain(i..i + 2).nth(1).unwrap()),
            Some(_) => return Err("--judge-blocks <tsv>".into()),
            None => None,
        };
        // the batch run's end (Sprint 219): the reference log's line count and the Windows main thread's calls
        let log_lines = match args.iter().position(|a| a == "--log-lines") {
            Some(i) if args.len() >= i + 2 => {
                let n = args.drain(i..i + 2).nth(1).unwrap();
                Some(n.parse::<usize>().map_err(|e| format!("--log-lines {n}: {e}"))?)
            }
            Some(_) => return Err("--log-lines <count>".into()),
            None => None,
        };
        let main_calls = match args.iter().position(|a| a == "--main-thread-calls") {
            Some(i) if args.len() >= i + 2 => Some(args.drain(i..i + 2).nth(1).unwrap()),
            Some(_) => return Err("--main-thread-calls <tsv>".into()),
            None => None,
        };
        // a guest run's instruction budget (Sprint 221): the family sweep ends a hang by count
        let max_steps = match args.iter().position(|a| a == "--max-steps") {
            Some(i) if args.len() >= i + 2 => {
                let n = args.drain(i..i + 2).nth(1).unwrap();
                Some(n.parse::<u64>().map_err(|e| format!("--max-steps {n}: {e}"))?)
            }
            Some(_) => return Err("--max-steps <instructions>".into()),
            None => None,
        };
        let end = match (log_lines, main_calls) {
            (Some(lines), Some(file)) => Some((lines, file)),
            (None, None) => None,
            _ => return Err("--log-lines and --main-thread-calls go together".into()),
        };
        if args == ["--wfe-probe"] {
            return wfe_probe();
        }
        let usage = "usage: w2m-vm <guest.exe> --output <file> [--serve-times <json> | --runtime-word <json> | --stop \
                     <tsv>] [--beside <name> <file>] [--exit-ledger <tsv>] [--instruction-clock] \
                     | w2m-vm --game <directory> <program.exe> --output <file> --stop <tsv> [--log-lines <count> \
                     --main-thread-calls <tsv>] [--time-compile] | w2m-vm --wfe-probe";
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
            return game(
                Path::new(directory),
                program,
                output,
                stop_file,
                GameOptions {
                    ledger: ledger.as_deref(),
                    end,
                    time_compile,
                    judge: judge.as_deref(),
                },
            );
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
        let output_file = match &family_start {
            Some(options) => family_options(&mut start, options)?,
            None => None,
        };
        let root = root();
        start.program_path = Some(root.join(guest_path));
        // the Producer's input arrives as one JSON line; this run takes none, but the line is read so the pipe closes
        let mut input = String::new();
        io::stdin().lock().read_line(&mut input).map_err(|e| e.to_string())?;

        // the guest, as `tests/test_timed_waits.py` `run_guest` loads it: the program's own name probe.exe
        let data = fs::read(root.join(guest_path)).map_err(|e| format!("{guest_path}: {e}"))?;
        // the DLL beside the program is the loader's to map when the program imports it (`run_in_vm`'s supply; Sprint
        // 221: static_tls, loader and threads import their own DLL, and with no supply its exports bound to thunks no
        // service answers, 34 family runs stopping on "import <dll>!<export>")
        let beside_bytes = match &beside {
            Some((_, file)) => Some(fs::read(root.join(file)).map_err(|e| format!("--beside {file}: {e}"))?),
            None => None,
        };
        let supply = |dll: &str| match (&beside, &beside_bytes) {
            (Some((name, _)), Some(bytes)) if name.eq_ignore_ascii_case(dll) => Some(bytes.clone()),
            _ => None,
        };
        let loaded = pe_loader::load(&data, Some(&supply), "probe.exe").map_err(|e| format!("{guest_path}: {e:?}"))?;
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
        guest.limited = true;
        let out = on_one_vcpu(&mut guest, &loaded, Some(GUEST_SECONDS), max_steps.unwrap_or(MAX_STEPS));
        drop(guest.files.take());
        // a family's rows from the file its probe writes in the program's directory, empty if it wrote none
        // (`run_in_vm`, family_run.py 2949-2952 at the tag), read before the directory goes; a file not written is
        // named on stderr, so a held run's cause is not only its missing rows (reviewer-3's Low 1 of its review at 231)
        let rows = match &output_file {
            Some(name) => fs::read(app.join(name)).unwrap_or_else(|_| {
                eprintln!("w2m-vm: the output file {name} was not written; its rows are empty");
                Vec::new()
            }),
            None => guest.stdout.clone(),
        };
        let _ = fs::remove_dir_all(&app);
        let out = out?;

        fs::write(root.join(output), &rows).map_err(|e| format!("{output}: {e}"))?;
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

    /// The guest run on one vCPU, the host compiling the blocks the VM's C declines, to its stop, `seconds`, or
    /// `max_steps` instructions (`--max-steps`, Sprint 221: a family run that hangs ends at the same step on every run,
    /// not at a wall-clock limit).
    fn on_one_vcpu(
        guest: &mut Guest,
        loaded: &pe_loader::Loaded,
        seconds: Option<u64>,
        max_steps: u64,
    ) -> Result<Outcome, String> {
        let vm = Vm::new(40).map_err(|e| format!("{e:?}"))?;
        vm.map(&guest.memory, IPA_BASE, memory::READ | memory::WRITE | memory::EXEC)
            .map_err(|e| format!("{e:?}"))?;
        let vcpu = Vcpu::new(&vm).map_err(|e| format!("{e:?}"))?;
        let mut host = HostCompiler::default();
        // a guest's run stops at GUEST_SECONDS, as family_run.py's `run_in_vm` stops a probe at 60 s (`timeout_ms`), so
        // a guest that spins fails by name rather than hanging the gates (Sprint 197); the batch run at GAME_SECONDS,
        // the bound its record states (Sprint 210)
        if let Some(seconds) = seconds {
            let handle = vcpu.handle();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(seconds));
                let _ = handle.stop();
            });
        }
        run_compiling(guest, loaded, &vcpu, max_steps, &mut host).map_err(|e| format!("{e:?}"))
    }

    /// A stop by its kind and reason, without host addresses, which move when the runtime image is rebuilt (Sprint
    /// 186): what `store/batch_run/first-stop-2026-10-28.tsv` holds.
    fn stop_name(stop: &Stop) -> String {
        match stop {
            Stop::HostRequest { op, why, .. } => format!("HostRequest op {op}: {why}"),
            Stop::HostFailure { op, why, .. } => format!("HostFailure op {op}: {why}"),
            Stop::Hvc { number, .. } => format!("Hvc {number}"),
            Stop::Handover { why, .. } => format!("Handover: {why}"),
            Stop::Model { rip, why } => format!("Model at 0x{rip:x}: {why}"),
            Stop::TimeLimit => "the run's time limit".into(),
            Stop::StepLimit { count, max, pc } => format!("StepLimit: {count} of {max} steps at pc 0x{pc:x}"),
            Stop::ReferenceEnd(why) => format!("ReferenceEnd: {why}"),
            // the frozen host's `Unimplemented(f"import {name}")` (vm_backend 5263)
            Stop::Import { name, .. } => format!("import {name}"),
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
            // the parent's exit ledger, this process's rows beside it (Sprint 206)
            if let Some(base) = text("exit_ledger") {
                let path = std::path::PathBuf::from(format!("{base}.{pid}"));
                guest.exit_ledger = Some(w2m_vm::exit_ledger::Ledger::create(&path)?);
            }
            guest.link = Some(w2m_vm::turns::Link::Child(w2m_vm::turns::ChildLink::open(
                page_fd, slot, pid,
            )?));
            // the child starts at the machine's clock, as `VM.steps` catches every process up at its start (Sprint 199;
            // reviewer-2's High 1 of its review of 196-198: tests/test_timed_waits.py 141-152's plant gives "clock behind")
            w2m_vm::threads::catch_up(&mut guest, link.machine(), 2);
            guest.limited = s["limited"].as_bool().unwrap_or(true);
            let seconds = guest.limited.then_some(GUEST_SECONDS);
            on_one_vcpu(&mut guest, &loaded, seconds, MAX_STEPS)
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

    /// How a game run is watched beside its record: the exit ledger (Sprint 206), the batch run's end (Sprint 219),
    /// the in-VM compiler timed (Sprint 247), the whole-block judge (Sprint 252).
    struct GameOptions<'a> {
        ledger: Option<&'a str>,
        end: Option<(usize, String)>,
        time_compile: bool,
        judge: Option<&'a str>,
    }

    fn game(
        directory: &Path,
        program: &str,
        output: &str,
        stop_file: &str,
        options: GameOptions<'_>,
    ) -> Result<(), String> {
        let GameOptions {
            ledger,
            end,
            time_compile,
            judge,
        } = options;
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
        start.time_compile = time_compile;
        // the Producer's input, one JSON line, read so the pipe closes, as a guest run reads it
        let mut input = String::new();
        io::stdin().lock().read_line(&mut input).map_err(|e| e.to_string())?;
        let mut guest = guest_of(&root, &loaded, &start)?;
        // the depot mounted read-only, as `run_frontier.py` 268 makes it (Sprint 204)
        guest.files = Some(w2m_vm::file_bridge::FileBridge::new(directory, true, None)?);
        if let Some(path) = ledger {
            guest.exit_ledger = Some(w2m_vm::exit_ledger::Ledger::create(&root.join(path))?);
        }
        if let Some(path) = judge {
            guest.block_judge = Some(w2m_vm::block_judge::BlockJudge::create(&root.join(path))?);
        }
        // the game's log, C:\w2m\unity.log on the bridge's drive (`run_frontier.py` 288, `files.drive / "w2m" /
        // "unity.log"`); the run ends where its reference did (Sprint 219)
        let log = guest.files.as_ref().map(|f| f.drive.join("w2m").join("unity.log"));
        if let (Some((lines, file)), Some(log)) = (&end, &log) {
            let calls = w2m_vm::batch_stop::read_calls(&text(&root.join(file))?)?;
            guest.batch_stop = Some(w2m_vm::batch_stop::BatchStop::new(log.clone(), *lines, calls));
        }
        let out = on_one_vcpu(&mut guest, &loaded, Some(GAME_SECONDS), MAX_STEPS)?;
        // the log beside the stop file, which `batch_run.rs` holds to the reference's, line for line (Sprint 219)
        if let Some(log) = &log {
            let beside = format!("{}.unity.log", stop_file.trim_end_matches(".tsv"));
            fs::write(root.join(&beside), fs::read(log).unwrap_or_default()).map_err(|e| format!("{beside}: {e}"))?;
        }
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
        let w2m_vm::guest::Traced {
            calls,
            in_flight,
            dropped,
            strings,
        } = guest.traced()?;
        let calls_file = format!("{}.calls.tsv", stop_file.trim_end_matches(".tsv"));
        fs::write(root.join(&calls_file), &calls).map_err(|e| format!("{calls_file}: {e}"))?;
        // the calls' string arguments beside them, as run_frontier.py writes `<run>.strings.tsv` (Sprint 214): the
        // comparison against Windows names the game's threads by them
        let strings_file = format!("{}.calls.strings.tsv", stop_file.trim_end_matches(".tsv"));
        fs::write(root.join(&strings_file), strings).map_err(|e| format!("{strings_file}: {e}"))?;
        // the calls not yet returned at the stop (Sprint 194), without their return and error
        let in_flight_file = format!("{}.in-flight.tsv", stop_file.trim_end_matches(".tsv"));
        fs::write(root.join(&in_flight_file), &in_flight).map_err(|e| format!("{in_flight_file}: {e}"))?;
        tsv += &format!(
            "calls traced\t{}\ncalls in flight\t{}\ncalls dropped\t{dropped}\n",
            calls.lines().count(),
            in_flight.lines().count()
        );
        fs::write(root.join(stop_file), &tsv).map_err(|e| format!("{stop_file}: {e}"))?;
        // the run's execution record on standard output (Sprint 210), as `run_frontier.py` writes the batch run's
        let subject = sha256(&data);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let id = format!("frontier-{}-{nanos}", &subject[..12]);
        // the stopping thread's calls in flight at the stop, "DLL!Function" (`write_tsv`'s columns 2, 4 and 5): another
        // thread's call of the same function does not vouch for the word on this thread's stack (reviewer-3's Medium 1
        // of its review of Sprint 211)
        let stopped = w2m_vm::threads::current_tid(&guest).to_string();
        let flying: Vec<String> = in_flight
            .lines()
            .filter_map(|l| {
                let cells: Vec<&str> = l.split('\t').collect();
                (*cells.get(1)? == stopped).then(|| format!("{}!{}", cells[3], cells[4]))
            })
            .collect();
        // a service the runtime reported unimplemented names itself (HOST_UNIMPLEMENTED, Sprint 219), whatever image
        // called it; any other unserved request is named from the call the trace has in flight
        let end = match guest.unimplemented.clone() {
            Some((service, return_address)) => w2m_vm::record::RunEnd::Unimplemented {
                service,
                return_address,
            },
            None => w2m_vm::record::RunEnd::of(&out.stop.kind(), guest.called_service(), &flying),
        };
        let stem = Path::new(program)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let stop = stop_name(&out.stop);
        // the in-VM compiler's time (Sprint 247) and how it bunches (Sprint 248), when the monitor code timed its calls:
        // ticks over the counter's frequency; a timed run that compiled blocks and summed no tick is named, not
        // recorded as no time
        let compile_ms = if time_compile {
            let compiled = out.counts.get("compiled").copied().unwrap_or(0);
            if out.compile_ticks.total == 0 && compiled > 0 {
                return Err(format!(
                    "--time-compile: {compiled} blocks compiled in the VM and no tick summed"
                ));
            }
            let frequency = w2m_vm::host_state::counter_frequency()?;
            let ms = |ticks: u64| ticks as f64 * 1000.0 / frequency as f64;
            Some(w2m_vm::record::CompileMs {
                total: ms(out.compile_ticks.total),
                longest: ms(out.compile_ticks.longest),
                most_in_a_frame: ms(out.compile_ticks.most_in_a_frame),
            })
        } else {
            None
        };
        // the compiles by where their code lies (Sprint 249); a count that misses a compile is named, not recorded
        let images: Vec<(&str, u64, u64)> = guest
            .images
            .iter()
            .map(|i| (i.name.as_str(), i.base, u64::from(i.pe.size_of_image)))
            .collect();
        let compiles_by_image = w2m_vm::record::compiles_by_image(&images, out.vm_compile_starts.iter().copied())?;
        // the whole-block judge's totals, last in its file (Sprint 252)
        if let Some(j) = guest.block_judge.as_mut() {
            j.finish()?;
            eprintln!(
                "w2m-vm: the block judge: {} judged, {} equal, {} differing",
                j.judged, j.equal, j.differing
            );
        }
        let by_image: u64 = compiles_by_image.iter().map(|(_, n)| n).sum();
        let compiled = out.counts.get("compiled").copied().unwrap_or(0);
        if by_image != compiled {
            return Err(format!(
                "the compiles by image add up to {by_image}, where the in-VM compiler counted {compiled}"
            ));
        }
        let record = w2m_vm::record::execution_lines(&w2m_vm::record::Execution {
            id: &id,
            subject_sha256: &subject,
            program_stem: &stem,
            timeout_seconds: GAME_SECONDS,
            kill_grace_seconds: KILL_GRACE_SECONDS,
            executed: out.executed,
            run_seconds: out.run_ns.iter().sum::<u64>() as f64 / 1e9,
            host_seconds: out.serve_ns.iter().sum::<u64>() as f64 / 1e9,
            host_by_kind: out
                .serve_by_kind
                .iter()
                .map(|(k, ns)| (k.clone(), ns.iter().sum::<u64>() as f64 / 1e9))
                .collect(),
            compile_counts: w2m_vm::guest::VMC_COUNT_NAMES
                .iter()
                .map(|n| (*n, out.counts.get(n).copied().unwrap_or(0)))
                .collect(),
            compiles_by_image,
            compile_ms,
            exits: out.exits.iter().map(|(k, n)| (*k, *n)).collect(),
            end,
            calls: (&sha256(calls.as_bytes()), calls.lines().count() as u64),
            stop: &stop,
            elapsed: started.elapsed(),
            load_at_end: w2m_vm::host_state::load_average(),
        });
        let mut stdout = io::stdout().lock();
        for line in record {
            writeln!(stdout, "{line}").map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
