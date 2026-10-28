// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The run loop's judges. Sprint 155: `tests/data/timed_waits/qpc.exe` runs in a Rust VM with the host compiling, in
//! Rust, the blocks the VM's C compiler declines (`w2m_vm::compile`), past its first decline to a named stop; the
//! blocks the host compiled, in order, are the frozen Python host's compiles of `qpc` (`build/run_loop/qpc-python.json`,
//! made by running it, its sha256 in `store/run_loop/qpc-python.sha256`). Sprint 156: `sleeper.exe` creates a second
//! thread, which the Rust host makes as the frozen Python host does, field for field
//! (`build/run_loop/sleeper-python.json`, its sha256 in `store/run_loop/sleeper-python.sha256`), and runs on with two
//! threads to a named stop. Sprint 157: the spinner ends, and `sleeper` runs to ExitProcess with the Python host's
//! output, thread requests and hand-over (`build/run_loop/sleeper-end-python.json`, its sha256 in
//! `store/run_loop/sleeper-end-python.sha256`). The fixtures' generators are named in WORKING_AGREEMENT.
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use cpu_cases::sha256;
use guest_runtime::layout::{IPA_BASE, RUNTIME_IMAGE_VA, runtime_defines};
use hypervisor::{Vcpu, Vm, memory};
use pe_loader::{Loaded, load};
use w2m_vm::compile::HostCompiler;
use w2m_vm::guest::{Guest, vm_compiler_symbols};
use w2m_vm::process::Start;
use w2m_vm::run::{Outcome, Stop, run_compiling};
use w2m_vm::tables::page_descriptor;

/// One VM per process (Hypervisor.framework): the tests here each make one, in turn.
static ONE_VM: std::sync::Mutex<()> = std::sync::Mutex::new(());

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

/// A Python host fixture under `build/run_loop/`, checked against its sha256 under `store/run_loop/`; None when it is
/// missing (and the gates do not require it).
fn fixture(root: &Path, name: &str) -> Option<serde_json::Value> {
    let Ok(text) = fs::read_to_string(root.join(format!("build/run_loop/{name}-python.json"))) else {
        missing(&format!(
            "no Python host run of {name} (build/run_loop/{name}-python.json)"
        ));
        return None;
    };
    let pinned = fs::read_to_string(root.join(format!("store/run_loop/{name}-python.sha256"))).unwrap();
    assert_eq!(
        sha256(text.as_bytes()),
        pinned.trim(),
        "the {name} fixture is not the one committed by its sha256"
    );
    Some(serde_json::from_str(&text).unwrap())
}

/// `tests/data/timed_waits/<name>.exe` loaded and its guest built, as `tests/test_timed_waits.py` runs it.
fn guest_for(root: &Path, name: &str) -> (Loaded, Guest) {
    let data = fs::read(root.join(format!("tests/data/timed_waits/{name}.exe"))).unwrap();
    let loaded = load(&data, None, "probe.exe").unwrap();
    let (_, image) = guest_runtime::build(root, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    let services = guest_runtime::services::services(&guest_runtime::texts(root).unwrap()).unwrap();
    let exports: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("specs/windows_exports.json")).unwrap()).unwrap();
    let modules: BTreeSet<String> = exports["modules"].as_object().unwrap().keys().cloned().collect();
    let symbols =
        vm_compiler_symbols(&fs::read_to_string(root.join("tools/vm_compiler/stencil_table.h")).unwrap()).unwrap();
    let guest = Guest::new(
        &loaded,
        &image,
        &services,
        &Start::console("probe.exe"),
        &modules,
        &symbols,
    )
    .unwrap();
    (loaded, guest)
}

/// The run, in a VM of its own, with the host compiling.
fn run(guest: &mut Guest, loaded: &Loaded, host: &mut HostCompiler) -> Outcome {
    let vm = Vm::new(40).unwrap();
    vm.map(&guest.memory, IPA_BASE, memory::READ | memory::WRITE | memory::EXEC)
        .unwrap();
    let vcpu = Vcpu::new(&vm).unwrap();
    run_compiling(guest, loaded, &vcpu, 1 << 40, host).unwrap()
}

/// The Python host's compiles, (start, end) in order.
fn python_compiles(python: &serde_json::Value) -> Vec<(u64, u64)> {
    python["translations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| (t[0].as_u64().unwrap(), t[1].as_u64().unwrap()))
        .collect()
}

/// Sprint 158: the run's translations, the C's and the host's, are the Python's `vm.translations` in order: (start, end,
/// discovery), `static` for the entry.
fn translations_in_order(python: &serde_json::Value, out: &Outcome, entry: u64) {
    let want: Vec<(u64, u64, String)> = python["translations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            (
                t[0].as_u64().unwrap(),
                t[1].as_u64().unwrap(),
                t[2].as_str().unwrap().to_string(),
            )
        })
        .collect();
    let ours: Vec<(u64, u64, String)> = out
        .translations
        .iter()
        .map(|t| {
            let discovery = if t.start == entry { "static" } else { "runtime_transfer" };
            (t.start, t.end, discovery.to_string())
        })
        .collect();
    assert_eq!(ours, want, "the translations in compile order (start, end, discovery)");
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn number(v: &serde_json::Value) -> u64 {
    match v {
        serde_json::Value::String(s) => u64::from_str_radix(s.trim_start_matches("0x"), 16).unwrap(),
        _ => v.as_u64().unwrap(),
    }
}

#[test]
fn qpc_runs_past_its_first_decline_compiling_as_the_python_host() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let Some(python) = fixture(&root, "qpc") else { return };
    let python_compiles = python_compiles(&python);
    let (loaded, mut guest) = guest_for(&root, "qpc");
    let mut host = HostCompiler::default();
    let before = monotonic_count();
    let out = run(&mut guest, &loaded, &mut host);
    translations_in_order(&python, &out, loaded.entry);
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

/// Sprint 181: `model_steps.exe` reaches the model, ADDSS by hvc #10 and RCR (with its STC) by hvc #11, and the run loop
/// serves them through `model_step`, the function the case runner judges; the run ends as the frozen Python host's
/// (`build/run_loop/model-steps-python.json`): its stop, its output and its model exits. Sprint 185 widens the guest to
/// the batch run's other hvc #11 classes (POPFQ, FSIN, FCOS, XGETBV) and one #XM raised and caught by a vectored
/// handler: 301 hvc #10 and 701 hvc #11 exits.
#[test]
fn model_steps_runs_through_the_model_as_the_python_host() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let Some(python) = fixture(&root, "model-steps") else {
        return;
    };
    let (loaded, mut guest) = guest_for(&root, "model_steps");
    let mut host = HostCompiler::default();
    let out = run(&mut guest, &loaded, &mut host);
    println!("stop: {:?}", out.stop);
    println!("exits: {:?}", out.exits);
    println!("stdout: {:?}", String::from_utf8_lossy(&guest.stdout));
    assert_eq!(python["stop"], "ExitProcess");
    assert_eq!(out.stop, Stop::ExitProcess { code: 0 });
    assert_eq!(
        String::from_utf8_lossy(&guest.stdout),
        python["stdout"].as_str().unwrap(),
        "the output differs from the Python host's"
    );
    for number in [10u16, 11] {
        assert_eq!(
            out.exits.get(&number).copied().unwrap_or(0),
            python["exits"][number.to_string()].as_u64().unwrap(),
            "hvc #{number}'s count differs from the Python host's"
        );
    }
}

#[test]
fn sleeper_creates_and_ends_its_thread_as_the_python_host() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let Some(python) = fixture(&root, "sleeper") else {
        return;
    };
    let end = fixture(&root, "sleeper-end");
    let python_compiles = python_compiles(&python);
    let (loaded, mut guest) = guest_for(&root, "sleeper");
    let mut host = HostCompiler::default();
    let out = run(&mut guest, &loaded, &mut host);
    translations_in_order(&python, &out, loaded.entry);

    println!("stop: {:?}", out.stop);
    println!("blocks the C compiled: {:x?}", out.blocks.keys().collect::<Vec<_>>());
    println!("blocks the host compiled: {:x?}", host.translations);
    println!("exits: {:?}", out.exits);
    println!("threads: {:x?}", guest.threads);
    println!("stdout: {:?}", String::from_utf8_lossy(&guest.stdout));

    // the created thread, at THREAD_CREATE's return, as the Python host's
    let want = &python["created"][0];
    assert_eq!(python["created"].as_array().unwrap().len(), 1);
    assert_eq!(out.created.len(), 1, "threads created: {:?}", out.created.len());
    let created = &out.created[0];
    assert_eq!(created.tid, number(&want["tid"]), "the thread id");
    assert_eq!(
        (created.count, created.limit),
        (number(&want["count"]), number(&want["limit"])),
        "the count and the limit (`_set_budget`)"
    );
    assert_eq!(
        number(&want["entry_index"]),
        guest.threads[&created.tid].index,
        "the run table entry's index"
    );
    let entry = want["entry"].as_str().unwrap();
    for (name, from, to) in [
        ("id, status, suspend, deadline, TEB, vector slot", 0, 0x40),
        ("registers", 0x40, 0x138),
        ("vCPU mark", 0x138, 0x140),
        ("monitor words", 0x140, 0x280),
        ("state block", 0x280, 0x600),
    ] {
        assert_eq!(
            hex(&created.entry[from..to]),
            entry[2 * from..2 * to],
            "the run table entry's {name}"
        );
    }
    assert_eq!(
        hex(&created.vectors),
        want["vectors"].as_str().unwrap(),
        "the vector slot"
    );
    let t = &guest.threads[&created.tid];
    assert_eq!(t.teb, number(&want["teb"]), "the TEB");
    assert_eq!(
        [t.stack.0, t.stack.1, t.stack.2].to_vec(),
        want["stack"].as_array().unwrap().iter().map(number).collect::<Vec<_>>(),
        "the stack"
    );
    assert_eq!((t.parent, t.start), (number(&want["parent"]), number(&want["start"])));
    // `_map_pages(dealloc, limit, False)`: the reserved and guard pages unmapped in the stage-1 tables, the committed
    // top mapped
    let (dealloc, limit, top) = t.stack;
    let mapped = |va: u64| page_descriptor(guest.memory.as_slice(), va).is_some();
    for page in (dealloc..top).step_by(0x1000) {
        assert_eq!(mapped(page), page >= limit, "the stack page 0x{page:x}'s mapping");
    }
    let read = |va: u64, n: u64| {
        let at = guest.layout.offset(va).unwrap() as usize;
        guest.memory.as_slice()[at..at + n as usize].to_vec()
    };
    for (off, value) in want["teb_words"].as_object().unwrap() {
        let off = number(&serde_json::Value::String(off.clone()));
        let ours = u64::from_le_bytes(read(t.teb + off, 8).try_into().unwrap());
        assert_eq!(ours, number(value), "TEB+0x{off:x}");
    }
    for region in want["arena"].as_array().unwrap() {
        let base = number(&region["base"]);
        let r = &guest.arena.reservations[&base];
        assert_eq!(
            r.size,
            number(&region["size"]),
            "the arena region at 0x{base:x}: its size"
        );
        let state: Vec<u64> = r.state.iter().map(|&s| u64::from(s)).collect();
        let protect: Vec<u64> = r.protect.iter().map(|&p| u64::from(p)).collect();
        assert_eq!(
            state,
            region["state"]
                .as_array()
                .unwrap()
                .iter()
                .map(number)
                .collect::<Vec<_>>(),
            "the arena region at 0x{base:x}: its pages' states"
        );
        assert_eq!(
            protect,
            region["protect"]
                .as_array()
                .unwrap()
                .iter()
                .map(number)
                .collect::<Vec<_>>(),
            "the arena region at 0x{base:x}: its pages' protections"
        );
    }
    let tls = number(&want["teb_words"]["0x58"]);
    let tls_hex = want["tls"].as_str().unwrap();
    assert_eq!(hex(&read(tls, tls_hex.len() as u64 / 2)), tls_hex, "the static TLS");
    let stub = number(&want["stub_va"]);
    let stub_hex = want["stub"].as_str().unwrap();
    assert_eq!(hex(&read(stub, stub_hex.len() as u64 / 2)), stub_hex, "the attach stub");

    // Sprint 157: the spinner ends; its thread requests and the hand-over at its end are the Python host's
    if let Some(end) = &end {
        let requests: Vec<(u64, u64, u64, u64)> = end["requests"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                (
                    number(&r["sub"]),
                    number(&r["a0"]),
                    number(&r["answer"]),
                    number(&r["current"]),
                )
            })
            .collect();
        let ours: Vec<(u64, u64, u64, u64)> = out
            .thread_requests
            .iter()
            .map(|r| (r.sub, r.a0, r.answer, r.current))
            .collect();
        assert_eq!(
            ours, requests,
            "the thread requests (sub, argument, answer, current thread)"
        );
        for r in end["requests"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| number(&r["sub"]) == 2)
        {
            let (va, hex_bytes) = (number(&r["answer"]), r["stub"].as_str().unwrap());
            assert_eq!(hex(&read(va, hex_bytes.len() as u64 / 2)), hex_bytes, "the detach stub");
        }
        let handovers: Vec<(u64, u64, u64)> = end["handovers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| {
                let to = if h["ended"].as_bool().unwrap() {
                    0
                } else {
                    number(&h["to"])
                };
                (number(&h["schedule"]), number(&h["from"]), to)
            })
            .collect();
        let ours: Vec<(u64, u64, u64)> = out
            .handovers
            .iter()
            .map(|h| (u64::from(h.schedule), h.from, h.to))
            .collect();
        assert_eq!(ours, handovers, "the hand-overs (schedule bits, from, to)");
        assert_eq!(
            out.stop,
            Stop::ExitProcess {
                code: number(&end["exit_code"]) as u32
            },
            "the run's stop"
        );
        assert_eq!(python["stop"], "ExitProcess");
        assert_eq!(
            String::from_utf8_lossy(&guest.stdout),
            python["stdout"].as_str().unwrap(),
            "the run's output"
        );
    }

    // the run's output (so far, without the second fixture) is the Python run's, and every block compiled is a Python
    // host compile
    assert!(
        !matches!(out.stop, Stop::Exit(_)),
        "the run stopped at an exit it does not name: {:?}",
        out.stop
    );
    assert!(
        python["stdout"].as_str().unwrap().as_bytes().starts_with(&guest.stdout),
        "the output so far is not the Python run's"
    );
    let python_starts: BTreeSet<u64> = python_compiles.iter().map(|(s, _)| *s).collect();
    for start in out.blocks.keys().chain(host.translations.iter().map(|(s, _)| s)) {
        assert!(
            python_starts.contains(start),
            "0x{start:x} compiled, which the Python host did not compile"
        );
    }
    for compile in &host.translations {
        assert!(
            python_compiles.contains(compile),
            "the host compiled {compile:x?}, which the Python did not"
        );
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
