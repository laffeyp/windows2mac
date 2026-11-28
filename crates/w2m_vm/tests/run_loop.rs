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
use std::time::{Duration, Instant};

use cpu_cases::sha256;
use guest_runtime::layout::{
    AP_READ_ONLY, CACHE_CODE_VA, CACHE_FILE_SIZE, CACHE_FILE_VA, CODE_CACHE, CODE_CACHE_SIZE, DISPATCH_TABLE, IPA_BASE,
    LOCK_OWNER, MONITOR_DATA, MONITOR_VA, PAGE, PROCESS_VA, RECON_COUNT, RECON_TABLE, RUN_TABLE, RUNTIME_IMAGE_VA,
    STATE_OFF, VCPU_AREA, VCPU_STRIDE, VM_COMPILER, VMC_LOG, runtime_defines,
};
use hypervisor::{Vcpu, Vm, memory};
use pe_loader::{Loaded, load};
use translator::lower::RuntimeCalls;
use w2m_vm::cache_file::{self, Exits};
use w2m_vm::compile::HostCompiler;
use w2m_vm::guest::{Guest, vm_compiler_symbols};
use w2m_vm::monitor_code::{RUNTIME_LOCK, VMC_INSTALLER, VMC_LOG_COUNT};
use w2m_vm::process::Start;
use w2m_vm::run::{JUDGE_KIND, Outcome, Stop, run_compiling};
use w2m_vm::tables::{block_descriptor, page_descriptor};

/// One VM per process (Hypervisor.framework): the tests here each make one, in turn.
use crate::ONE_VM; // one VM at a time across the binary (Sprint 306)

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
    let guest = guest_with(root, &loaded, &Start::console("probe.exe"));
    (loaded, guest)
}

/// The guest of `loaded` as `start` asks.
fn guest_with(root: &Path, loaded: &Loaded, start: &Start) -> Guest {
    guest_with_defines(root, loaded, start, &runtime_defines())
}

/// `guest_with`, its runtime image built with `defines` (Sprint 359: the timed image's, `runtime_defines_for(true)`).
fn guest_with_defines(root: &Path, loaded: &Loaded, start: &Start, defines: &[(String, u64)]) -> Guest {
    let (_, image) = guest_runtime::build(root, defines, RUNTIME_IMAGE_VA).unwrap();
    let services = guest_runtime::services::services(&guest_runtime::texts(root).unwrap()).unwrap();
    let exports: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("specs/windows_exports.json")).unwrap()).unwrap();
    let modules: BTreeSet<String> = exports["modules"].as_object().unwrap().keys().cloned().collect();
    let symbols =
        vm_compiler_symbols(&fs::read_to_string(root.join("tools/vm_compiler/stencil_table.h")).unwrap()).unwrap();
    Guest::new(loaded, &image, &services, start, &modules, &symbols).unwrap()
}

/// `tests/data/timed_waits/<name>.exe` with the translation cache of its own image in use (Sprint 257): the miss stub
/// sends the image's range to the host, and the cache is made from the image's bytes under the guest's setup, as
/// `w2m-vm --make-cache` makes UnityPlayer.dll's. With `in_vm`, the installer inside the VM installs first (Sprint 263).
fn cached_guest(root: &Path, name: &str, in_vm: bool) -> (Loaded, Guest, HostCompiler) {
    cached_guest_with(root, name, in_vm, false)
}

/// `cached_guest`, the instructions at each in-VM compile and install written when `instructions_at_compile` (Sprint
/// 266).
fn cached_guest_with(
    root: &Path,
    name: &str,
    in_vm: bool,
    instructions_at_compile: bool,
) -> (Loaded, Guest, HostCompiler) {
    let data = fs::read(root.join(format!("tests/data/timed_waits/{name}.exe"))).unwrap();
    let loaded = load(&data, None, "probe.exe").unwrap();
    let program = &loaded.images[0];
    let mut start = Start::console("probe.exe");
    start.host_range = Some((program.base, program.base + u64::from(program.pe.size_of_image)));
    start.install_in_vm = in_vm;
    start.instructions_at_compile = instructions_at_compile;
    let mut guest = guest_with(root, &loaded, &start);
    let exits = Exits {
        exit_va: guest.code.dispatch_va,
        slice_va: guest.slice_va,
        state_va: PROCESS_VA + STATE_OFF,
        runtime: RuntimeCalls::from_symbols(|s| guest.symbols.get(s).copied()),
    };
    let key = format!("{name:0>64}"); // a key's width, 64
    let (file, _) = cache_file::make(&program.pe, &data, program.base, &exits, key);
    assert!(!file.blocks.is_empty(), "{name}'s cache holds no block");
    guest.place_cache_file(&cache_file::to_bytes(&file)).unwrap();
    let mut host = HostCompiler::default();
    host.use_cache(&mut guest, file).unwrap();
    (loaded, guest, host)
}

/// The reconstruction table's marks, (host, x86), as the C runtime reads them.
fn recon_marks(guest: &Guest) -> Vec<(u64, u64)> {
    let mem = guest.memory.as_slice();
    let word = |at: u64| u64::from_le_bytes(mem[at as usize..at as usize + 8].try_into().unwrap());
    (0..word(RUN_TABLE + RECON_COUNT))
        .map(|i| (word(RECON_TABLE + 16 * i), word(RECON_TABLE + 16 * i + 8)))
        .collect()
}

/// The run, in a VM of its own, with the host compiling.
fn run(guest: &mut Guest, loaded: &Loaded, host: &mut HostCompiler) -> Outcome {
    run_limited(guest, loaded, host, None)
}

/// `run`, stopped at `limit` (Sprint 326: the run makes its vCPU and the limit's timer).
fn run_limited(guest: &mut Guest, loaded: &Loaded, host: &mut HostCompiler, limit: Option<Duration>) -> Outcome {
    let vm = Vm::new(40).unwrap();
    vm.map(&guest.memory, IPA_BASE, memory::READ | memory::WRITE | memory::EXEC)
        .unwrap();
    run_compiling(guest, loaded, &vm, limit, 1 << 40, host).unwrap()
}

/// Sprint 326: the run's time limit stops its vCPU, the timer started by the run itself; a limit of 0 s stops `qpc`
/// before it ends (a cancel before the vCPU's first run returns at once, `hv_vcpu.h`), and no limit lets it end. No
/// upper bound on the wall clock is held (`no_wall_clock_bound`).
#[test]
fn the_runs_time_limit_stops_its_vcpu() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let (loaded, mut guest) = guest_for(&root, "qpc");
    let out = run_limited(&mut guest, &loaded, &mut HostCompiler::default(), Some(Duration::ZERO));
    assert_eq!(out.stop, Stop::TimeLimit, "qpc with a limit of 0 s");
    let (loaded, mut guest) = guest_for(&root, "qpc");
    let out = run_limited(&mut guest, &loaded, &mut HostCompiler::default(), None);
    assert_eq!(out.stop, Stop::ExitProcess { code: 0 }, "qpc with no limit");
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
    println!("stdout: {:?}", String::from_utf8_lossy(&guest.served.stdout));

    // `tests/test_timed_waits.py` 218-223, carried: the process ends, and its counter lies between the host's monotonic
    // count before the run and after it
    assert_eq!(out.stop, Stop::ExitProcess { code: 0 });
    let text = String::from_utf8(guest.served.stdout.clone()).unwrap();
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
    // Sprint 286: every miss the host served, its compiles among them, ran under the runtime word, taken by the miss
    // service or held by the vCPU (plant 1 takes none)
    assert_eq!(out.served_outside_word, 0, "services served outside the runtime word");
    // and each host compile's words, at the same code address, are the Python host's words for that block
    for (start, va) in &host.blocks {
        let want = &python["blocks"][format!("0x{start:x}")];
        assert_eq!(
            want[0].as_str().unwrap(),
            format!("0x{va:x}"),
            "block 0x{start:x}'s code address"
        );
        let hex = want[1].as_str().unwrap();
        let at = guest.offset(*va).unwrap() as usize;
        let ours: String = guest.memory.as_slice()[at..at + hex.len() / 2]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(ours, hex, "block 0x{start:x}'s words differ from the Python host's");
    }
}

/// Sprint 257: `qpc.exe` with the cache of its own image runs to the same stop and output as without, the host
/// installing its blocks; each installed block's marks open with its code and its start and lie within its words, so the
/// C runtime's lookups find the block where it was installed.
#[test]
fn qpc_runs_with_its_blocks_installed_from_the_cache() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let (loaded, mut guest, mut host) = cached_guest(&root, "qpc", false);
    let before = monotonic_count();
    let out = run(&mut guest, &loaded, &mut host);
    let after = monotonic_count();
    assert_eq!(out.stop, Stop::ExitProcess { code: 0 });
    let counter = u64::from_str_radix(String::from_utf8(guest.served.stdout.clone()).unwrap().trim_end(), 16).unwrap();
    assert!(
        before <= counter && counter <= after,
        "{before} <= {counter} <= {after}"
    );
    let cache = host.cache.as_ref().unwrap();
    assert!(cache.installs > 0, "the host installed nothing");
    assert_eq!(cache.stopped_at, None, "qpc writes no code");
    let compiled: BTreeSet<u64> = host.translations.iter().map(|(s, _)| *s).collect();
    let marks = recon_marks(&guest);
    let mut installed = 0;
    for (&start, &va) in host.blocks.iter().filter(|(s, _)| !compiled.contains(s)) {
        let b = cache
            .file
            .blocks
            .iter()
            .find(|b| cache.file.base + u64::from(b.start) == start)
            .unwrap();
        let first = marks
            .iter()
            .position(|&m| m == (va, start))
            .unwrap_or_else(|| panic!("no mark (0x{va:x}, 0x{start:x}): block 0x{start:x}'s marks are not rebased"));
        let end = va + 4 * u64::from(b.nwords);
        for &(h, x) in &marks[first..first + b.nmarks as usize] {
            assert!(
                (va..=end).contains(&h),
                "block 0x{start:x}'s mark (0x{h:x}, 0x{x:x}) lies outside its words"
            );
        }
        installed += 1;
    }
    assert_eq!(installed, cache.installs, "the installs counted");
}

/// Sprint 266 (card 5's gate on the guest's own clock): with the instructions at each call written, every in-VM compile
/// (`qpc.exe` without the cache, the C compiling its blocks) and every install (with it, the installer installing them)
/// carries a count, in the log's order never falling, none past the run's and none 0 but the first, the entry's,
/// compiled before any instruction ran; without, every word is 0, as the C leaves it. The limit is absolute,
/// `executed + TIME_SLICE` with threads (`threads::set_budget`), so the count is the run's and not the slice's.
#[test]
fn each_compile_and_install_carries_the_instructions_retired_at_it() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let uncached = |counted: bool| {
        let data = fs::read(root.join("tests/data/timed_waits/qpc.exe")).unwrap();
        let loaded = load(&data, None, "probe.exe").unwrap();
        let mut start = Start::console("probe.exe");
        start.instructions_at_compile = counted;
        let guest = guest_with(&root, &loaded, &start);
        (loaded, guest, HostCompiler::default())
    };
    for (counted, cached) in [(true, false), (true, true), (false, false)] {
        let (loaded, mut guest, mut host) = if cached {
            cached_guest_with(&root, "qpc", true, counted)
        } else {
            uncached(counted)
        };
        let out = run(&mut guest, &loaded, &mut host);
        assert_eq!(out.stop, Stop::ExitProcess { code: 0 });
        let at = &out.instructions_at;
        let (what, counts) = if cached {
            ("install", &at.installs)
        } else {
            ("compile", &at.compiles)
        };
        assert!(!counts.is_empty(), "no {what} inside the VM");
        if !counted {
            assert!(
                counts.iter().all(|&c| c == 0),
                "an uncounted run wrote a {what}'s count"
            );
            continue;
        }
        for (i, &c) in counts.iter().enumerate() {
            assert!(
                (i == 0 || 0 < c) && c <= out.executed,
                "{what} {i}: {c} instructions, the run's {}",
                out.executed
            );
        }
        assert!(
            counts.windows(2).all(|w| w[0] <= w[1]),
            "the {what}s' counts fall in the log's order"
        );
    }
}

/// Sprint 257 (reviewer-3's Medium 1 at 257): the whole-block judge's host time is its own kind, not the exit's it
/// follows; the kinds still add up to the run's host time, and an exit's kind holds only its service.
#[test]
fn the_judges_time_is_counted_apart_from_the_exits() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let (loaded, mut guest) = guest_for(&root, "qpc");
    let file = std::env::temp_dir().join(format!("w2m-judge-time-{}.tsv", std::process::id()));
    guest.block_judge = Some(w2m_vm::block_judge::BlockJudge::create(&file).unwrap());
    let mut host = HostCompiler::default();
    let out = run(&mut guest, &loaded, &mut host);
    let _ = fs::remove_file(&file);
    assert_eq!(out.stop, Stop::ExitProcess { code: 0 });
    let judged = guest.block_judge.as_ref().unwrap().judged;
    assert!(judged > 0, "the judge judged nothing");
    let judge: u64 = out.serve_by_kind.get(JUDGE_KIND).map_or(0, |v| v.iter().sum());
    assert!(
        judge > 0,
        "{judged} blocks judged and no host time counted as the judge's"
    );
    let by_kind: u64 = out.serve_by_kind.values().flatten().sum();
    assert_eq!(
        by_kind,
        out.serve_ns.iter().sum::<u64>(),
        "the kinds add up to the host's time"
    );
}

/// Sprint 261 (card 5's I): a guest built for a cache holds the file's bytes at `CACHE_FILE_VA`, its region's blocks
/// read-only; a guest built without one has no such region and its maps are as before.
#[test]
fn a_cached_guest_holds_the_file_read_only_at_its_region() {
    let root = root();
    let (_, guest, host) = cached_guest(&root, "qpc", false);
    let bytes = cache_file::to_bytes(&host.cache.as_ref().unwrap().file);
    let at = guest.offset(CACHE_FILE_VA).expect("no map at CACHE_FILE_VA") as usize;
    assert_eq!(
        &guest.memory.as_slice()[at..at + bytes.len()],
        bytes.as_slice(),
        "the file's bytes"
    );
    assert!(
        IPA_BASE + guest.layout.size <= 1 << 40,
        "the guest's memory, 0x{:x}, past the VM's 40-bit IPA (`Vm::new(40)`)",
        guest.layout.size
    );
    for va in (CACHE_FILE_VA..CACHE_FILE_VA + CACHE_FILE_SIZE).step_by(0x20_0000) {
        let d = block_descriptor(guest.memory.as_slice(), va).unwrap_or_else(|| panic!("no block at 0x{va:x}"));
        let desc = u64::from_le_bytes(guest.memory.as_slice()[d as usize..d as usize + 8].try_into().unwrap());
        assert_ne!(desc & AP_READ_ONLY, 0, "the block at 0x{va:x} is writable");
    }
    let (_, plain) = guest_for(&root, "qpc");
    assert_eq!(plain.layout.cache_file_off, None);
    assert_eq!(
        plain.offset(CACHE_FILE_VA),
        None,
        "a guest without a cache maps the region"
    );
}

/// The installer inside the VM called once, alone, with `target` in x0, in a VM of its own over the guest's memory: its
/// x0, the code or 0. Its return lands on an `hvc #0x55` at the code area's last word.
fn call_installer(guest: &mut Guest, target: u64) -> u64 {
    use hypervisor::{Exit, Reg};
    let mem = guest.memory.as_mut_slice();
    let read = |mem: &[u8], at: u64| u64::from_le_bytes(mem[at as usize..at as usize + 8].try_into().unwrap());
    let installer = read(mem, VM_COMPILER + VMC_INSTALLER);
    assert_ne!(installer, 0, "the guest has no installer");
    let back = CODE_CACHE + CODE_CACHE_SIZE - 4;
    mem[back as usize..back as usize + 4].copy_from_slice(&a64_encoder::hvc(0x55).to_le_bytes());
    let vm = Vm::new(40).unwrap();
    vm.map(&guest.memory, IPA_BASE, memory::READ | memory::WRITE | memory::EXEC)
        .unwrap();
    let vcpu = Vcpu::new(&vm).unwrap();
    w2m_vm::monitor::set_system_registers(&vcpu, guest.ttbr).unwrap();
    for (r, v) in [(0, target), (28, MONITOR_VA + MONITOR_DATA), (30, MONITOR_VA + back)] {
        vcpu.set_reg(Reg::X(r), v).unwrap();
    }
    vcpu.set_reg(Reg::Cpsr, w2m_vm::CPSR_EL1H).unwrap();
    vcpu.set_reg(Reg::Pc, installer).unwrap();
    loop {
        match vcpu.run().unwrap() {
            Exit::VtimerActivated => vcpu.set_vtimer_mask(true).unwrap(),
            e => {
                assert_eq!(e.hvc_immediate(), Some(0x55), "the installer stopped otherwise: {e:?}");
                return vcpu.reg(Reg::X(0)).unwrap();
            }
        }
    }
}

/// Sprint 263 (card 5's III, V, VII; reviewer-3's Low 2 at 261): the installer inside the VM installs a cached start,
/// its words those `relocate` gives at the code, its marks rebased there, its log entry marked an install and its
/// dispatch pair written; and declines a start below the image, before its first block and after its last.
#[test]
fn the_installer_installs_a_cached_start_and_declines_the_rest() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let (_, mut guest, host) = cached_guest(&root, "qpc", true);
    let file = host.cache.as_ref().unwrap().file.clone();
    let b = *file.blocks.iter().max_by_key(|b| b.nfixups).unwrap();
    assert!(b.nfixups > 0, "qpc's cache holds no block with a fix-up");
    let start = file.base + u64::from(b.start);
    let read = |m: &[u8], at: u64| u64::from_le_bytes(m[at as usize..at as usize + 8].try_into().unwrap());
    // the guest's memory is sparse and over 513 GiB: read in place, never copied
    let n = read(guest.memory.as_slice(), RUN_TABLE + RECON_COUNT);
    let code = call_installer(&mut guest, start);
    assert_ne!(code, 0, "the cached start 0x{start:x} declined");
    let after = guest.memory.as_slice();
    // the words, as `relocate` moves them
    let from = CACHE_CODE_VA + 4 * u64::from(b.word);
    let want = cache_file::relocate(&file.words[b.word as usize..(b.word + b.nwords) as usize], from, code).unwrap();
    let at = guest.offset(code).unwrap() as usize;
    let got: Vec<u32> = (0..want.len())
        .map(|k| u32::from_le_bytes(after[at + 4 * k..at + 4 * k + 4].try_into().unwrap()))
        .collect();
    for k in 0..want.len() {
        assert_eq!(got[k], want[k], "block 0x{start:x}'s word {k}");
    }
    // the marks, from the count before
    assert_eq!(read(after, RUN_TABLE + RECON_COUNT), n + u64::from(b.nmarks));
    for j in 0..u64::from(b.nmarks) {
        let (h, x) = file.marks[(u64::from(b.mark) + j) as usize];
        let entry = RECON_TABLE + 16 * (n + j);
        assert_eq!(
            (read(after, entry), read(after, entry + 8)),
            (u64::from(h) + code - 4 * u64::from(b.word), file.base + u64::from(x)),
            "block 0x{start:x}'s mark {j}"
        );
    }
    // the log entry, an install, and the dispatch pair
    let log = VM_COMPILER + VMC_LOG;
    assert_eq!(read(after, VM_COMPILER + VMC_LOG_COUNT), 1);
    assert_eq!((read(after, log), read(after, log + 16)), (start, code));
    assert_eq!(
        &after[(log + 52) as usize..(log + 56) as usize],
        &1u32.to_le_bytes(),
        "pad"
    );
    let slot = DISPATCH_TABLE + 16 * w2m_vm::monitor_code::dispatch_slot(start);
    assert!(
        [slot, slot + 16]
            .iter()
            .any(|s| (read(after, *s), read(after, s + 8)) == (start, code)),
        "no dispatch pair (0x{start:x}, 0x{code:x})"
    );
    // the block the host opens the call trace at, left to the host as the C leaves it
    let host_only = VM_COMPILER + w2m_vm::monitor_code::VMC_HOST_ONLY;
    let other = file.base + u64::from(file.blocks[0].start);
    let mem = guest.memory.as_mut_slice();
    mem[host_only as usize..host_only as usize + 8].copy_from_slice(&other.to_le_bytes());
    assert_eq!(
        call_installer(&mut guest, other),
        0,
        "the host's block 0x{other:x} installed"
    );
    // the misses: below the image, before the first block, after the last
    let (first, last) = (file.blocks[0].start, file.blocks.last().unwrap().start);
    for (what, target) in [
        ("below the image", file.base - 1),
        ("before the first block", file.base + u64::from(first) - 1),
        ("after the last block", file.base + u64::from(last) + 1),
    ] {
        assert_eq!(call_installer(&mut guest, target), 0, "{what}, 0x{target:x}, installed");
    }
}

/// Sprint 263 (reviewer-3's Low 2 at 263): the exits a cached block's words embed, the dispatcher and the slice stub,
/// lie at the same addresses whether the installer is placed or not, so the cache made without it serves a run with it.
#[test]
fn the_installer_moves_no_exit_a_cached_block_embeds() {
    let root = root();
    let (_, plain, _) = cached_guest(&root, "qpc", false);
    let (_, with, _) = cached_guest(&root, "qpc", true);
    assert_eq!(with.code.dispatch_va, plain.code.dispatch_va, "the dispatcher");
    assert_eq!(with.slice_va, plain.slice_va, "the slice stub");
    let read = |g: &Guest| {
        let at = (VM_COMPILER + VMC_INSTALLER) as usize;
        u64::from_le_bytes(g.memory.as_slice()[at..at + 8].try_into().unwrap())
    };
    assert_eq!(read(&plain), 0, "an installer without install_in_vm");
    assert!(read(&with) > with.slice_va, "the installer before the slice stub");
}

/// Sprint 263 (card 5's VI, VII): `qpc` runs with its blocks installed inside the VM to the stop, the output and the
/// instruction count of its run with the host's install, with fewer `hvc 1` exits; a write to the image's code then
/// stops the installer.
#[test]
fn qpc_runs_with_its_blocks_installed_inside_the_vm() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let (loaded, mut by_host, mut host_install) = cached_guest(&root, "qpc", false);
    let hosts = run(&mut by_host, &loaded, &mut host_install);
    let (loaded, mut guest, mut host) = cached_guest(&root, "qpc", true);
    let out = run(&mut guest, &loaded, &mut host);
    assert_eq!(out.stop, Stop::ExitProcess { code: 0 });
    let cache = host.cache.as_ref().unwrap();
    assert!(cache.installs > 0, "nothing installed");
    assert_eq!(
        cache.installs,
        host_install.cache.as_ref().unwrap().installs,
        "the installs"
    );
    assert_eq!(out.executed, hosts.executed, "the instructions retired");
    assert!(
        out.exits.get(&1) < hosts.exits.get(&1),
        "hvc 1 exits {:?}, the host's install {:?}",
        out.exits.get(&1),
        hosts.exits.get(&1)
    );
    // the write rule: the stop word set, the installer declining after it
    let (_, mut guest, mut host) = cached_guest(&root, "qpc", true);
    let file = host.cache.as_ref().unwrap().file.clone();
    let start = file.base + u64::from(file.blocks[0].start);
    assert!(
        host.code_written(&mut guest, start, &mut || Ok(())).unwrap(),
        "the write was not taken"
    );
    assert_eq!(
        call_installer(&mut guest, start),
        0,
        "the installer installed after the write"
    );
}

/// Sprint 257, the write rule: a write into the image's code stops the cache for the rest of the run, drops every block
/// installed from it, makes the image's pages no block is behind writable again, and installs nothing after.
#[test]
fn a_write_to_the_image_code_stops_the_cache() {
    let root = root();
    let (loaded, mut guest, mut host) = cached_guest(&root, "qpc", false);
    let entry = loaded.image_entry;
    let va = host
        .install(&mut guest, entry, &mut || Ok(()))
        .expect("the cache holds the entry's block")
        .unwrap();
    assert_eq!(host.blocks.get(&entry), Some(&va));
    let other = {
        let file = &host.cache.as_ref().unwrap().file;
        file.blocks
            .iter()
            .map(|b| file.base + u64::from(b.start))
            .find(|&s| s != entry)
            .unwrap()
    };
    // a write to the image's first code page with no block behind it: not one of the entry block's pages
    let entry_pages: BTreeSet<u64> = host.code_pages.keys().copied().collect();
    let program = &loaded.images[0];
    let text = program
        .pe
        .sections
        .iter()
        .find(|s| s.characteristics & pe_image::SCN_MEM_EXECUTE != 0)
        .unwrap();
    let written = (0..u64::from(text.virtual_size))
        .step_by(PAGE as usize)
        .map(|o| (program.base + u64::from(text.virtual_address) + o) & !(PAGE - 1))
        .find(|p| !entry_pages.contains(p));
    let written = written.unwrap_or_else(|| *entry_pages.iter().next().unwrap()) + 8;
    assert!(
        host.code_written(&mut guest, written, &mut || Ok(())).unwrap(),
        "the write to 0x{written:x} was not taken"
    );
    let cache = host.cache.as_ref().unwrap();
    assert_eq!(cache.stopped_at, Some(written), "the cache did not stop");
    assert!(!host.blocks.contains_key(&entry), "the installed block was not dropped");
    assert!(
        host.install(&mut guest, other, &mut || Ok(())).is_none(),
        "the cache installed after the write"
    );
    let at = page_descriptor(guest.memory.as_slice(), written & !(PAGE - 1)).unwrap();
    let desc = u64::from_le_bytes(
        guest.memory.as_slice()[at as usize..at as usize + 8]
            .try_into()
            .unwrap(),
    );
    assert!(
        entry_pages.contains(&(written & !(PAGE - 1))) || desc & AP_READ_ONLY == 0,
        "the written page 0x{written:x} is still read-only"
    );
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
    println!("stdout: {:?}", String::from_utf8_lossy(&guest.served.stdout));
    assert_eq!(python["stop"], "ExitProcess");
    assert_eq!(out.stop, Stop::ExitProcess { code: 0 });
    assert_eq!(
        String::from_utf8_lossy(&guest.served.stdout),
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

/// A block split in the Arm order (Sprint 305; the vCPU reports BBM 0, Sprint 303): when its flush runs, the block's
/// descriptor is already invalid and not yet the table; after it, the table. The flush here reads the descriptor
/// through guest memory's address, taken before `page_descriptor` borrows the guest.
#[test]
fn a_split_makes_the_block_invalid_before_its_flush_and_the_table_after() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let (_, mut guest) = guest_for(&root, "sleeper");
    // the first valid 2 MB block the tables hold, found by walking them (levels 0 and 1 tables, level 2 a block)
    let mem = guest.memory.as_slice();
    let read = |off: u64| u64::from_le_bytes(mem[off as usize..off as usize + 8].try_into().unwrap());
    let next = |desc: u64| (desc & 0x0000_FFFF_FFFF_F000) - IPA_BASE;
    let mut found = None;
    'walk: for i0 in 0..512u64 {
        let d0 = read(guest_runtime::layout::TRANSLATION_TABLES + 8 * i0);
        if d0 & 3 != 3 {
            continue;
        }
        for i1 in 0..512u64 {
            let d1 = read(next(d0) + 8 * i1);
            if d1 & 3 != 3 {
                continue;
            }
            for i2 in 0..512u64 {
                if read(next(d1) + 8 * i2) & 3 == 1 {
                    found = Some((i0 << 39) | (i1 << 30) | (i2 << 21));
                    break 'walk;
                }
            }
        }
    }
    let va = found.expect("a valid 2 MB block in the tables");
    let at = block_descriptor(mem, va).expect("its level-2 descriptor");
    let before = u64::from_le_bytes(
        guest.memory.as_slice()[at as usize..at as usize + 8]
            .try_into()
            .unwrap(),
    );
    let base = guest.memory.as_slice().as_ptr() as usize;
    let mut seen = Vec::new();
    let mut flush = || {
        // SAFETY: the mapping outlives the call, and the host alone writes it, with no vCPU running
        seen.push(unsafe { std::ptr::read_volatile((base + at as usize) as *const u64) });
        Ok(())
    };
    guest.page_descriptor(va, &mut flush).unwrap();
    let after = u64::from_le_bytes(
        guest.memory.as_slice()[at as usize..at as usize + 8]
            .try_into()
            .unwrap(),
    );
    assert_eq!(seen.len(), 1, "one flush a split");
    assert_eq!(
        seen[0],
        before & !1,
        "at the flush the block is invalid, its other bits kept: 0x{:x}",
        seen[0]
    );
    assert_eq!(after & 3, 3, "after the flush, a table: 0x{after:x}");
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
    println!("threads: {:x?}", guest.served.threads);
    println!("stdout: {:?}", String::from_utf8_lossy(&guest.served.stdout));
    println!("blocks split: {}, flushes: {}", guest.served.splits, out.flushes);

    // each block a request split was followed by the flush stub before the vCPU ran on (Sprint 304)
    assert_eq!(
        out.flushes, guest.served.splits,
        "a split without its flush: the vCPU ran on with every TLB's block entry"
    );

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
        guest.served.threads[&created.tid].index,
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
    let t = &guest.served.threads[&created.tid];
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
        let at = guest.offset(va).unwrap() as usize;
        guest.memory.as_slice()[at..at + n as usize].to_vec()
    };
    for (off, value) in want["teb_words"].as_object().unwrap() {
        let off = number(&serde_json::Value::String(off.clone()));
        let ours = u64::from_le_bytes(read(t.teb + off, 8).try_into().unwrap());
        assert_eq!(ours, number(value), "TEB+0x{off:x}");
    }
    for region in want["arena"].as_array().unwrap() {
        let base = number(&region["base"]);
        let r = &guest.served.arena.reservations[&base];
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
        // each hand-over's `hvc #12`, and the misses, served under the runtime word (Sprint 294)
        assert_eq!(out.served_outside_word, 0, "services served outside the runtime word");
        assert_eq!(
            out.stop,
            Stop::ExitProcess {
                code: number(&end["exit_code"]) as u32
            },
            "the run's stop"
        );
        assert_eq!(python["stop"], "ExitProcess");
        assert_eq!(
            String::from_utf8_lossy(&guest.served.stdout),
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
        python["stdout"]
            .as_str()
            .unwrap()
            .as_bytes()
            .starts_with(&guest.served.stdout),
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

/// Sprint 364 (card 4f3; until then Sprint 325's refusal of a spawn on several vCPUs): on a start of 2 vCPUs `twoproc`'s
/// CreateProcessW makes the request server's link, not the turns', and its child's end is the server's to see. In
/// process the child `w2m-vm` is this test binary, which refuses `--child` and ends without a word, while `twoproc`
/// polls its handle with no end of its own: the server's look finds the child gone, and its stop reaches the run through
/// the first process's link thread, naming the child, as the server's stops do.
#[test]
fn a_spawn_on_two_vcpus_makes_the_request_servers_link() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let data = fs::read(root.join("tests/data/timed_waits/twoproc.exe")).unwrap();
    let loaded = load(&data, None, "probe.exe").unwrap();
    let mut start = Start::console("probe.exe");
    start.vcpus = 2;
    let mut guest = guest_with(&root, &loaded, &start);
    let out = run_limited(
        &mut guest,
        &loaded,
        &mut HostCompiler::default(),
        Some(Duration::from_secs(60)),
    );
    match &out.stop {
        Stop::Handover { why, .. } => assert!(
            why.starts_with("child process ") && why.contains(": gone without a word"),
            "twoproc on 2 vCPUs stopped as {why}"
        ),
        other => panic!("twoproc on 2 vCPUs stopped as {other:?}"),
    }
    assert!(
        guest.kernel.served_by_the_request_server() && !guest.kernel.turns(),
        "the link made by a spawn on 2 vCPUs: request server's {}, turns {}",
        guest.kernel.served_by_the_request_server(),
        guest.kernel.turns()
    );
}

/// Sprint 327 (the run-loop design's 4c3b2b): two vCPUs run one process and end it as one vCPU does, `qpc`'s clock
/// reading held by its form as `batch_run.rs` holds it; vCPU 1's `LOCK_OWNER`, at its own monitor page
/// (`VCPU_AREA + VCPU_STRIDE`, `Guest::private_chain`), is 2; and `twoready`'s vCPU 1 runs guest code, its main thread
/// spinning on its vCPU while it readies two workers, so a waiting vCPU must take one (`twoready.c` 4-5). The run's
/// limit of 60 s turns a vCPU left parked into a named stop, as the gates' GUEST_SECONDS does; no elapsed time is held.
#[test]
fn two_vcpus_end_each_guest_as_one_does() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    for name in ["qpc", "sleeper", "twoready"] {
        let (loaded, mut one) = guest_for(&root, name);
        let alone = run(&mut one, &loaded, &mut HostCompiler::default());
        // each guest ends on one vCPU, so a run that fails alike on one and two cannot pass by matching (Sprint 328:
        // `statsize`, which in-process has no file bridge and stops at its stat, so matched)
        assert_eq!(alone.stop, Stop::ExitProcess { code: 0 }, "{name} on one vCPU");
        let mut start = Start::console("probe.exe");
        start.vcpus = 2;
        let mut two = guest_with(&root, &loaded, &start);
        let both = run_limited(
            &mut two,
            &loaded,
            &mut HostCompiler::default(),
            Some(Duration::from_secs(60)),
        );
        // read first, so a run whose owners are wrong fails here, whatever its stop
        let owner = two.memory_view().read_u64(VCPU_AREA + VCPU_STRIDE + LOCK_OWNER);
        assert_eq!(owner, 2, "{name}: vCPU 1's LOCK_OWNER");
        assert_eq!(both.stop, alone.stop, "{name}'s stop on 2 vCPUs");
        let (said, said_alone) = (
            String::from_utf8_lossy(&two.served.stdout),
            String::from_utf8_lossy(&one.served.stdout),
        );
        if name == "qpc" {
            let reading = said.strip_suffix('\n').unwrap_or("");
            assert!(
                reading.len() == 16 && reading.chars().all(|c| c.is_ascii_hexdigit()),
                "qpc's output on 2 vCPUs: {said:?}"
            );
        } else {
            assert_eq!(said, said_alone, "{name}'s output on 2 vCPUs");
        }
        assert_eq!(both.runs_by_vcpu.len(), 2, "{name}: a count of runs for each vCPU");
        if name == "twoready" {
            assert!(
                both.runs_by_vcpu[1] > 0,
                "twoready: vCPU 1 ran no guest code, runs {:?}",
                both.runs_by_vcpu
            );
        }
    }
}

/// Sprint 338 (Change 1's first card, the runtime word's take behind one guard): no served exit leaves the runtime word
/// held. `sleeper` runs on one vCPU and on two to its ExitProcess, and the run table's word (`RUNTIME_LOCK`) is read
/// free after the stop, held neither by the host nor by a guest; the services the run reaches are held by their counts:
/// every turn's start and every exit's diagnostics take the word, and so do `hvc #1` and `hvc #12`, and vCPU 1 takes it
/// for a thread. A word the host leaves held makes the next take wait in the host for ever, which the run's limit does
/// not end (it stops guest code), so the run is on a thread of its own and one not ended in 120 s fails by name, the
/// limit a hang's stop as the run's 60 s is; no elapsed time is held.
#[test]
fn the_runtime_word_is_free_after_each_served_exit() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    for vcpus in [1, 2] {
        let (done, ended) = std::sync::mpsc::channel();
        let running = std::thread::spawn(move || {
            let root = root();
            let data = fs::read(root.join("tests/data/timed_waits/sleeper.exe")).unwrap();
            let loaded = load(&data, None, "probe.exe").unwrap();
            let mut start = Start::console("probe.exe");
            start.vcpus = vcpus;
            let mut guest = guest_with(&root, &loaded, &start);
            let limit = Some(Duration::from_secs(60));
            let out = run_limited(&mut guest, &loaded, &mut HostCompiler::default(), limit);
            let word = guest.memory_view().read_u32(RUN_TABLE + RUNTIME_LOCK);
            let _ = done.send((out, word));
        });
        let (out, word) = match ended.recv_timeout(Duration::from_secs(120)) {
            Ok(ran) => ran,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                panic!("sleeper on {vcpus} vCPUs did not end in 120 s: a take waits on a runtime word left held")
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                std::panic::resume_unwind(running.join().expect_err("the run's thread ended with no outcome"))
            }
        };
        assert_eq!(out.stop, Stop::ExitProcess { code: 0 }, "sleeper on {vcpus} vCPUs");
        assert_eq!(
            word, 0,
            "sleeper on {vcpus} vCPUs: the runtime word after the stop, held by {word:#x}"
        );
        let exits = |n: u16| out.exits.get(&n).copied().unwrap_or(0);
        assert!(
            !out.turn_start_ns.is_empty() && exits(1) > 0 && exits(12) > 0,
            "sleeper on {vcpus} vCPUs: the services reached, {} turns' starts and the exits {:?}",
            out.turn_start_ns.len(),
            out.exits
        );
        eprintln!(
            "sleeper on {vcpus} vCPUs: {} turns' starts, exits {:?}, runs by vCPU {:?}",
            out.turn_start_ns.len(),
            out.exits,
            out.runs_by_vcpu
        );
        if vcpus == 2 {
            assert!(
                out.runs_by_vcpu[1] > 0,
                "sleeper: vCPU 1 took no thread, runs {:?}",
                out.runs_by_vcpu
            );
        }
    }
}

/// `tests/data/timed_waits/<name>.exe` run by the `w2m-vm` binary on `vcpus` vCPUs, through `scripts/sign_and_run.sh`,
/// which signs it for the hypervisor, as the gates run each guest: its stop, its output and its stderr. The binary
/// gives the guest what an in-process run does not: its program's file and the file bridge (Sprint 328).
fn through_w2m_vm(name: &str, vcpus: u32) -> (String, String, String) {
    let program = root().join(format!("tests/data/timed_waits/{name}.exe"));
    program_through_w2m_vm(name, &program, &[], vcpus)
}

/// `program` run by the `w2m-vm` binary on `vcpus` vCPUs with `extra` arguments, as `through_w2m_vm` runs a guest
/// (Sprint 330: a family's probe with the arguments its `known` row gives, as the gates run it).
fn program_through_w2m_vm(name: &str, program: &Path, extra: &[String], vcpus: u32) -> (String, String, String) {
    let root = root();
    let dir = std::env::temp_dir().join(format!("w2m-{name}-{vcpus}-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let (output, stop) = (dir.join(format!("{name}.txt")), dir.join(format!("{name}.stop")));
    let mut child = std::process::Command::new(root.join("scripts/sign_and_run.sh"))
        .arg(env!("CARGO_BIN_EXE_w2m-vm"))
        .arg(program)
        .args(extra)
        .args(["--vcpus", &vcpus.to_string(), "--output"])
        .arg(&output)
        .arg("--stop")
        .arg(&stop)
        .current_dir(&root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        child.stdin.take().unwrap().write_all(b"{}\n").unwrap();
    }
    let ran = child.wait_with_output().unwrap();
    let stopped = fs::read_to_string(&stop).unwrap_or_default();
    let said = fs::read_to_string(&output).unwrap_or_default();
    let _ = fs::remove_dir_all(&dir);
    (
        stopped.trim_end().to_string(),
        said,
        String::from_utf8_lossy(&ran.stderr).into(),
    )
}

/// A guest's run through `w2m-vm` started in a process group of its own, so every process it starts is in that group
/// (Sprint 364): its stop, its output, its stderr, its time, and what `pgrep -g` finds of the group once it has ended.
struct GroupRun {
    stopped: String,
    said: String,
    stderr: String,
    took: Duration,
    left: String,
}

/// `tests/data/timed_waits/<name>.exe` run as `through_w2m_vm` runs it, with `extra` arguments, its process group its
/// own; `tag` names its directory.
fn guest_through_w2m_vm_in_a_group(name: &str, tag: &str, extra: &[&str], vcpus: u32) -> GroupRun {
    use std::os::unix::process::CommandExt;
    let root = root();
    let dir = std::env::temp_dir().join(format!("w2m-{tag}-{vcpus}-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let (output, stop) = (dir.join(format!("{name}.txt")), dir.join(format!("{name}.stop")));
    let started = Instant::now();
    let mut child = std::process::Command::new(root.join("scripts/sign_and_run.sh"))
        .arg(env!("CARGO_BIN_EXE_w2m-vm"))
        .arg(root.join(format!("tests/data/timed_waits/{name}.exe")))
        .args(extra)
        .args(["--vcpus", &vcpus.to_string(), "--output"])
        .arg(&output)
        .arg("--stop")
        .arg(&stop)
        .current_dir(&root)
        .process_group(0)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let group = child.id();
    {
        use std::io::Write;
        child.stdin.take().unwrap().write_all(b"{}\n").unwrap();
    }
    let ran = child.wait_with_output().unwrap();
    let took = started.elapsed();
    let left = std::process::Command::new("/usr/bin/pgrep")
        .args(["-l", "-g", &group.to_string()])
        .output()
        .unwrap();
    let stopped = fs::read_to_string(&stop).unwrap_or_default();
    let said = fs::read_to_string(&output).unwrap_or_default();
    let _ = fs::remove_dir_all(&dir);
    GroupRun {
        stopped: stopped.trim_end().to_string(),
        said,
        stderr: String::from_utf8_lossy(&ran.stderr).into(),
        took,
        left: String::from_utf8_lossy(&left.stdout).trim().to_string(),
    }
}

/// Sprint 364 (card 4f3; until then Sprint 327's refusal by name): `busychild` through `w2m-vm --vcpus 2` runs its
/// parent and its child at once, no turns between them (the child on one vCPU at Sprint 364, on two from Sprint 365):
/// the child spins for ever in its own process while the parent's two threads take their 20 rounds and end the run,
/// which ends the child; none of the group is left.
#[test]
fn busychild_through_w2m_vm_on_two_vcpus_runs_its_child_at_once() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let r = guest_through_w2m_vm_in_a_group("busychild", "busychild", &[], 2);
    assert_eq!(
        (r.stopped.as_str(), r.said.as_str()),
        ("ExitProcess { code: 0 }", "done\n"),
        "busychild through w2m-vm --vcpus 2; stderr {}",
        r.stderr
    );
    assert_eq!(r.left, "", "processes of the run's group left after its end");
}

/// Sprint 364 (card 4f3's gate, the design's "The deadlock rule"): `two_process_deadlock` on two vCPUs, a parent and its
/// child each waiting INFINITE on a named event only the other sets, stops as a deadlock by `threads::DEADLOCK`'s words,
/// the request server's verdict across processes; none of the group is left. Without the verdict each process parks
/// with none of its own and the run goes on to its time limit, GUEST_SECONDS (60 s), whose stop is "the run's time
/// limit". The verdict comes 1 to 1.1 s after both processes park (`cross_process::QUIET`, the server's 100 ms look),
/// and the whole run, the two `w2m-vm`s' loads included, took 1.19 s alone on this Mac (2026-11-26) and 2.18 s beside
/// 43 other tests. The time is printed, not held: no correctness test holds a wall-clock bound, which fails on the
/// machine's load (`crates/hypervisor/tests/no_wall_clock_bound.rs`, Sprint 224); the stop's words tell the verdict from
/// the time limit.
#[test]
fn a_deadlock_across_two_processes_on_two_vcpus_stops_as_one() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let r = guest_through_w2m_vm_in_a_group("two_process_deadlock", "two-process-deadlock", &[], 2);
    println!("two_process_deadlock on 2 vCPUs: {:?} in {:?}", r.stopped, r.took);
    assert_eq!(
        r.stopped,
        format!("Handover: {}", w2m_vm::threads::DEADLOCK),
        "two_process_deadlock through w2m-vm --vcpus 2 in {:?}; stderr {}",
        r.took,
        r.stderr
    );
    assert_eq!(r.left, "", "processes of the run's group left after its end");
}

/// Sprint 364 (card 4f3, the design's "The stop"; the user's Decision of 2026-10-18): a run that stops ends every child,
/// one waiting on a slot the server never answered among them. `two_process_deadlock`'s child, started with "-g", asks
/// CreateProcessW for a third process, which the request server refuses: it leaves the child's slot unanswered, the
/// child waiting on it, and stops the run naming the child; the run's end ends the child, and none of the group is left.
#[test]
fn a_stopped_run_ends_a_child_waiting_on_an_unanswered_slot() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let r = guest_through_w2m_vm_in_a_group(
        "two_process_deadlock",
        "unanswered-slot",
        &["--start", r#"{"arguments":"grandchild"}"#],
        2,
    );
    println!(
        "two_process_deadlock grandchild on 2 vCPUs: {:?} in {:?}",
        r.stopped, r.took
    );
    assert_eq!(
        r.stopped, "Handover: child process 4100: a grandchild process: not served yet",
        "stderr {}",
        r.stderr
    );
    assert_eq!(r.left, "", "processes of the run's group left after its end");
}

/// Sprint 365 (card 4f4's gate, `docs/design-a-process-that-starts-another-2026-11-26.md` "The cards that follow" 4):
/// `two_process_pipe_pingpong` through `w2m-vm --vcpus 2`, its child started with the run's count, so both processes
/// run on two vCPUs at once: the parent and its child pass one byte back and forth over two pipes, 10,000 rounds, every
/// read INFINITE. Each read blocks as often as not until the other process's write wakes it through the request server
/// and its link thread, so a wake lost between processes leaves both reading until GUEST_SECONDS (60 s), whose stop is
/// "the run's time limit". Twenty runs, each held to its own exit (`ExitProcess { code: 0 }`), the parent's count of
/// rounds answered right and the child's exit code ("rounds 10000, child 7"), each process's stderr line naming two
/// vCPUs (4096 the parent, 4100 the child), and none of the group left; each run's result and time printed, the failed
/// runs counted before the assertion, so a plant's result says in how many of 20 it failed. No time is held
/// (`crates/hypervisor/tests/no_wall_clock_bound.rs`).
#[test]
fn a_pipe_pingpong_across_two_processes_on_two_vcpus_ends_in_every_run() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let mut failed = Vec::new();
    for run in 1..=20 {
        let r = guest_through_w2m_vm_in_a_group("two_process_pipe_pingpong", &format!("pipe-pingpong-{run}"), &[], 2);
        let vcpus: Vec<&str> = r
            .stderr
            .lines()
            .filter(|l| l.contains(" vCPUs, runs by vCPU "))
            .collect();
        println!(
            "two_process_pipe_pingpong on 2 vCPUs, run {run}: {:?}, output {:?} in {:?}; {vcpus:?}",
            r.stopped, r.said, r.took
        );
        let on_two = |pid: u64| {
            vcpus
                .iter()
                .any(|l| l.starts_with(&format!("w2m-vm: process {pid} on 2 vCPUs, runs by vCPU ")))
        };
        if r.stopped != "ExitProcess { code: 0 }"
            || r.said != "rounds 10000, child 7\n"
            || !on_two(4096)
            || !on_two(4100)
            || !r.left.is_empty()
        {
            failed.push(format!(
                "run {run}: stop {:?}, output {:?}, vCPUs {vcpus:?}, left {:?}",
                r.stopped, r.said, r.left
            ));
        }
    }
    assert!(
        failed.is_empty(),
        "two_process_pipe_pingpong: {} of 20 runs on two vCPUs failed: {failed:#?}",
        failed.len()
    );
}

/// Sprint 365 (reviewer-4's thirty-third review, Low 1, taken into card 4f4): the run loop itself refuses the
/// instruction clock above one vCPU, by `run::instruction_clock_allowed`'s words, for every caller, before any thread or
/// vCPU; until then only `w2m-vm` refused it (`vcpus_allowed`), and a `run_compiling` caller with two vCPUs and the
/// clock ran (the Decision "The instruction clock on several vCPUs": "Only `w2m-vm` refuses it"). `qpc` started on the
/// instruction clock at the batch run's rate and epoch: on one vCPU it ends; on two the run stops at once by name,
/// no instruction run.
#[test]
fn the_run_loop_refuses_the_instruction_clock_above_one_vcpu() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let data = fs::read(root.join("tests/data/timed_waits/qpc.exe")).unwrap();
    let loaded = load(&data, None, "probe.exe").unwrap();
    for vcpus in [1, 2] {
        let mut start = Start::console("probe.exe");
        start.vcpus = vcpus;
        // `w2m-vm`'s CLOCK_RATE and REFERENCE_START, the batch run's
        start.instruction_clock = Some((1_400_000_000, 134_362_624_920_000_000));
        let mut guest = guest_with(&root, &loaded, &start);
        let out = run_limited(
            &mut guest,
            &loaded,
            &mut HostCompiler::default(),
            Some(Duration::from_secs(60)),
        );
        println!(
            "qpc on the instruction clock, {vcpus} vCPU(s): stop {:?}, {} instructions",
            out.stop, out.executed
        );
        if vcpus == 1 {
            assert_eq!(
                out.stop,
                Stop::ExitProcess { code: 0 },
                "qpc on the instruction clock, one vCPU"
            );
        } else {
            assert_eq!(
                out.stop,
                Stop::Exit("2 vCPUs refused: the instruction clock reads the running vCPU's own count".into()),
                "qpc on the instruction clock, two vCPUs: the run loop's own refusal"
            );
            assert_eq!(out.executed, 0, "instructions run before the refusal");
        }
    }
}

/// Deferred (Sprint 327, partial), met at Sprint 328: `statsize`'s worker, on vCPU 1 while the main thread spins on
/// vCPU 0, asks GetFileAttributesExW, whose stat the host writes into the requesting vCPU's runtime stack, and prints
/// "done" only when both threads read the same size; through `w2m-vm`, which gives it its file, on two vCPUs as on one.
/// In-process it has no file bridge and stops at its stat on any count of vCPUs, so it is not judged there.
#[test]
fn statsize_through_w2m_vm_on_two_vcpus_reads_its_own_stat() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    for vcpus in [1, 2] {
        let (stopped, said, stderr) = through_w2m_vm("statsize", vcpus);
        assert_eq!(
            stopped, "ExitProcess { code: 0 }",
            "statsize on {vcpus} vCPUs; stderr {stderr}"
        );
        assert_eq!(said, "done\n", "statsize's output on {vcpus} vCPUs");
    }
}

/// The threads family's probe as its row in `store/family_runs/known-2026-10-29.tsv` names it: the program, the DLL's
/// name beside it and the DLL's file.
fn threads_probe_files(root: &Path) -> (PathBuf, String, PathBuf) {
    let known = fs::read_to_string(root.join("store/family_runs/known-2026-10-29.tsv")).unwrap();
    let row: Vec<&str> = known
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split('\t').collect::<Vec<_>>())
        .find(|r| r[0] == "threads")
        .expect("the threads family's row");
    let (binary, beside, options) = (row[2], row[4], row[6]);
    let (name, dll) = beside.split_once(':').expect("the threads probe's DLL");
    assert_eq!(options, "-", "the threads probe takes no start options");
    (
        root.join(format!("store/binaries/{binary}.exe")),
        name.to_string(),
        root.join(format!("store/binaries/{dll}.dll")),
    )
}

/// The threads family's probe through `w2m-vm` on `vcpus` vCPUs, with the binary and DLL its row names, as the gates
/// run it (Sprint 330).
fn threads_probe(vcpus: u32) -> (String, String, String) {
    let (program, name, dll) = threads_probe_files(&root());
    let extra = [
        "--beside".to_string(),
        name,
        dll.to_string_lossy().into(),
        "--max-steps".into(),
        "20000000000".into(),
    ];
    program_through_w2m_vm("threads", &program, &extra, vcpus)
}

/// Sprint 342 (Sprint 341's Deferred: the none-class bridges let a guest's own calls judge the ring): the threads
/// family's probe runs in this process, as `w2m-vm` runs it (its DLL beside it, its file bridge on a fresh directory,
/// its 20,000,000,000 steps), on two vCPUs to its ExitProcess. Its main thread and its worker, which runs on vCPU 1
/// while the main thread goes on (Sprint 330), each call GetCurrentThreadId, GetLastError, SetLastError, TlsGetValue
/// and TlsSetValue (`probes/threads/threads.c` 79-91, 114-157), services of lock class none whose bridges take no
/// runtime word. After the run the call ring's `count` is its `per_api` total, each of those five counted, and the
/// 0x1000 bytes of the exception area past the ring are as before the run.
#[test]
fn the_threads_probes_own_calls_keep_the_ring_exact_on_two_vcpus() {
    use guest_runtime::layout::{EXCEPTION_OFF, RING_OFF};
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let (program, name, dll) = threads_probe_files(&root);
    let (data, dll_bytes) = (fs::read(&program).unwrap(), fs::read(&dll).unwrap());
    let supply = |d: &str| name.eq_ignore_ascii_case(d).then(|| dll_bytes.clone());
    let loaded = load(&data, Some(&supply), "probe.exe").unwrap();
    let app = std::env::temp_dir().join(format!("w2m-ring-threads-{}", std::process::id()));
    let _ = fs::remove_dir_all(&app);
    fs::create_dir_all(&app).unwrap();
    fs::write(app.join("probe.exe"), &data).unwrap();
    fs::write(app.join(&name), &dll_bytes).unwrap();
    let mut start = Start::console("probe.exe");
    start.vcpus = 2;
    start.app_root = Some(app.clone());
    let mut guest = guest_with(&root, &loaded, &start);
    guest.served.files = Some(w2m_vm::file_bridge::FileBridge::new(&app, false, None).unwrap());
    guest.limited = true;
    let (ring, exception) = (
        guest.offset(PROCESS_VA + RING_OFF).unwrap(),
        guest.offset(PROCESS_VA + EXCEPTION_OFF).unwrap(),
    );
    let mut exception_before = vec![0u8; 0x1000];
    guest.memory_view().read(exception, &mut exception_before);
    let vm = Vm::new(40).unwrap();
    vm.map(&guest.memory, IPA_BASE, memory::READ | memory::WRITE | memory::EXEC)
        .unwrap();
    let limit = Some(Duration::from_secs(60));
    let out = run_compiling(
        &mut guest,
        &loaded,
        &vm,
        limit,
        20_000_000_000,
        &mut HostCompiler::default(),
    )
    .unwrap();
    drop(guest.served.files.take());
    let _ = fs::remove_dir_all(&app);
    assert_eq!(
        out.stop,
        Stop::ExitProcess { code: 0 },
        "the threads probe on two vCPUs"
    );
    assert!(
        out.runs_by_vcpu.len() == 2 && out.runs_by_vcpu[1] > 0,
        "the threads probe: vCPU 1 ran no guest code, runs {:?}",
        out.runs_by_vcpu
    );
    let view = guest.memory_view();
    let count = view.read_u64(ring);
    let per_api = |id: u64| view.read_u64(ring + 16 + 8 * id);
    let total: u64 = (0..512).map(per_api).sum();
    let names = guest_runtime::services::api_names(&guest.services).unwrap();
    let id = |n: &str| names.iter().position(|m| m == n).unwrap() as u64 + 1;
    let none: Vec<(&str, u64)> = [
        "GetCurrentThreadId",
        "GetLastError",
        "SetLastError",
        "TlsGetValue",
        "TlsSetValue",
    ]
    .iter()
    .map(|n| (*n, per_api(id(n))))
    .collect();
    let mut after = vec![0u8; 0x1000];
    view.read(exception, &mut after);
    let changed = (0..0x1000).find(|&i| after[i] != exception_before[i]);
    eprintln!(
        "the threads probe on two vCPUs: the ring's count {count}, per_api's total {total}, the none-class calls \
         {none:?}, runs by vCPU {:?}, the exception area's first change {changed:?}",
        out.runs_by_vcpu
    );
    assert_eq!(count, total, "the ring's count against per_api's total");
    assert!(
        none.iter().all(|(_, n)| *n > 0),
        "a none-class service not called: {none:?}"
    );
    assert_eq!(
        changed, None,
        "the exception area's first changed byte past EXCEPTION_OFF"
    );
}

/// Sprint 330 (the run-loop design's 4c3b3, the kick): SuspendThread of the threads probe's spinner, which runs on vCPU
/// 1 while the main thread loops on SwitchToThread, stops it before it returns. Measured before the card: on two vCPUs
/// with no kick, `suspend.frozen`'s `count_unchanged` 0 and `context`'s `rip_in_image` 0, where one vCPU reads 1. Each
/// is held to 1, and the whole output to the one-vCPU run's.
#[test]
fn a_thread_suspended_on_another_vcpu_stops_before_suspend_returns() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let (stopped_one, said_one, _) = threads_probe(1);
    let (stopped, said, stderr) = threads_probe(2);
    assert_eq!(stopped_one, "ExitProcess { code: 0 }", "the threads probe on one vCPU");
    assert_eq!(
        stopped, "ExitProcess { code: 0 }",
        "the threads probe on two vCPUs; stderr {stderr}"
    );
    let row = |name: &str| {
        said.lines()
            .find(|l| l.starts_with(&format!("{{\"s\":\"{name}\",")))
            .unwrap_or_else(|| panic!("no {name} row"))
            .to_string()
    };
    assert!(
        row("suspend.frozen").contains("\"count_unchanged\":1"),
        "the suspended thread went on: {}",
        row("suspend.frozen")
    );
    assert!(
        row("context").contains("\"rip_in_image\":1"),
        "the context read before the thread stopped: {}",
        row("context")
    );
    assert_eq!(said, said_one, "the threads probe's rows on two vCPUs as on one");
}

/// Sprint 331 (the run-loop design's 4d1): `probes/locked_race`, a misaligned `lock xadd` stepped in the model
/// (`hvc #11`) on one vCPU against an aligned one, an exclusive pair in the VM, on another, the two sharing a byte. Its
/// rows on one vCPU and on two are held to the laptop's (run lap-20261127T221622950Z-62b931c2-001, two runs alike;
/// from Sprint 374, the runner's run 41326239822 equal to it by `cmp`): A's sum 100,000, B's adds lost 0; and its count
/// of locked steps to A's 100,000 adds.
#[test]
fn a_locked_model_step_loses_no_add_of_another_vcpus_aligned_lock() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let run = root.join("store/probe-runs/lap-20261127T221622950Z-62b931c2-001");
    let windows = fs::read_to_string(run.join("body-1.jsonl")).unwrap();
    let binary = fs::read_to_string(run.join("probe.sha256")).unwrap();
    let program = root.join(format!("store/binaries/{}.exe", binary.trim()));
    for vcpus in [1, 2] {
        let (stopped, said, stderr) = program_through_w2m_vm("locked_race", &program, &[], vcpus);
        assert_eq!(
            stopped, "ExitProcess { code: 0 }",
            "locked_race on {vcpus} vCPUs; stderr {stderr}"
        );
        assert_eq!(said, windows, "locked_race's rows on {vcpus} vCPUs as on Windows");
        // thread A's 100,000 misaligned adds, each a locked model step (precondition 8's count; reviewer-3's Low 1 at
        // 331); B's aligned adds run in the VM and count none
        assert!(
            stderr.contains("w2m-vm: 100000 model steps of a locked form"),
            "locked_race's locked steps on {vcpus} vCPUs; stderr {stderr}"
        );
    }
}

/// Sprint 344 (reviewer-4's thirteenth review, Medium 1): `probes/thread_values_race`, threads A and B each 200,000
/// rounds at once, each round setting its own last error and TLS value and reading them back and reading its own thread
/// id, all calls of lock class none (Sprint 342). Its rows on one vCPU and on two, but `overlap`, are held to the
/// laptop's (run lap-20261127T221622950Z-62b931c2-002, two runs alike but `overlap`; from Sprint 374, the runner's run
/// 45760720489 equal to it but `overlap`): each mismatch count 0. `overlap` counts the
/// rounds in which a thread saw the other's round count move: measured before the floor, 21 runs each way, 260 and 260
/// in every run on one vCPU, where the two loops meet only at the time slices' switches, and 155,991 to 170,312 (A)
/// and 158,350 to 170,509 (B) on two. On two vCPUs each is held above `OVERLAP_FLOOR`, which shows the loops ran at
/// once. The two-vCPU run is held first, so a fault both runs show is named by this card's judge: a process-wide last
/// error (the card's plant 2) also read 43 of A's rounds wrong on one vCPU, a time slice ending between its set and
/// its read.
#[test]
fn each_threads_last_error_tls_value_and_id_stay_its_own_on_two_vcpus() {
    const OVERLAP_FLOOR: u64 = 10_000;
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let run = root.join("store/probe-runs/lap-20261127T221622950Z-62b931c2-002");
    let but_overlap = |body: &str| -> String {
        body.lines()
            .filter(|l| !l.starts_with("{\"s\":\"overlap\","))
            .map(|l| format!("{l}\n"))
            .collect()
    };
    let windows = but_overlap(&fs::read_to_string(run.join("body-1.jsonl")).unwrap());
    for i in (2..).take_while(|i| run.join(format!("body-{i}.jsonl")).is_file()) {
        let body = fs::read_to_string(run.join(format!("body-{i}.jsonl"))).unwrap();
        assert_eq!(
            but_overlap(&body),
            windows,
            "the Windows run's body-{i} but overlap against body-1"
        );
    }
    let binary = fs::read_to_string(run.join("probe.sha256")).unwrap();
    let program = root.join(format!("store/binaries/{}.exe", binary.trim()));
    for vcpus in [2, 1] {
        let (stopped, said, stderr) = program_through_w2m_vm("thread_values_race", &program, &[], vcpus);
        assert_eq!(
            stopped, "ExitProcess { code: 0 }",
            "thread_values_race on {vcpus} vCPUs; stderr {stderr}"
        );
        eprintln!("thread_values_race on {vcpus} vCPUs:\n{said}");
        assert_eq!(
            but_overlap(&said),
            windows,
            "thread_values_race's rows but overlap on {vcpus} vCPUs as on Windows"
        );
        let overlap: serde_json::Value = said
            .lines()
            .find(|l| l.starts_with("{\"s\":\"overlap\","))
            .map(|l| serde_json::from_str(l).unwrap())
            .expect("thread_values_race's overlap row");
        let seen = ["a_saw_b_move", "b_saw_a_move"].map(|k| overlap[k].as_u64().unwrap());
        if vcpus == 2 {
            assert!(
                seen.iter().all(|&n| n > OVERLAP_FLOOR),
                "thread_values_race's loops on 2 vCPUs overlapped too little to judge, {seen:?}, floor \
                 {OVERLAP_FLOOR}"
            );
        }
    }
}

/// Sprint 345 (Change 1's TLS and FLS slots' lock): `probes/thread_slots_race`. Part 1: threads A and B each 20,000
/// rounds at once, each round taking 8 TLS indices by TlsAlloc, marking each by a compare-and-swap on a shared owner
/// word, then clearing the marks and freeing them by TlsFree; an index found marked by the other is a `duplicate`.
/// Part 2: 16 threads each make their FLS data by a first FlsSetValue of one index at once, the main thread frees it,
/// and a value read back after the free is `stale`. Its rows on two vCPUs and on one, but `tls.overlap`, are held to the
/// Windows runner's (run 45763850660, three runs alike but `tls.overlap`; kept from Sprint 374 until the laptop runs the
/// probe, Smart App Control having refused both its binaries there in Sprint 372, as
/// `store/reference_machine/runner-runs-after-the-laptop-2026-11-27.tsv` says): `duplicate` 0 and `stale` 0. `tls.overlap`
/// counts the rounds in which a thread saw the other's round count move: measured before the floor on the slots' lock,
/// 191 and 191 in each of 21 runs on one vCPU, where the loops meet only at the time slices' switches, and on two, in
/// 71 runs, 3,072 to 19,633, with one run at 0 and 0 (one thread's 20,000 rounds done before the other's began); with
/// Sprint 346's code sync, 19,581 to 19,894 in 100 runs of this test and 18,726 to 19,900 in 100 runs of the probe. On
/// two vCPUs each is held above `OVERLAP_FLOOR`, which shows the loops ran at once. With the seven
/// services off the runtime word and no lock, five runs of five on two vCPUs stopped at TlsFree of an index the other
/// thread had freed ("TlsFree argument form 0x5"), two threads given one index. Ignored from Sprint 345 to 346: on two
/// vCPUs the probe stopped at a guest runtime fault inside the code cache ("ESR 0x2000000 (EC 0x0)") in 30 of 100 runs,
/// a vCPU branching to a new block's entry before the block's words reached its instruction fetch; Sprint 346 syncs the
/// words before the entry is published and puts an ISB before the dispatcher's branch, and the test runs in the gates.
#[test]
fn two_threads_taking_and_freeing_tls_and_fls_slots_at_once_never_share_one_on_two_vcpus() {
    const OVERLAP_FLOOR: u64 = 1_000;
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let run = root.join("store/probe-runs/45763850660");
    let but_overlap = |body: &str| -> String {
        body.lines()
            .filter(|l| !l.starts_with("{\"s\":\"tls.overlap\","))
            .map(|l| format!("{l}\n"))
            .collect()
    };
    let windows = but_overlap(&fs::read_to_string(run.join("body-1.jsonl")).unwrap());
    for i in [2, 3] {
        let body = fs::read_to_string(run.join(format!("body-{i}.jsonl"))).unwrap();
        assert_eq!(
            but_overlap(&body),
            windows,
            "the Windows run's body-{i} but tls.overlap against body-1"
        );
    }
    let binary = fs::read_to_string(run.join("probe.sha256")).unwrap();
    let program = root.join(format!("store/binaries/{}.exe", binary.trim()));
    for vcpus in [2, 1] {
        let (stopped, said, stderr) = program_through_w2m_vm("thread_slots_race", &program, &[], vcpus);
        eprintln!("thread_slots_race on {vcpus} vCPUs:\n{said}");
        assert_eq!(
            stopped, "ExitProcess { code: 0 }",
            "thread_slots_race on {vcpus} vCPUs; its rows {said}; stderr {stderr}"
        );
        assert_eq!(
            but_overlap(&said),
            windows,
            "thread_slots_race's rows but tls.overlap on {vcpus} vCPUs as on Windows"
        );
        let overlap: serde_json::Value = said
            .lines()
            .find(|l| l.starts_with("{\"s\":\"tls.overlap\","))
            .map(|l| serde_json::from_str(l).unwrap())
            .expect("thread_slots_race's tls.overlap row");
        let seen = ["a_saw_b_move", "b_saw_a_move"].map(|k| overlap[k].as_u64().unwrap());
        if vcpus == 2 {
            assert!(
                seen.iter().all(|&n| n > OVERLAP_FLOOR),
                "thread_slots_race's loops on 2 vCPUs overlapped too little to judge, {seen:?}, floor \
                 {OVERLAP_FLOOR}"
            );
        }
    }
}

/// Sprint 347 (reviewer-4's fifteenth review, Low 1): `probes/tls_free_reach`. A worker sets its value at a TLS index
/// and so does the main thread; the main thread frees the index and the worker reads it; the main thread allocates
/// again, taking the same index, and the worker reads again, then the main thread reads its own. Once at a TEB slot
/// (below 64) and once at an expansion slot (64 and up), each step waiting on an event. Its rows on one vCPU and on
/// two are held to the laptop's (run lap-20261127T221622950Z-62b931c2-004, two runs alike; from Sprint 374, the
/// runner's run 45780352907 equal to it by `cmp`) field by field, each field unlike Windows'
/// named: the worker reads 0 after the free and after the new allocation. Before the change both read 2827 (0xB0B, the
/// worker's value) on one vCPU and on two, TlsFree and TlsAlloc clearing the caller's slot alone.
#[test]
fn another_threads_tls_value_after_tlsfree_and_tlsalloc_is_windows_on_one_and_two_vcpus() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let run = root.join("store/probe-runs/lap-20261127T221622950Z-62b931c2-004");
    let windows = fs::read_to_string(run.join("body-1.jsonl")).unwrap();
    for i in (2..).take_while(|i| run.join(format!("body-{i}.jsonl")).is_file()) {
        let body = fs::read_to_string(run.join(format!("body-{i}.jsonl"))).unwrap();
        assert_eq!(body, windows, "the Windows run's body-{i} against body-1");
    }
    let rows = |body: &str| -> Vec<serde_json::Map<String, serde_json::Value>> {
        body.lines()
            .map(|l| match serde_json::from_str(l).unwrap() {
                serde_json::Value::Object(m) => m,
                other => panic!("a row not an object: {other}"),
            })
            .collect()
    };
    let expected = rows(&windows);
    let binary = fs::read_to_string(run.join("probe.sha256")).unwrap();
    let program = root.join(format!("store/binaries/{}.exe", binary.trim()));
    for vcpus in [1, 2] {
        let (stopped, said, stderr) = program_through_w2m_vm("tls_free_reach", &program, &[], vcpus);
        eprintln!("tls_free_reach on {vcpus} vCPUs:\n{said}");
        assert_eq!(
            stopped, "ExitProcess { code: 0 }",
            "tls_free_reach on {vcpus} vCPUs; its rows {said}; stderr {stderr}"
        );
        let seen = rows(&said);
        let names = |r: &[serde_json::Map<String, serde_json::Value>]| -> Vec<String> {
            r.iter().map(|m| m["s"].to_string()).collect()
        };
        assert_eq!(
            names(&seen),
            names(&expected),
            "tls_free_reach's rows on {vcpus} vCPUs against Windows'"
        );
        let unlike: Vec<String> = seen
            .iter()
            .zip(&expected)
            .flat_map(|(s, w)| {
                w.iter()
                    .filter(|(k, v)| s.get(*k) != Some(v))
                    .map(|(k, v)| {
                        let row = w["s"].as_str().unwrap_or("?");
                        let had = s.get(k).map_or("none".into(), |x| x.to_string());
                        format!("{row}.{k}: {had} against {v}")
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        assert!(
            unlike.is_empty(),
            "tls_free_reach on {vcpus} vCPUs, fields unlike Windows' {unlike:?}"
        );
        assert_eq!(said, windows, "tls_free_reach's rows on {vcpus} vCPUs as on Windows");
    }
}

/// Sprint 347's amendment (found by builder-347's experiment): `probes/tls_expansion_first`. A worker takes TLS
/// indices until one lies past the 64 TEB slots and ends; the first thread, which took none past 63, sets its own
/// value at that index and reads it back. Its row on one vCPU and on two is held to the laptop's (run
/// lap-20261127T221622950Z-62b931c2-005, two runs alike; from Sprint 374, the runner's run 45783238565 equal to it by
/// `cmp`): set 1, got 1511, the last error 0 after the read. Before the fix the first thread
/// had no expansion array, and the run stopped at "HostRequest op 4: TlsSetValue argument form 0x40".
#[test]
fn the_first_thread_uses_an_expansion_index_another_thread_allocated_on_one_and_two_vcpus() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let run = root.join("store/probe-runs/lap-20261127T221622950Z-62b931c2-005");
    let windows = fs::read_to_string(run.join("body-1.jsonl")).unwrap();
    for i in (2..).take_while(|i| run.join(format!("body-{i}.jsonl")).is_file()) {
        let body = fs::read_to_string(run.join(format!("body-{i}.jsonl"))).unwrap();
        assert_eq!(body, windows, "the Windows run's body-{i} against body-1");
    }
    let binary = fs::read_to_string(run.join("probe.sha256")).unwrap();
    let program = root.join(format!("store/binaries/{}.exe", binary.trim()));
    for vcpus in [1, 2] {
        let (stopped, said, stderr) = program_through_w2m_vm("tls_expansion_first", &program, &[], vcpus);
        eprintln!("tls_expansion_first on {vcpus} vCPUs:\n{said}");
        assert_eq!(
            stopped, "ExitProcess { code: 0 }",
            "tls_expansion_first on {vcpus} vCPUs; its rows {said}; stderr {stderr}"
        );
        assert_eq!(
            said, windows,
            "tls_expansion_first's row on {vcpus} vCPUs as on Windows"
        );
    }
}

/// Sprint 350 (reviewer-4's nineteenth review, Medium 1): `tests/data/timed_waits/shared_close`. A worker waits for all
/// of a named event, the host's, and an unnamed one; the main thread closes the named event's only handle, writes
/// "closed", sets the unnamed event, writes "done" and ends the process. The worker's entry still holds the named event,
/// yet the host's close of it (W2M_HOST_KERNEL 33, W2M_KERNEL_CLOSE 6) comes at CloseHandle, before "closed" is written
/// (W2M_HOST_WRITE 1), as it did before the sprint: the exit ledger's request rows hold the order. The set, which offers
/// the wait-all its objects, asks the host nothing of the closed event (its id is no longer an event there), and the run
/// ends.
#[test]
fn the_hosts_close_of_a_shared_event_comes_at_its_last_handle_while_a_wait_holds_it() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let (loaded, mut guest) = guest_for(&root, "shared_close");
    let path = std::env::temp_dir().join(format!("w2m-shared-close-{}.tsv", std::process::id()));
    guest.exit_ledger = Some(w2m_vm::exit_ledger::Ledger::create(&path).unwrap());
    let out = run(&mut guest, &loaded, &mut HostCompiler::default());
    let stdout = String::from_utf8_lossy(&guest.served.stdout).into_owned();
    guest.exit_ledger = None;
    let ledger = fs::read_to_string(&path).unwrap();
    let _ = fs::remove_file(&path);
    assert_eq!(
        out.stop,
        Stop::ExitProcess { code: 0 },
        "shared_close's stop; stdout {stdout:?}"
    );
    assert_eq!(stdout, "closed\ndone\n", "shared_close's output");
    let requests: Vec<(usize, &str, &str)> = ledger
        .lines()
        .enumerate()
        .filter_map(|(n, l)| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() > 6 && f[1] == "hvc 8").then(|| (n, f[5], f[6]))
        })
        .collect();
    let close = requests
        .iter()
        .find(|(_, op, a0)| *op == "21" && *a0 == "6")
        .map(|r| r.0);
    let write = requests.iter().find(|(_, op, _)| *op == "1").map(|r| r.0);
    assert!(
        close.is_some() && write.is_some() && close < write,
        "the host's close of the shared event (ledger row {close:?}) before the write of \"closed\" (row {write:?})"
    );
}

/// The bound on the ratio of the largest to the smallest of three_counters' window counts (Sprint 353).
const SHARE_RATIO: f64 = 1.5;

/// Sprint 353 (reviewer-4's twenty-first review, Medium 1): `tests/data/timed_waits/three_counters`, three CPU-bound
/// threads in one loop on two vCPUs, the second and third made while the first runs on vCPU 1, judges slicing past
/// vCPU 0. Its window runs from the first's readying of the third, on its own vCPU, to the third's count of 40,000,000,
/// all three ready throughout; each thread's count over it, from the guest's `.counts` section, is its share of the two
/// vCPUs. A vCPU whose `SLICE_UNTIL` another vCPU's readying or put-down left at ~0 ran one thread alone while two
/// shared the other, about twice their counts; a vCPU that took a thread on the run's whole budget (Sprint 351) the
/// same. 20 runs on two vCPUs, each held to ExitProcess inside the run's limit of 60 s, "done" and every count above 0;
/// then every run's window, the largest count at most `SHARE_RATIO` times the smallest, each run quoted first.
#[test]
fn three_busy_threads_on_two_vcpus_each_count_and_the_run_ends() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let data = fs::read(root.join("tests/data/timed_waits/three_counters.exe")).unwrap();
    let loaded = load(&data, None, "probe.exe").unwrap();
    let program = &loaded.images[0];
    let section = program
        .pe
        .sections
        .iter()
        .find(|s| s.name == ".counts")
        .expect("three_counters' .counts section");
    let counts_va = program.base + u64::from(section.virtual_address);
    // the switches the VM made and the host's (`hvc #12`), for one vCPU's run against two's (reviewer-4's twenty-third
    // review, Low 1: on two vCPUs a vCPU's slice ends at each TIME_SLICE while the other holds a thread)
    let switches = |guest: &Guest, out: &Outcome| {
        (
            guest.memory_view().read_u64(RUN_TABLE + w2m_vm::threads::RT_SWITCHES),
            out.exits.get(&12).copied().unwrap_or(0),
        )
    };
    let mut alone = guest_with(&root, &loaded, &Start::console("probe.exe"));
    let out = run_limited(
        &mut alone,
        &loaded,
        &mut HostCompiler::default(),
        Some(Duration::from_secs(60)),
    );
    println!(
        "three_counters on 1 vCPU: stop {:?}, switches in the VM and hvc #12 {:?}",
        out.stop,
        switches(&alone, &out)
    );
    assert_eq!(out.stop, Stop::ExitProcess { code: 0 }, "three_counters on 1 vCPU");
    let mut start = Start::console("probe.exe");
    start.vcpus = 2;
    let mut windows = Vec::new();
    for run in 1..=20 {
        let mut guest = guest_with(&root, &loaded, &start);
        let out = run_limited(
            &mut guest,
            &loaded,
            &mut HostCompiler::default(),
            Some(Duration::from_secs(60)),
        );
        let at = guest.offset(counts_va).expect("the .counts section mapped");
        // the counts, then each at the first's readying of the third, then each at the third's WINDOW
        let words: Vec<u64> = (0..9).map(|i| guest.memory_view().read_u64(at + 8 * i)).collect();
        let (counts, at_go, at_ran) = (&words[0..3], &words[3..6], &words[6..9]);
        let window: Vec<u64> = (0..3).map(|j| at_ran[j].wrapping_sub(at_go[j])).collect();
        let stdout = String::from_utf8_lossy(&guest.served.stdout).into_owned();
        println!(
            "three_counters on 2 vCPUs, run {run}: stop {:?}, counts {counts:?}, window {window:?}, runs by vCPU {:?}, \
             switches in the VM and hvc #12 {:?}",
            out.stop,
            out.runs_by_vcpu,
            switches(&guest, &out)
        );
        assert_eq!(
            out.stop,
            Stop::ExitProcess { code: 0 },
            "three_counters on 2 vCPUs, run {run}: counts {counts:?}, output {stdout:?}"
        );
        assert!(
            counts.iter().all(|&c| c > 0),
            "three_counters on 2 vCPUs, run {run}: a thread's count 0, counts {counts:?}"
        );
        assert_eq!(stdout, "done\n", "three_counters' output on 2 vCPUs, run {run}");
        windows.push(window);
    }
    let ratios: Vec<f64> = windows
        .iter()
        .map(|w| *w.iter().max().unwrap() as f64 / (*w.iter().min().unwrap()).max(1) as f64)
        .collect();
    println!("three_counters' window ratios, largest to smallest: {ratios:.3?}");
    if let Some(run) = ratios.iter().position(|&r| r > SHARE_RATIO) {
        panic!(
            "three_counters on 2 vCPUs, run {}: window {:?}, a thread ran {:.3} times another's, over {SHARE_RATIO}",
            run + 1,
            windows[run],
            ratios[run]
        );
    }
}

/// The lost-wakeup guest's instruction limit (Sprint 354): about eleven times its runs' count, 1,800,073 on one vCPU and
/// on two (its instructions are the program's alone, no thread spinning), so a run that loses a readying, once card 5
/// opens the window, stops as a deadlock or at the limit, never at ExitProcess.
const LOST_WAKEUP_LIMIT: u64 = 20_000_000;

/// Sprint 354 (card 3 of `docs/design-handles-and-sync-objects-2026-11-26.md`, its judge "The lost wakeup"):
/// `tests/data/timed_waits/lost_wakeup`, two threads taking turns over two auto-reset events 100,000 rounds, every wait
/// INFINITE, so a readying lost between a wait's check and its park leaves both waiting with no deadline and the run
/// stops as a deadlock. No run reaches that window before card 5 takes waits off the runtime word, so the guest is
/// held here for card 5 and its plant (the woken mark ignored in `w2m_block`): one run on one vCPU and 20 on two, each
/// held to ExitProcess and "done" inside `LOST_WAKEUP_LIMIT` instructions and 60 s, each run's count printed. Sprint 356:
/// the failed runs are counted before the assertion, so a plant's result says in how many of 20 runs it failed.
#[test]
fn the_lost_wakeup_guest_ends_inside_its_limit_on_two_vcpus() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let data = fs::read(root.join("tests/data/timed_waits/lost_wakeup.exe")).unwrap();
    let loaded = load(&data, None, "probe.exe").unwrap();
    let mut counts = Vec::new();
    let mut failed = Vec::new();
    for (run, vcpus) in std::iter::once((0, 1)).chain((1..=20).map(|run| (run, 2))) {
        let mut start = Start::console("probe.exe");
        start.vcpus = vcpus;
        let mut guest = guest_with(&root, &loaded, &start);
        let vm = Vm::new(40).unwrap();
        vm.map(&guest.memory, IPA_BASE, memory::READ | memory::WRITE | memory::EXEC)
            .unwrap();
        let out = run_compiling(
            &mut guest,
            &loaded,
            &vm,
            Some(Duration::from_secs(60)),
            LOST_WAKEUP_LIMIT,
            &mut HostCompiler::default(),
        )
        .unwrap();
        let stdout = String::from_utf8_lossy(&guest.served.stdout).into_owned();
        println!(
            "lost_wakeup on {vcpus} vCPU(s), run {run}: stop {:?}, {} instructions, runs by vCPU {:?}, output {stdout:?}",
            out.stop, out.executed, out.runs_by_vcpu
        );
        let ran_both = vcpus == 1 || (out.runs_by_vcpu.len() == 2 && out.runs_by_vcpu[1] > 0);
        if out.stop != (Stop::ExitProcess { code: 0 })
            || stdout != "done\n"
            || out.executed >= LOST_WAKEUP_LIMIT
            || !ran_both
        {
            failed.push(format!(
                "{vcpus} vCPU(s) run {run}: stop {:?}, output {stdout:?}, {} instructions (limit {LOST_WAKEUP_LIMIT}), \
                 runs by vCPU {:?}",
                out.stop, out.executed, out.runs_by_vcpu
            ));
        }
        if vcpus == 2 {
            counts.push(out.executed);
        }
    }
    println!(
        "lost_wakeup's 20 runs on two vCPUs: {} to {} instructions, limit {LOST_WAKEUP_LIMIT}",
        counts.iter().min().unwrap(),
        counts.iter().max().unwrap()
    );
    assert!(
        failed.is_empty(),
        "lost_wakeup: {} of 21 runs failed (one on one vCPU, 20 on two): {failed:#?}",
        failed.len()
    );
}

/// The floor on each monotonic_clock thread's readings taken after the other's last reading moved, on two vCPUs (Sprint
/// 360): a run below it read beside the other too little to judge the clock across vCPUs.
const CLOCK_OVERLAP_FLOOR: u64 = 1_000;

/// The monotonic_clock guest's instruction limit (Sprint 360): its two threads' 200,000 rounds each, about 30 x86
/// instructions a round, well inside.
const MONOTONIC_CLOCK_LIMIT: u64 = 200_000_000;

/// Sprint 360 (its card's monotonicity judge, on the clock a run of several vCPUs takes, the wall clock):
/// `tests/data/timed_waits/monotonic_clock`, two threads reading QueryPerformanceCounter 200,000 times each, each before
/// every reading taking the other's last and its own, so a reading below one any thread took before counts in its
/// `below`. QueryPerformanceCounter "does not go backward" and is "consistent across all processors in multi-core and
/// multi-processor systems, even when measured on different threads or processes" (Microsoft Learn, "Acquiring
/// high-resolution time stamps"). One run on one vCPU and 20 on two, each held to ExitProcess and "done" inside
/// `MONOTONIC_CLOCK_LIMIT` and 60 s, both `below` 0; on two vCPUs both vCPUs ran and each thread saw the other's reading
/// move at least `CLOCK_OVERLAP_FLOOR` times. Each run's figures are printed, and the failed runs counted before the
/// assertion, so a plant's result says in how many of 21 it failed.
#[test]
fn the_wall_clock_never_reads_below_a_reading_taken_on_two_vcpus() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let data = fs::read(root.join("tests/data/timed_waits/monotonic_clock.exe")).unwrap();
    let loaded = load(&data, None, "probe.exe").unwrap();
    let program = &loaded.images[0];
    let section = program
        .pe
        .sections
        .iter()
        .find(|s| s.name == ".clock")
        .expect("monotonic_clock's .clock section");
    let clock_va = program.base + u64::from(section.virtual_address);
    let mut failed = Vec::new();
    for (run, vcpus) in std::iter::once((0, 1)).chain((1..=20).map(|run| (run, 2))) {
        let mut start = Start::console("probe.exe");
        start.vcpus = vcpus;
        let mut guest = guest_with(&root, &loaded, &start);
        let vm = Vm::new(40).unwrap();
        vm.map(&guest.memory, IPA_BASE, memory::READ | memory::WRITE | memory::EXEC)
            .unwrap();
        let out = run_compiling(
            &mut guest,
            &loaded,
            &vm,
            Some(Duration::from_secs(60)),
            MONOTONIC_CLOCK_LIMIT,
            &mut HostCompiler::default(),
        )
        .unwrap();
        let at = guest.offset(clock_va).expect("the .clock section mapped");
        // below, moved, first_low, first_seen, last: two words each, the main thread's first
        let w: Vec<u64> = (0..10).map(|i| guest.memory_view().read_u64(at + 8 * i)).collect();
        let (below, moved, first_low, first_seen, last) = (&w[0..2], &w[2..4], &w[4..6], &w[6..8], &w[8..10]);
        let stdout = String::from_utf8_lossy(&guest.served.stdout).into_owned();
        println!(
            "monotonic_clock on {vcpus} vCPU(s), run {run}: stop {:?}, {} instructions, runs by vCPU {:?}, below \
             {below:?}, moved {moved:?}, first reading below one seen {first_low:?} under {first_seen:?}, last \
             readings {last:?}",
            out.stop, out.executed, out.runs_by_vcpu
        );
        let ran_both = vcpus == 1 || (out.runs_by_vcpu.len() == 2 && out.runs_by_vcpu[1] > 0);
        let overlapped = vcpus == 1 || moved.iter().all(|&m| m >= CLOCK_OVERLAP_FLOOR);
        if out.stop != (Stop::ExitProcess { code: 0 })
            || stdout != "done\n"
            || below.iter().any(|&b| b != 0)
            || !ran_both
            || !overlapped
        {
            failed.push(format!(
                "{vcpus} vCPU(s) run {run}: stop {:?}, output {stdout:?}, below {below:?} (first {first_low:?} under \
                 {first_seen:?}), moved {moved:?} (floor {CLOCK_OVERLAP_FLOOR}), runs by vCPU {:?}",
                out.stop, out.runs_by_vcpu
            ));
        }
    }
    assert!(
        failed.is_empty(),
        "monotonic_clock: {} of 21 runs failed (one on one vCPU, 20 on two): {failed:#?}",
        failed.len()
    );
}

/// Sprint 360: a family's start on the instruction clock, as the gates' two-vCPU family runs give it (`--start
/// {"instruction_clock":true} --vcpus 2`), runs on the wall clock, which `w2m-vm` says on stderr; the instruction clock
/// stays on one vCPU (`run::vcpus_allowed`). `monotonic_clock` so started, through the binary, 20 times: each run held
/// to ExitProcess, its stderr naming the wall clock, and its output "done", no reading below one taken before. The
/// failed runs are counted before the assertion.
#[test]
fn a_family_start_on_the_instruction_clock_reads_the_wall_clock_on_two_vcpus() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let program = root().join("tests/data/timed_waits/monotonic_clock.exe");
    let start = ["--start".to_string(), r#"{"instruction_clock":true}"#.to_string()];
    let mut failed = Vec::new();
    for run in 1..=20 {
        let (stopped, said, stderr) = program_through_w2m_vm("monotonic_clock", &program, &start, 2);
        let wall = stderr.contains("the family's instruction clock runs on one vCPU; on 2 vCPUs, the wall clock");
        println!(
            "monotonic_clock through w2m-vm on 2 vCPUs, run {run}: stop {stopped:?}, output {said:?}, wall clock {wall}"
        );
        if stopped != "ExitProcess { code: 0 }" || said != "done\n" || !wall {
            failed.push(format!(
                "run {run}: stop {stopped:?}, output {said:?}, stderr {stderr:?}"
            ));
        }
    }
    assert!(
        failed.is_empty(),
        "monotonic_clock through w2m-vm on 2 vCPUs: {} of 20 runs failed: {failed:#?}",
        failed.len()
    );
}

/// The floor on each handle_values thread's rounds that saw the other's round count move, on two vCPUs (Sprint 355).
const HANDLE_OVERLAP_FLOOR: u64 = 1_000;

/// Sprint 355 (card 4 of `docs/design-handles-and-sync-objects-2026-11-26.md`, its judge "Handles from two vCPUs"):
/// `tests/data/timed_waits/handle_values`, two threads each creating 10,000 unnamed events and duplicating one of the
/// main thread's five objects 10,000 times (two unnamed manual-reset events, one set, one never; a named event; a
/// semaphore and an auto-reset event that a third thread waits on, blocking, and gives, while it also makes pipes and
/// sets and reads their flags), waiting on the first three's duplicates and the new event with a zero timeout and
/// closing both; then 1,000 bursts each of 16 duplicates of each event, a zero-timeout wait for any of the set one's,
/// every 8th burst 1 ms waits for any and for all of the unset one's, which block and drop their entries' references
/// at the timeout, and the 32 closes. CreateEventA and DuplicateHandle take the handle table's lock and no runtime word,
/// CloseHandle and the waits take the word; meanwhile the main thread's 1 ms waits for any and for all of those events
/// block too. Its `.counts` section holds the values claimed and those two threads held at once, the closes answered
/// TRUE and FALSE, the waits and duplicates that answered unlike their object, the first create's value with the value
/// of the first of 256 creates after every handle is closed, each thread's rounds that saw the other's move, and the
/// main thread's and the third thread's rounds. One run on one vCPU and 20 on two, each printed; every run must end at
/// ExitProcess with "done", its counts whole (104,064 values claimed, none at once with another, 104,332 closes TRUE,
/// none FALSE, none unlike its object, 256 made, the first value again), and on two vCPUs each thread's overlap above
/// `HANDLE_OVERLAP_FLOOR`, a round of the main thread's and of the third's, and the second vCPU running a thread. The failed runs are
/// counted before the assertion, so a plant's result says in how many of 20 runs it failed.
#[test]
fn two_vcpus_creating_duplicating_and_closing_handles_keep_every_value_distinct_and_every_slot() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let data = fs::read(root.join("tests/data/timed_waits/handle_values.exe")).unwrap();
    let loaded = load(&data, None, "probe.exe").unwrap();
    let program = &loaded.images[0];
    let section = program
        .pe
        .sections
        .iter()
        .find(|s| s.name == ".counts")
        .expect("handle_values' .counts section");
    let counts_va = program.base + u64::from(section.virtual_address);
    const NAMES: [&str; 12] = [
        "claimed", "collided", "closed", "refused", "wrong", "first", "again", "made", "overlap1", "overlap2", "waits",
        "third",
    ];
    let whole: [u64; 8] = [104_064, 0, 104_332, 0, 0, 0x2000, 0x2000, 256];
    let mut failed = Vec::new();
    for (run, vcpus) in std::iter::once((0, 1)).chain((1..=20).map(|run| (run, 2))) {
        let mut start = Start::console("probe.exe");
        start.vcpus = vcpus;
        let mut guest = guest_with(&root, &loaded, &start);
        let out = run_limited(
            &mut guest,
            &loaded,
            &mut HostCompiler::default(),
            Some(Duration::from_secs(60)),
        );
        let at = guest.offset(counts_va).expect("the .counts section mapped");
        let words: Vec<u64> = (0..12).map(|i| guest.memory_view().read_u64(at + 8 * i)).collect();
        let stdout = String::from_utf8_lossy(&guest.served.stdout).into_owned();
        let counts: Vec<String> = NAMES.iter().zip(&words).map(|(n, w)| format!("{n} {w:#x}")).collect();
        println!(
            "handle_values on {vcpus} vCPU(s), run {run}: stop {:?}, output {stdout:?}, {}, runs by vCPU {:?}",
            out.stop,
            counts.join(", "),
            out.runs_by_vcpu
        );
        let overlapped = vcpus == 1
            || (words[8] > HANDLE_OVERLAP_FLOOR
                && words[9] > HANDLE_OVERLAP_FLOOR
                && words[10] > 0
                && words[11] > 0
                && out.runs_by_vcpu.len() == 2
                && out.runs_by_vcpu[1] > 0);
        if out.stop != (Stop::ExitProcess { code: 0 }) || stdout != "done\n" || words[..8] != whole || !overlapped {
            failed.push(format!(
                "{vcpus} vCPU(s) run {run}: stop {:?}, output {stdout:?}, {}",
                out.stop,
                counts.join(", ")
            ));
        }
    }
    assert!(
        failed.is_empty(),
        "handle_values: {} of 21 runs failed (one on one vCPU, 20 on two): {failed:#?}",
        failed.len()
    );
}

/// The floor on each semaphore_units thread's rounds that saw the other threads' rounds move, on two vCPUs (Sprint 356).
const SEMAPHORE_OVERLAP_FLOOR: u64 = 1_000;

/// semaphore_units' wall-clock limit a run (Sprint 356): its runs end in 0.9 to 1.4 s on two vCPUs, and a run a plant
/// leaves waiting is held alive by its 1 ms waits, so the limit, not a deadlock, ends it.
const SEMAPHORE_RUN_LIMIT: Duration = Duration::from_secs(20);

/// Sprint 356 (card 5 of `docs/design-handles-and-sync-objects-2026-11-26.md`, its judge "A semaphore's units across
/// vCPUs"): `tests/data/timed_waits/semaphore_units`, four threads each releasing one unit of one semaphore (maximum
/// 1,000) and then waiting for one, 100,000 rounds: by WaitForSingleObject with no timeout, every 8th with a zero
/// timeout, and by WaitForMultipleObjects with no timeout, for any of it (every 16th from the 4th), of it and the main
/// thread's handle (every 16th from the 12th), for all of it (every 32nd from the 16th), and of it and an ended thread's
/// handle (every 32nd). Strengthened for plant 4: two locks the four threads share and enter by turns, a mutex (every
/// 8th round waited on with no timeout, every other even round with a zero timeout, then released) and an auto-reset
/// event made set (every 8th round from the 5th with no timeout, every other odd round with a zero timeout, then set
/// again), each entry counted by an atomic add and a plain add, which a second holder at once would lose; every 64th
/// round a manual-reset event reset, set and waited on; and 1 ms waits that time out, the main thread's on each
/// worker's handle in turn until it ends and two more threads' on the main thread's handle until the four end. No wait
/// is on a named event, whose set readies every blocked thread of the process and so would make good a grant a plant
/// had lost. The eight services take the sync objects' lock and hold the runtime word only to park, to ready and, for
/// the waits naming one of the host's objects, from their check to their park. Its `.counts` section holds the
/// releases answered TRUE and FALSE, the semaphore's waits satisfied, its zero-timeout waits timed out, the other
/// answers, the semaphore's final count, each worker's rounds that saw the others' move, each lock's entries counted
/// both ways, the entries that found another thread inside, the manual-reset event's waits returned, and the 1 ms waits
/// that timed out. One run on one vCPU and 20 on two, each printed; every run must end at ExitProcess with "done"
/// inside `SEMAPHORE_RUN_LIMIT`, its counts whole (400,000 releases TRUE, none FALSE, none wrong, every wait satisfied
/// or timed out at a zero timeout, the final count the releases less the satisfied waits, each lock's two counts equal and at least its
/// 50,000 waits with no timeout, none shared, 6,248 gate waits, a 1 ms wait timed out), and on two vCPUs each worker's
/// overlap above `SEMAPHORE_OVERLAP_FLOOR` and the second vCPU running a thread. The failed runs are counted before the
/// assertion, so a plant's result says in how many of 20 runs it failed.
#[test]
fn four_threads_on_two_vcpus_keep_a_semaphores_units_and_every_wait_returns() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let data = fs::read(root.join("tests/data/timed_waits/semaphore_units.exe")).unwrap();
    let loaded = load(&data, None, "probe.exe").unwrap();
    let program = &loaded.images[0];
    let section = program
        .pe
        .sections
        .iter()
        .find(|s| s.name == ".counts")
        .expect("semaphore_units' .counts section");
    let counts_va = program.base + u64::from(section.virtual_address);
    const NAMES: [&str; 17] = [
        "released",
        "refused",
        "satisfied",
        "timed_out",
        "wrong",
        "final",
        "overlap0",
        "overlap1",
        "overlap2",
        "overlap3",
        "mutex_taken",
        "mutex_counted",
        "turn_taken",
        "turn_counted",
        "shared",
        "gated",
        "watched",
    ];
    const WAITS: u64 = 400_000;
    const LOCKED: u64 = 50_000;
    const GATED: u64 = 4 * (100_000 / 64);
    let mut failed = Vec::new();
    for (run, vcpus) in std::iter::once((0, 1)).chain((1..=20).map(|run| (run, 2))) {
        let mut start = Start::console("probe.exe");
        start.vcpus = vcpus;
        let mut guest = guest_with(&root, &loaded, &start);
        let out = run_limited(
            &mut guest,
            &loaded,
            &mut HostCompiler::default(),
            Some(SEMAPHORE_RUN_LIMIT),
        );
        let at = guest.offset(counts_va).expect("the .counts section mapped");
        let w: Vec<u64> = (0..17).map(|i| guest.memory_view().read_u64(at + 8 * i)).collect();
        let stdout = String::from_utf8_lossy(&guest.served.stdout).into_owned();
        let mut counts: Vec<String> = NAMES
            .iter()
            .zip(&w)
            .map(|(n, v)| format!("{n} {}", *v as i64))
            .collect();
        // the run table's count of blocked threads against its entries' BLOCKED statuses (strengthened for plant 4, a
        // park or a readying without the runtime word: `set_status`' read, add and store of the count, threads.c 76-79,
        // lose one another's change only when two vCPUs make them at once)
        let view = guest.memory_view();
        let blocked = view.read_u64(RUN_TABLE + w2m_vm::threads::RT_BLOCKED);
        let in_entries = guest
            .served
            .threads
            .values()
            .filter(|t| {
                let entry = RUN_TABLE + guest_runtime::layout::RT_ENTRIES + guest_runtime::layout::RUN_ENTRY * t.index;
                view.read_u64(entry + w2m_vm::threads::E_STATUS) == w2m_vm::threads::BLOCKED
            })
            .count() as u64;
        counts.push(format!("blocked {} against {in_entries} entries", blocked as i64));
        println!(
            "semaphore_units on {vcpus} vCPU(s), run {run}: stop {:?}, output {stdout:?}, {}, runs by vCPU {:?}",
            out.stop,
            counts.join(", "),
            out.runs_by_vcpu
        );
        let whole = w[0] == WAITS
            && w[1] == 0
            && w[4] == 0
            && w[2] + w[3] == WAITS
            && w[5] == w[0].wrapping_sub(w[2])
            && w[10] == w[11]
            && w[10] >= LOCKED
            && w[12] == w[13]
            && w[12] >= LOCKED
            && w[14] == 0
            && w[15] == GATED
            && w[16] > 0
            && blocked == in_entries;
        let overlapped = vcpus == 1
            || (w[6..10].iter().all(|&o| o > SEMAPHORE_OVERLAP_FLOOR)
                && out.runs_by_vcpu.len() == 2
                && out.runs_by_vcpu[1] > 0);
        if out.stop != (Stop::ExitProcess { code: 0 }) || stdout != "done\n" || !whole || !overlapped {
            failed.push(format!(
                "{vcpus} vCPU(s) run {run}: stop {:?}, output {stdout:?}, {}",
                out.stop,
                counts.join(", ")
            ));
        }
    }
    assert!(
        failed.is_empty(),
        "semaphore_units: {} of 21 runs failed (one on one vCPU, 20 on two): {failed:#?}",
        failed.len()
    );
}

/// Sprint 350's amendment (found by builder-350's reading of `sync_objects.c`): `probes/wait_closed_handle`. A worker
/// waits 1000 ms on a semaphore; the main thread closes its only handle, makes a new semaphore, releases it once and
/// waits on it with a zero timeout. The main thread's row, which Windows defines, is held on one vCPU and on two to the
/// laptop's (run lap-20261127T225150596Z-62b931c2, two runs alike, its binary the laptop's build of the probe's one
/// source; from Sprint 374, the runner's run 45797800625 equal to it by `cmp`), field by field, each unlike field named;
/// w2m runs the laptop's binary. The worker's row is
/// recorded and not compared, Microsoft Learn calling a wait on a handle closed meanwhile undefined (the worker
/// timed out, 258, on the runner and on the laptop, as w2m's now does, its entry holding the closed semaphore). Before
/// the fix the closed semaphore
/// was freed under the worker's waiter entry, the new one took its index, the release went to the stale entry, and the
/// main thread's wait answered 258 on one vCPU and on two.
#[test]
fn a_semaphore_made_after_a_waited_handle_is_closed_keeps_its_unit_on_one_and_two_vcpus() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let run = root.join("store/probe-runs/lap-20261127T225150596Z-62b931c2");
    let row_of = |body: &str, name: &str| -> serde_json::Map<String, serde_json::Value> {
        let line = body
            .lines()
            .find(|l| l.starts_with(&format!("{{\"s\":\"{name}\",")))
            .unwrap_or_else(|| panic!("no {name} row in {body}"));
        match serde_json::from_str(line).unwrap() {
            serde_json::Value::Object(m) => m,
            other => panic!("a row not an object: {other}"),
        }
    };
    let windows = fs::read_to_string(run.join("body-1.jsonl")).unwrap();
    let expected = row_of(&windows, "main");
    for i in (2..).take_while(|i| run.join(format!("body-{i}.jsonl")).is_file()) {
        let body = fs::read_to_string(run.join(format!("body-{i}.jsonl"))).unwrap();
        assert_eq!(
            row_of(&body, "main"),
            expected,
            "the Windows run's body-{i} main row against body-1's"
        );
    }
    let binary = fs::read_to_string(run.join("probe.sha256")).unwrap();
    let program = root.join(format!("store/binaries/{}.exe", binary.trim()));
    for vcpus in [1, 2] {
        let (stopped, said, stderr) = program_through_w2m_vm("wait_closed_handle", &program, &[], vcpus);
        eprintln!("wait_closed_handle on {vcpus} vCPUs:\n{said}");
        assert_eq!(
            stopped, "ExitProcess { code: 0 }",
            "wait_closed_handle on {vcpus} vCPUs; its rows {said}; stderr {stderr}"
        );
        let names: Vec<&str> = said.lines().map(|l| l.split('"').nth(3).unwrap_or("?")).collect();
        assert_eq!(names, ["main", "worker"], "wait_closed_handle's rows on {vcpus} vCPUs");
        let seen = row_of(&said, "main");
        let unlike: Vec<String> = expected
            .iter()
            .filter(|(k, v)| seen.get(*k) != Some(v))
            .map(|(k, v)| {
                let had = seen.get(k).map_or("none".into(), |x| x.to_string());
                format!("main.{k}: {had} against {v}")
            })
            .collect();
        assert!(
            unlike.is_empty(),
            "wait_closed_handle on {vcpus} vCPUs, fields unlike Windows' {unlike:?}"
        );
        assert_eq!(
            seen, expected,
            "wait_closed_handle's main row on {vcpus} vCPUs as on Windows"
        );
    }
}

/// Sprint 352 (card 2 of `docs/design-handles-and-sync-objects-2026-11-26.md`): `probes/wait_closed_kinds`. The rows
/// Windows defines, held on one vCPU and on two to the laptop's (run lap-20261127T221622950Z-62b931c2-007, two runs,
/// those rows alike; from Sprint 374, the runner's run 45813040124 equal to it by `cmp`),
/// field by field, each unlike field named: WaitForMultipleObjects of a closed handle beside a set event, for any and
/// for all, its return, last error and the event left set (`any_closed`, `all_closed`); the main thread's calls
/// (`main`); and 600 rounds over every caller of the object lookup (`cycles`), more than the 256 objects, so a lookup's
/// reference left undropped stops the run at a create. The rows Microsoft Learn leaves undefined, a manual-reset event's
/// handle closed during a wait and then set through a duplicate (`manual_worker`), and a wait for any of a closed and
/// an open event, the open one then set (`any_worker`), are recorded and held to the design's range: the outcome the
/// wait would have had, WAIT_OBJECT_0 (the set event's index) at the set, or its timeout. Before the change the run
/// stopped at the first row, "WaitForMultipleObjects argument form 0x2004" (`sync_objects.c` 461).
#[test]
fn the_waits_keep_their_objects_and_a_closed_handle_fails_wait_for_multiple_objects_on_one_and_two_vcpus() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let run = root.join("store/probe-runs/lap-20261127T221622950Z-62b931c2-007");
    let row_of = |body: &str, name: &str| -> serde_json::Map<String, serde_json::Value> {
        let line = body
            .lines()
            .find(|l| l.starts_with(&format!("{{\"s\":\"{name}\",")))
            .unwrap_or_else(|| panic!("no {name} row in {body}"));
        match serde_json::from_str(line).unwrap() {
            serde_json::Value::Object(m) => m,
            other => panic!("a row not an object: {other}"),
        }
    };
    let defined = ["any_closed", "all_closed", "main", "cycles"];
    let windows = fs::read_to_string(run.join("body-1.jsonl")).unwrap();
    eprintln!("wait_closed_kinds on Windows:\n{windows}");
    for i in (2..).take_while(|i| run.join(format!("body-{i}.jsonl")).is_file()) {
        let body = fs::read_to_string(run.join(format!("body-{i}.jsonl"))).unwrap();
        for name in defined {
            assert_eq!(
                row_of(&body, name),
                row_of(&windows, name),
                "the Windows run's body-{i} {name} row against body-1's"
            );
        }
    }
    let binary = fs::read_to_string(run.join("probe.sha256")).unwrap();
    let program = root.join(format!("store/binaries/{}.exe", binary.trim()));
    for vcpus in [1, 2] {
        let (stopped, said, stderr) = program_through_w2m_vm("wait_closed_kinds", &program, &[], vcpus);
        eprintln!("wait_closed_kinds on {vcpus} vCPUs:\n{said}");
        assert_eq!(
            stopped, "ExitProcess { code: 0 }",
            "wait_closed_kinds on {vcpus} vCPUs; its rows {said}; stderr {stderr}"
        );
        let names: Vec<&str> = said.lines().map(|l| l.split('"').nth(3).unwrap_or("?")).collect();
        assert_eq!(
            names,
            [
                "any_closed",
                "all_closed",
                "main",
                "manual_worker",
                "any_worker",
                "cycles"
            ],
            "wait_closed_kinds's rows on {vcpus} vCPUs"
        );
        let mut unlike = Vec::new();
        for name in defined {
            let (expected, seen) = (row_of(&windows, name), row_of(&said, name));
            for (k, v) in &expected {
                if seen.get(k) != Some(v) {
                    let had = seen.get(k).map_or("none".into(), |x| x.to_string());
                    unlike.push(format!("{name}.{k}: {had} against {v}"));
                }
            }
            if seen.len() != expected.len() {
                unlike.push(format!("{name}: {} fields against {}", seen.len(), expected.len()));
            }
        }
        assert!(
            unlike.is_empty(),
            "wait_closed_kinds on {vcpus} vCPUs, fields unlike Windows' {unlike:?}"
        );
        for (name, set) in [("manual_worker", 0), ("any_worker", 1)] {
            let wait = row_of(&said, name)["wait"].as_u64();
            assert!(
                wait == Some(set) || wait == Some(258),
                "wait_closed_kinds on {vcpus} vCPUs, {name}.wait {wait:?} outside the design's range \
                 [{set}, 258]: the set's index or the timeout"
            );
        }
    }
}

/// Sprint 356's amendment (reviewer-4's twenty-sixth review, Medium 1): `probes/mutex_abandoned`, waits on a mutex
/// whose owner thread returned holding it. Windows run lap-20261127T221622950Z-62b931c2-008 (the laptop, Windows 11
/// client 10.0.26200.9974, a 13th Gen Intel Core i5-13420H, two runs alike; from Sprint 374, equal by `cmp` to runner
/// run 45871497648, Server 2025 10.0.26100.33828 on an Intel Xeon Platinum 8370C, three runs alike) answers
/// WAIT_ABANDONED (128) to the first wait and 0 to the owner's recursive wait
/// and to a fresh wait after two releases; 129, WAIT_ABANDONED_0 + 1, to a wait for any whose abandoned mutex is at
/// index 1; 128, WAIT_ABANDONED_0, to a wait for all of a set event and an abandoned mutex at index 1; 128 to a worker
/// blocked with no timeout when the holder ends, the worker ending inside 5000 ms; and FALSE with 288 to a non-owner's
/// ReleaseMutex. Before the amendment w2m never released such a mutex: 258 to every wait, the blocked worker never
/// returning. Every row is defined; each is held on one vCPU and on two to the laptop's, field by field, each unlike
/// field named, and the laptop's two bodies to each other.
#[test]
fn a_mutex_whose_owner_ends_answers_wait_abandoned_as_windows_on_one_and_two_vcpus() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let run = root.join("store/probe-runs/lap-20261127T221622950Z-62b931c2-008");
    let row_of = |body: &str, name: &str| -> serde_json::Map<String, serde_json::Value> {
        let line = body
            .lines()
            .find(|l| l.starts_with(&format!("{{\"s\":\"{name}\",")))
            .unwrap_or_else(|| panic!("no {name} row in {body}"));
        match serde_json::from_str(line).unwrap() {
            serde_json::Value::Object(m) => m,
            other => panic!("a row not an object: {other}"),
        }
    };
    let rows = ["single", "any", "all", "blocked", "not_owner"];
    let windows = fs::read_to_string(run.join("body-1.jsonl")).unwrap();
    eprintln!("mutex_abandoned on Windows:\n{windows}");
    for i in (2..).take_while(|i| run.join(format!("body-{i}.jsonl")).is_file()) {
        let body = fs::read_to_string(run.join(format!("body-{i}.jsonl"))).unwrap();
        for name in rows {
            assert_eq!(
                row_of(&body, name),
                row_of(&windows, name),
                "the Windows run's body-{i} {name} row against body-1's"
            );
        }
    }
    let binary = fs::read_to_string(run.join("probe.sha256")).unwrap();
    let program = root.join(format!("store/binaries/{}.exe", binary.trim()));
    let mut unlike = Vec::new();
    for vcpus in [1, 2] {
        let (stopped, said, stderr) = program_through_w2m_vm("mutex_abandoned", &program, &[], vcpus);
        eprintln!("mutex_abandoned on {vcpus} vCPUs:\n{said}");
        assert_eq!(
            stopped, "ExitProcess { code: 0 }",
            "mutex_abandoned on {vcpus} vCPUs; its rows {said}; stderr {stderr}"
        );
        let names: Vec<&str> = said.lines().map(|l| l.split('"').nth(3).unwrap_or("?")).collect();
        assert_eq!(names, rows, "mutex_abandoned's rows on {vcpus} vCPUs");
        for name in rows {
            let (expected, seen) = (row_of(&windows, name), row_of(&said, name));
            for (k, v) in &expected {
                if seen.get(k) != Some(v) {
                    let had = seen.get(k).map_or("none".into(), |x| x.to_string());
                    unlike.push(format!("{vcpus} vCPU(s) {name}.{k}: {had} against {v}"));
                }
            }
            if seen.len() != expected.len() {
                unlike.push(format!(
                    "{vcpus} vCPU(s) {name}: {} fields against {}",
                    seen.len(),
                    expected.len()
                ));
            }
        }
    }
    assert!(unlike.is_empty(), "mutex_abandoned, fields unlike Windows': {unlike:?}");
}

/// Deferred (Sprint 329), met at Sprint 330: the single-process guests no other test runs on two vCPUs, through
/// `w2m-vm`, each held to its stop and output in `store/batch_run/first-stop-2026-10-28.tsv` (`qpc2`'s clock reading by
/// its form, as `batch_run.rs` holds it).
#[test]
fn the_other_single_process_guests_end_on_two_vcpus_as_held() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let held = fs::read_to_string(root().join("store/batch_run/first-stop-2026-10-28.tsv")).unwrap();
    for name in ["model_steps", "mutual", "mutual_context", "qpc2", "readied"] {
        let row: Vec<&str> = held
            .lines()
            .map(|l| l.split('\t').collect::<Vec<_>>())
            .find(|r| r.len() >= 5 && r[0] == "guest" && r[1] == name)
            .unwrap_or_else(|| panic!("no held row for {name}"));
        let (stopped, said, stderr) = through_w2m_vm(name, 2);
        assert_eq!(stopped, row[2], "{name} on two vCPUs; stderr {stderr}");
        if name == "qpc2" {
            let reading = said.strip_suffix('\n').unwrap_or("");
            assert!(
                reading.len() == 16 && reading.chars().all(|c| c.is_ascii_hexdigit()),
                "qpc2's output on two vCPUs: {said:?}"
            );
        } else {
            assert_eq!(said, row[4].replace("\\n", "\n"), "{name}'s output on two vCPUs");
        }
    }
}

/// Sprint 341 (the Decision "Card 341's form"): `w2m_logged` (`tools/guest_runtime/runtime.c`), which every logged
/// service returns through, writes the process-wide call ring exactly and within it from two vCPUs at once. Every bridge
/// takes the runtime word around its service (`guest::bridge`), so no guest's calls reach `w2m_logged` from two vCPUs
/// together; this drives the runtime image's own `w2m_logged` from a loop on each of two vCPUs, without the word, as a
/// bridge of lock class none (card 342) will call it. In each round the two loops meet at an arrival word, then call.
/// Round 0 starts from the ring as a process starts it and makes `CALLS_EACH` calls a loop, together more than the
/// ring's 10,751 records, so the capacity is crossed while both run. A call written past the ring needs the crossing
/// to fall inside one call's check and stores, which 2,000 such rounds never showed with the check made on a reloaded
/// `count` (plant 1); so each later round starts the ring one record short of its capacity, as a process past that
/// many calls has it (the calls before counted under `PRIOR_API`), and makes `CALLS_NEAR` calls a loop, the two loops'
/// first calls racing for the last record. Each round holds `count` to the calls before and made, `per_api`'s total to
/// `count`, and every byte of the exception area past the ring (`EXCEPTION_OFF`, 0x1000 bytes) to its value before the
/// rounds; after the rounds, each record a round wrote below the capacity is one call's whole (its api, its loop's
/// remaining count, its loop's tag), no call twice.
#[test]
fn the_call_ring_stays_exact_and_within_itself_on_two_vcpus() {
    use a64_encoder as a64;
    use guest_runtime::layout::{EXCEPTION_OFF, RING_OFF, RING_SIZE};
    use hypervisor::{Exit, Reg};
    use pe_loader::Memory;
    const DRIVER: u64 = MONITOR_VA + 0x2400; // the monitor's free area past the flush stub (`FLUSH_STUB_END`)
    const ARRIVAL: u64 = 0x3000_0000;
    const STACK: u64 = 0x4000_0000;
    const CALLS_EACH: u64 = 8_000;
    const CALLS_NEAR: u64 = 4;
    const ROUNDS: u64 = 20_000;
    const PRIOR_API: u64 = 0x100;
    const APIS: [u64; 2] = [0x101, 0x102]; // below W2M_RING_APIS (512), so per_api counts them
    const TAGS: [u64; 2] = [0xA000_0000_0000_0001, 0xA000_0000_0000_0002];
    const DONE: u16 = 0x77;
    const HEADER: u64 = 16 + 8 * 512; // count, capacity, per_api (`runtime.h` 89-94)
    let calls_each = |round: u64| if round == 0 { CALLS_EACH } else { CALLS_NEAR };
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let mut mem = Memory::default();
    mem.map(ARRIVAL, 0x1000, "arrival");
    mem.map(STACK, 0x10000, "stack");
    let loaded = Loaded::bare(mem, ARRIVAL, STACK + 0x10000);
    let mut guest = guest_with(&root, &loaded, &Start::console("ring.exe"));
    // each loop: x19 its calls left, x20 its api, x21 its tag, x22 `w2m_logged`, x23 the arrival word, x24 the vCPUs,
    // x25 its spins; it adds itself to the arrival word, waits for both, spins x25 times, then calls
    // w2m_logged(api, calls left, tag) to zero. The spins differ round by round and loop by loop: the last to arrive
    // leaves the wait first, ahead by more than a call's check and stores, and the spins move the two calls' start
    // across that window
    let driver = [
        a64::ldaxr(8, 9, 23),
        a64::addi(9, 9, 1),
        a64::stlxr(8, 10, 9, 23),
        a64::cbnz(10, -12),
        a64::ldr(8, 9, 23, 0),
        a64::sub(9, 9, 24),
        a64::cbnz(9, -8),
        a64::cbz(25, 12),
        a64::subi(25, 25, 1),
        a64::cbnz(25, -4),
        a64::mov(0, 20),
        a64::mov(1, 19),
        a64::mov(2, 21),
        a64::blr(22),
        a64::subi(19, 19, 1),
        a64::cbnz(19, -20),
        a64::hvc(DONE.into()),
    ];
    let bytes: Vec<u8> = driver.iter().flat_map(|w| w.to_le_bytes()).collect();
    let at = guest.offset(DRIVER).unwrap() as usize;
    guest.memory.as_mut_slice()[at..at + bytes.len()].copy_from_slice(&bytes);
    let logged = guest.symbols["w2m_logged"];
    let (ring, exception, arrival) = (
        guest.offset(PROCESS_VA + RING_OFF).unwrap(),
        guest.offset(PROCESS_VA + EXCEPTION_OFF).unwrap(),
        guest.offset(ARRIVAL).unwrap(),
    );
    let view = guest.memory_view();
    let capacity = view.read_u64(ring + 8);
    assert_eq!(capacity, (RING_SIZE - HEADER) / 24, "the ring's capacity");
    assert!(2 * CALLS_EACH > capacity, "the rounds' calls do not pass the capacity");
    let mut exception_before = vec![0u8; 0x1000];
    view.read(exception, &mut exception_before);
    let vm = Vm::new(40).unwrap();
    vm.map(&guest.memory, IPA_BASE, memory::READ | memory::WRITE | memory::EXEC)
        .unwrap();
    let ttbr = guest.ttbr;
    // the round's failure, which ends the rounds; and the first records that are not one call's whole
    let (mut failure, mut records_wrong): (Option<String>, Vec<String>) = (None, Vec::new());
    // each vCPU is made once and runs every round, the rounds marked by `gate` (the two vCPU threads and this one):
    // the ring reset here, a wait, both loops run to DONE, a wait, the ring judged here; a failure is held, not
    // panicked on, so that no thread is left at the gate, and stops the rounds
    let gate = std::sync::Barrier::new(3);
    let stop = std::sync::atomic::AtomicBool::new(false);
    let exits_wrong = std::sync::Mutex::new(Vec::<String>::new());
    let mut rounds = 0;
    std::thread::scope(|s| {
        for k in 0..2 {
            let (vm, gate, stop, exits_wrong) = (&vm, &gate, &stop, &exits_wrong);
            s.spawn(move || {
                let vcpu = Vcpu::new(vm).unwrap();
                w2m_vm::monitor::set_system_registers(&vcpu, ttbr).unwrap();
                for round in 0.. {
                    gate.wait();
                    if stop.load(std::sync::atomic::Ordering::Acquire) {
                        return;
                    }
                    for (r, v) in [
                        (19, calls_each(round)),
                        (20, APIS[k]),
                        (21, TAGS[k]),
                        (22, logged),
                        (23, ARRIVAL),
                        (24, 2),
                        (25, (round * [37, 61][k]) % 1024),
                        (28, MONITOR_VA + MONITOR_DATA),
                    ] {
                        vcpu.set_reg(Reg::X(r), v).unwrap();
                    }
                    // no stack: `w2m_logged` is a leaf that stores nothing to one, before and after (its disassembly)
                    vcpu.set_reg(Reg::Cpsr, w2m_vm::CPSR_EL1H).unwrap();
                    vcpu.set_reg(Reg::Pc, DRIVER).unwrap();
                    loop {
                        match vcpu.run().unwrap() {
                            Exit::VtimerActivated => vcpu.set_vtimer_mask(true).unwrap(),
                            e if e.hvc_immediate() == Some(DONE) => break,
                            e => {
                                let pc = vcpu.reg(Reg::Pc).unwrap();
                                exits_wrong.lock().unwrap().push(format!("vCPU {k}: {e:?} at {pc:#x}"));
                                break;
                            }
                        }
                    }
                    gate.wait();
                }
            });
        }
        for round in 0..ROUNDS {
            // the ring as the process starts it (`process.rs` 395-396), its records cleared; after round 0, as a
            // process past `before` calls has it
            let before = if round == 0 { 0 } else { capacity - 1 };
            view.write(ring, &vec![0u8; RING_SIZE as usize]);
            view.write_u64(ring, before);
            view.write_u64(ring + 8, capacity);
            view.write_u64(ring + 16 + 8 * PRIOR_API, before);
            view.write_u64(arrival, 0);
            gate.wait();
            gate.wait();
            rounds += 1;
            failure = (|| {
                if let Some(e) = exits_wrong.lock().unwrap().first() {
                    return Some(format!("round {round}: {e}, not the loop's hvc #{DONE:#x}"));
                }
                let count = view.read_u64(ring);
                let calls = 2 * calls_each(round);
                if count != before + calls {
                    return Some(format!(
                        "round {round}: the ring's count {count} against the {before} calls before and {calls} logged \
                         calls made"
                    ));
                }
                let total: u64 = (0..512).map(|a| view.read_u64(ring + 16 + 8 * a)).sum();
                if total != count {
                    return Some(format!("round {round}: per_api's total {total} against count {count}"));
                }
                let mut after = vec![0u8; 0x1000];
                view.read(exception, &mut after);
                let i = (0..0x1000).find(|&i| after[i] != exception_before[i])?;
                Some(format!(
                    "round {round}: the exception area changed, first at EXCEPTION_OFF + {i:#x}: {:#04x} to {:#04x}",
                    exception_before[i], after[i]
                ))
            })();
            let mut seen = BTreeSet::new();
            for i in before..capacity {
                let at = ring + HEADER + 24 * i;
                let (api, arg0, ret) = (view.read_u64(at), view.read_u64(at + 8), view.read_u64(at + 16));
                let whole = (0..2).any(|k| api == APIS[k] && ret == TAGS[k] && (1..=calls_each(round)).contains(&arg0));
                if (!whole || !seen.insert((api, arg0))) && records_wrong.len() < 4 {
                    records_wrong.push(format!("round {round}, record {i}: ({api:#x}, {arg0}, {ret:#x})"));
                }
            }
            if failure.is_some() {
                break;
            }
        }
        stop.store(true, std::sync::atomic::Ordering::Release);
        gate.wait();
    });
    if let Some(f) = failure {
        panic!("{f}");
    }
    assert!(
        records_wrong.is_empty(),
        "records not one call's whole: {records_wrong:?}"
    );
    eprintln!(
        "the call ring on two vCPUs: round 0's {} logged calls from an empty ring, count and per_api total {}; {} \
         rounds of {} from {} calls before, each count and total {}; capacity {capacity}, the exception area unchanged",
        2 * CALLS_EACH,
        2 * CALLS_EACH,
        rounds - 1,
        2 * CALLS_NEAR,
        capacity - 1,
        capacity - 1 + 2 * CALLS_NEAR
    );
}

/// Sprint 354 (card 3 of `docs/design-handles-and-sync-objects-2026-11-26.md`, "The wait across two locks"): a readying
/// that finds a thread running its wait on another vCPU, not yet BLOCKED, is kept by the woken mark, and that thread's
/// `w2m_block` leaves it READY with the mark clear, so the switch retries its call. Card 5 opens the window, a wait's
/// check of its object under the sync objects' lock and its park under the runtime word, which no run reaches before
/// it (the design, 356-357); so this drives the paths directly, in the form of Sprint 341's ring judge: each row
/// readies thread T on the path it names while T is held by vCPU owner 2 and READY, then runs the image's own
/// `w2m_block` as T on that vCPU, and reads T's status and mark. Two threads, R (index 0, the readier, on vCPU owner 1)
/// and T (index 1); the paths, the class `threads.c` and the host keep: the C runtime's `w2m_ready_thread` (a release's
/// grant) and `w2m_signal` (a manual-reset event's set, among others), the host's `wake_blocked` (a kernel answer's
/// wake) and `schedule`'s SCHEDULE_END (a thread's end); and a row with no readying, T then BLOCKED. Sprint 356 (Sprint
/// 354's Deferred (a)): T's wait arms the mark at its check (`w2m_wait_armed`, run as T on vCPU owner 2), and each path
/// runs again with T unarmed, running guest code there: the C's two then leave no mark and T's next park blocks it, the
/// spare retry gone; the host's two still mark every thread another vCPU holds (`threads.rs` `mark_woken`).
#[test]
fn a_readying_before_the_block_leaves_the_thread_ready() {
    use a64_encoder as a64;
    use guest_runtime::layout::{CURRENT, CURRENT_ENTRY, RT_ENTRIES, RUN_ENTRY, TEB_OFF};
    use hypervisor::{Exit, Reg};
    use pe_loader::Memory;
    use w2m_vm::threads::{BLOCKED, E_STATUS, E_VCPU, E_WOKEN, READY, RT_BLOCKED, RT_WAITING, SCHEDULE_END, Thread};
    const DRIVER: u64 = MONITOR_VA + 0x2400; // the monitor's free area past the flush stub (`FLUSH_STUB_END`)
    const STACK: u64 = 0x4000_0000;
    const T_TEB: u64 = 0x5000_0000;
    const DONE: u16 = 0x77;
    const KEY: u64 = 0x2468; // T's wait's key, an object's address in a run
    const SCRATCH_OFF: u64 = 0x1900; // runtime.h W2M_THREAD_SCRATCH_OFF
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let mut mem = Memory::default();
    mem.map(STACK, 0x10000, "stack");
    mem.map(T_TEB, 0x4000, "T's TEB");
    let loaded = Loaded::bare(mem, STACK, STACK + 0x10000);
    let mut guest = guest_with(&root, &loaded, &Start::console("woken.exe"));
    let (r_tid, t_tid) = (4u64, 8u64);
    for (tid, teb) in [(r_tid, PROCESS_VA + TEB_OFF), (t_tid, T_TEB)] {
        w2m_vm::threads::add_thread(
            &mut guest,
            Thread {
                tid,
                index: 0,
                teb,
                stack: (0, 0, 0),
                exit_code: w2m_vm::threads::STILL_ACTIVE,
                parent: 0,
                start: 0,
                created_suspended: false,
            },
        )
        .unwrap();
    }
    // the driver: sp from x15, the function in x22 called with x0 and x1, then the driver's own hvc
    let driver = [a64::addi(31, 15, 0), a64::blr(22), a64::hvc(DONE.into())];
    let bytes: Vec<u8> = driver.iter().flat_map(|w| w.to_le_bytes()).collect();
    let at = guest.offset(DRIVER).unwrap() as usize;
    guest.memory.as_mut_slice()[at..at + bytes.len()].copy_from_slice(&bytes);
    let (ready_thread, signal, block, armed) = (
        guest.symbols["w2m_ready_thread"],
        guest.symbols["w2m_signal"],
        guest.symbols["w2m_block"],
        guest.symbols["w2m_wait_armed"],
    );
    let monitor = guest.served.vcpu.monitor;
    let entry = |index: u64| RUN_TABLE + RT_ENTRIES + RUN_ENTRY * index;
    let gs_base = guest.offset(PROCESS_VA + STATE_OFF + 0xb0).unwrap();
    let t_scratch = guest.offset(T_TEB + SCRATCH_OFF).unwrap();
    let vm = Vm::new(40).unwrap();
    vm.map(&guest.memory, IPA_BASE, memory::READ | memory::WRITE | memory::EXEC)
        .unwrap();
    let vcpu = Vcpu::new(&vm).unwrap();
    w2m_vm::monitor::set_system_registers(&vcpu, guest.ttbr).unwrap();
    // one call of the image's `function` on this vCPU, as vCPU owner `owner` running thread `index` (its TEB `teb`)
    let call = |guest: &Guest, owner: u64, index: u64, teb: u64, function: u64, args: [u64; 2]| {
        let view = guest.memory_view();
        view.write_u64(monitor + LOCK_OWNER, owner);
        view.write_u64(monitor + CURRENT, index);
        view.write_u64(monitor + CURRENT_ENTRY, MONITOR_VA + entry(index));
        view.write_u64(gs_base, teb);
        for (r, v) in [
            (0, args[0]),
            (1, args[1]),
            (15, STACK + 0x10000),
            (22, function),
            (28, MONITOR_VA + MONITOR_DATA),
        ] {
            vcpu.set_reg(Reg::X(r), v).unwrap();
        }
        vcpu.set_reg(Reg::Cpsr, w2m_vm::CPSR_EL1H).unwrap();
        vcpu.set_reg(Reg::Pc, DRIVER).unwrap();
        loop {
            match vcpu.run().unwrap() {
                Exit::VtimerActivated => vcpu.set_vtimer_mask(true).unwrap(),
                e if e.hvc_immediate() == Some(DONE) => return,
                e => panic!("{e:?} at {:#x} in a call of {function:#x}", vcpu.reg(Reg::Pc).unwrap()),
            }
        }
    };
    let mut rows = Vec::new();
    let paths = ["none", "w2m_ready_thread", "w2m_signal", "wake_blocked", "SCHEDULE_END"];
    for (path, arm) in paths
        .iter()
        .map(|&p| (p, true))
        .chain(paths.iter().map(|&p| (p, false)))
    {
        // R current and running on vCPU owner 1; T READY and running on vCPU owner 2, its wait's check made (armed) or
        // running guest code (not); no thread blocked, no vCPU waiting
        let view = guest.memory_view();
        for (index, owner) in [(0, 1), (1, 2)] {
            view.write_u64(entry(index) + E_STATUS, READY);
            view.write_u64(entry(index) + E_VCPU, owner);
            view.write_u64(entry(index) + E_WOKEN, 0);
        }
        view.write_u64(RUN_TABLE + RT_BLOCKED, 0);
        view.write_u64(RUN_TABLE + RT_WAITING, 0);
        view.write_u64(t_scratch, 0);
        view.write_u64(t_scratch + 0x30, 0);
        if arm {
            call(&guest, 2, 1, T_TEB, armed, [0, 0]);
        }
        let view = guest.memory_view();
        view.write_u64(monitor + LOCK_OWNER, 1);
        view.write_u64(monitor + CURRENT, 0);
        view.write_u64(monitor + CURRENT_ENTRY, MONITOR_VA + entry(0));
        match path {
            "w2m_ready_thread" => call(&guest, 1, 0, PROCESS_VA + TEB_OFF, ready_thread, [t_tid, 0]),
            "w2m_signal" => call(&guest, 1, 0, PROCESS_VA + TEB_OFF, signal, [KEY, 0]),
            "wake_blocked" => w2m_vm::host_request::wake_blocked(&mut guest),
            "SCHEDULE_END" => {
                let view = guest.memory_view();
                assert!(w2m_vm::monitor_code::host_take_runtime_word(view, 1));
                let mut x = [0u64; 31];
                w2m_vm::threads::schedule(&mut guest, &mut x, SCHEDULE_END).unwrap();
                w2m_vm::monitor_code::host_release_runtime_word(guest.memory_view());
            }
            _ => {}
        }
        let marked = guest.memory_view().read_u64(entry(1) + E_WOKEN);
        // T's wait parks: `w2m_block` as T on vCPU owner 2, no deadline, its key
        call(&guest, 2, 1, T_TEB, block, [u64::MAX, KEY]);
        let view = guest.memory_view();
        let (status, woken) = (view.read_u64(entry(1) + E_STATUS), view.read_u64(entry(1) + E_WOKEN));
        let named = |s: u64| match s {
            READY => "READY".to_string(),
            BLOCKED => "BLOCKED".to_string(),
            s => format!("status {s}"),
        };
        rows.push((path, arm, marked, named(status), woken));
        eprintln!(
            "the woken mark, {path}, T {}: T's mark {marked} after the readying; after its w2m_block T {} with the mark \
             {woken}",
            if arm { "armed" } else { "not armed" },
            named(status)
        );
    }
    let wrong: Vec<String> = rows
        .iter()
        .filter(|(path, arm, _, status, woken)| {
            let c_path = *path == "w2m_ready_thread" || *path == "w2m_signal";
            let expected = if *path == "none" || (c_path && !arm) {
                "BLOCKED"
            } else {
                "READY"
            };
            status != expected || *woken != 0
        })
        .map(|(path, arm, marked, status, woken)| {
            format!("{path}, armed {arm}: mark {marked} after the readying, T {status}, mark {woken}")
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "a readying before T's block, T then READY with its mark clear (none, and the C's paths on T not armed: BLOCKED), \
         against {wrong:?}"
    );
}

/// Sprint 359 (its card's amendment; reviewer-4's thirty-first review, High 1): the runtime word's guard,
/// `W2M_HELD(runtime_word)` (`runtime.h`), lets the word go only when its take took it, and once. This drives the image's
/// own services, in the form of Sprint 341's ring judge, on one vCPU as vCPU owner 1, in the image a run builds and in
/// the timed one (`W2M_TIME_LOCKS`): (a) `DuplicateHandle` of a source that names no handle, whose look at the file
/// table and the standard handles holds the word by its guard (`sync_objects.c`), with the word held by this vCPU's
/// owner before the call, as a traced call's bridge holds it, and with it free; (b) `WaitForSingleObject` of a handle to
/// this process with no wait (`DuplicateHandle` of the pseudo-handle makes it), whose check holds the word by its guard
/// and lets it go early by `W2M_LET_GO_EARLY(runtime_word)` before `w2m_logged` (`wait_one_held`), the word held and
/// free as in (a). After a call made holding the word, the word is still this owner's; after a call made with it free,
/// it is free. In the timed image each release of a take the C made counts one in the runtime word's timing area
/// (`runtime.c` `word_released`, `monitor_code::LOCK_C_TAKINGS`): a call made holding the word counts none, (a) free
/// one, and (b) free one, where a second release at the guard's scope's end would count two.
#[test]
fn the_runtime_words_guard_lets_it_go_only_when_its_take_took_it_and_once() {
    use a64_encoder as a64;
    use guest_runtime::layout::TEB_OFF;
    use hypervisor::{Exit, Reg};
    use pe_loader::Memory;
    use std::sync::atomic::Ordering;
    use w2m_vm::monitor_code::{LOCK_C_TAKINGS, LOCK_TIMING};
    const DRIVER: u64 = MONITOR_VA + 0x2400; // the monitor's free area past the flush stub (`FLUSH_STUB_END`)
    const STACK: u64 = 0x4000_0000;
    const ARGS: u64 = STACK + 0x8000; // the call's RSP, its stack arguments at + 0x28 on
    const OUT: u64 = STACK + 0x9000; // DuplicateHandle's lpTargetHandle
    const DONE: u16 = 0x77;
    const OWNER: u64 = 1;
    const PSEUDO: u64 = !0; // GetCurrentProcess() (sync_objects.c CURRENT_PROCESS)
    const NO_HANDLE: u64 = 0x1234; // below the objects' handles, no file's and no standard handle
    const SAME_ACCESS: u32 = 2; // DUPLICATE_SAME_ACCESS
    const WAIT_TIMEOUT: u64 = 258;
    const ERROR_INVALID_HANDLE: u32 = 6;
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let mut rows = Vec::new();
    for timed in [false, true] {
        let mut mem = Memory::default();
        mem.map(STACK, 0x10000, "stack");
        let loaded = Loaded::bare(mem, STACK, STACK + 0x10000);
        let mut start = Start::console("word.exe");
        start.time_runtime_word = timed;
        let mut guest = guest_with_defines(&root, &loaded, &start, &w2m_vm::guest::runtime_defines_for(timed));
        // the driver: sp from x15, the function in x22 called with x0 to x4, then the driver's own hvc
        let driver = [a64::addi(31, 15, 0), a64::blr(22), a64::hvc(DONE.into())];
        let bytes: Vec<u8> = driver.iter().flat_map(|w| w.to_le_bytes()).collect();
        let at = guest.offset(DRIVER).unwrap() as usize;
        guest.memory.as_mut_slice()[at..at + bytes.len()].copy_from_slice(&bytes);
        let (duplicate, wait) = (
            guest.symbols["k32_DuplicateHandle"],
            guest.symbols["k32_WaitForSingleObject"],
        );
        let monitor = guest.served.vcpu.monitor;
        let gs_base = guest.offset(PROCESS_VA + STATE_OFF + 0xb0).unwrap();
        let teb = guest.offset(PROCESS_VA + TEB_OFF).unwrap();
        let (args, out) = (guest.offset(ARGS).unwrap(), guest.offset(OUT).unwrap());
        let takings = guest.offset(MONITOR_VA + LOCK_TIMING + LOCK_C_TAKINGS).unwrap();
        let vm = Vm::new(40).unwrap();
        vm.map(&guest.memory, IPA_BASE, memory::READ | memory::WRITE | memory::EXEC)
            .unwrap();
        let vcpu = Vcpu::new(&vm).unwrap();
        w2m_vm::monitor::set_system_registers(&vcpu, guest.ttbr).unwrap();
        let view = guest.memory_view();
        view.write_u64(monitor + LOCK_OWNER, OWNER);
        view.write_u64(gs_base, PROCESS_VA + TEB_OFF);
        view.write_u32(args + 0x30, 0); // bInheritHandle
        view.write_u32(args + 0x38, SAME_ACCESS); // dwOptions
        let word = view.atomic_u32(RUN_TABLE + RUNTIME_LOCK);
        // one call of the image's `function` on this vCPU, the word held by this owner before it or free: its answer,
        // the word after it, and the C's takings it counted (the timed image's)
        let call = |function: u64, regs: [u64; 5], held: bool| -> (u64, u32, u64) {
            word.store(if held { OWNER as u32 } else { 0 }, Ordering::Relaxed);
            let before = view.read_u64(takings);
            for (r, v) in regs.iter().enumerate() {
                vcpu.set_reg(Reg::X(r as u8), *v).unwrap();
            }
            for (r, v) in [(15, STACK + 0x10000), (22, function), (28, MONITOR_VA + MONITOR_DATA)] {
                vcpu.set_reg(Reg::X(r), v).unwrap();
            }
            vcpu.set_reg(Reg::Cpsr, w2m_vm::CPSR_EL1H).unwrap();
            vcpu.set_reg(Reg::Pc, DRIVER).unwrap();
            loop {
                match vcpu.run().unwrap() {
                    Exit::VtimerActivated => vcpu.set_vtimer_mask(true).unwrap(),
                    e if e.hvc_immediate() == Some(DONE) => break,
                    e => panic!("{e:?} at {:#x} in a call of {function:#x}", vcpu.reg(Reg::Pc).unwrap()),
                }
            }
            (
                vcpu.reg(Reg::X(0)).unwrap(),
                word.load(Ordering::Relaxed),
                view.read_u64(takings) - before,
            )
        };
        let image = if timed { "timed" } else { "untimed" };
        for held in [true, false] {
            let (answer, after, counted) = call(duplicate, [PSEUDO, NO_HANDLE, PSEUDO, OUT, ARGS], held);
            let failed = (answer, view.read_u64(out), view.read_u32(teb + 0x68));
            rows.push((
                image,
                "(a) DuplicateHandle",
                held,
                failed == (0, 0, ERROR_INVALID_HANDLE),
                after,
                counted,
            ));
            let (made, _, _) = call(duplicate, [PSEUDO, PSEUDO, PSEUDO, OUT, ARGS], false);
            let handle = view.read_u64(out);
            assert!(
                made == 1 && handle >= 0x2000,
                "{image}: DuplicateHandle of the process answered {made}, {handle:#x}"
            );
            let (answer, after, counted) = call(wait, [handle, 0, 0, 0, 0], held);
            rows.push((
                image,
                "(b) WaitForSingleObject",
                held,
                answer == WAIT_TIMEOUT,
                after,
                counted,
            ));
        }
    }
    let wrong: Vec<String> = rows
        .iter()
        .filter(|(image, _, held, answered, after, counted)| {
            let word_right = *after == if *held { OWNER as u32 } else { 0 };
            let count_right = *image == "untimed" || *counted == u64::from(!*held);
            !(*answered && word_right && count_right)
        })
        .map(|(image, call, held, answered, after, counted)| {
            format!(
                "{image} {call}, the word held before {held}: answered as Windows {answered}, the word {after} after, \
                     {counted} of the C's takings counted"
            )
        })
        .collect();
    for (image, call, held, answered, after, counted) in &rows {
        eprintln!("{image} {call}, held before {held}: answered {answered}, the word {after} after, {counted} counted");
    }
    assert!(
        wrong.is_empty(),
        "a call made holding the word leaves it this owner's and counts no taking; one made with it free leaves it free \
         and counts one; against {wrong:?}"
    );
}

/// Sprint 356 (Sprint 354's Deferred (b)): the C switch's clearing of the woken mark at a thread's load, judged alone,
/// in the form of `a_readying_before_the_block_leaves_the_thread_ready`. T (index 1) holds the mark WOKEN, as the host's
/// readying leaves it on a thread another vCPU ran guest code on, and is put down READY; R (index 0), current on vCPU
/// owner 1, calls the image's own `w2m_switch` with SCHEDULE_YIELD, which loads T there; then T's wait parks, the
/// image's `w2m_block` as T on that vCPU. T must have its mark clear after the load and be BLOCKED by the park; with the
/// clearing removed the mark survives the load and the park leaves T READY, its next wait passed over once.
#[test]
fn the_c_switch_clears_the_woken_mark_of_the_thread_it_loads() {
    use a64_encoder as a64;
    use guest_runtime::layout::{CURRENT, CURRENT_ENTRY, RT_ENTRIES, RUN_ENTRY, TEB_OFF};
    use hypervisor::{Exit, Reg};
    use pe_loader::Memory;
    use w2m_vm::threads::{BLOCKED, E_STATUS, E_VCPU, E_WOKEN, READY, RT_BLOCKED, RT_WAITING, Thread};
    const DRIVER: u64 = MONITOR_VA + 0x2400; // the monitor's free area past the flush stub (`FLUSH_STUB_END`)
    const STACK: u64 = 0x4000_0000;
    const T_TEB: u64 = 0x5000_0000;
    const DONE: u16 = 0x77;
    const KEY: u64 = 0x2468;
    const SCRATCH_OFF: u64 = 0x1900; // runtime.h W2M_THREAD_SCRATCH_OFF
    const SCHEDULE_YIELD: u64 = 1; // runtime.h W2M_SCHEDULE_YIELD
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let mut mem = Memory::default();
    mem.map(STACK, 0x10000, "stack");
    mem.map(T_TEB, 0x4000, "T's TEB");
    let loaded = Loaded::bare(mem, STACK, STACK + 0x10000);
    let mut guest = guest_with(&root, &loaded, &Start::console("woken.exe"));
    for (tid, teb) in [(4u64, PROCESS_VA + TEB_OFF), (8u64, T_TEB)] {
        w2m_vm::threads::add_thread(
            &mut guest,
            Thread {
                tid,
                index: 0,
                teb,
                stack: (0, 0, 0),
                exit_code: w2m_vm::threads::STILL_ACTIVE,
                parent: 0,
                start: 0,
                created_suspended: false,
            },
        )
        .unwrap();
    }
    let driver = [a64::addi(31, 15, 0), a64::blr(22), a64::hvc(DONE.into())];
    let bytes: Vec<u8> = driver.iter().flat_map(|w| w.to_le_bytes()).collect();
    let at = guest.offset(DRIVER).unwrap() as usize;
    guest.memory.as_mut_slice()[at..at + bytes.len()].copy_from_slice(&bytes);
    let (switch, block) = (guest.symbols["w2m_switch"], guest.symbols["w2m_block"]);
    let monitor = guest.served.vcpu.monitor;
    let entry = |index: u64| RUN_TABLE + RT_ENTRIES + RUN_ENTRY * index;
    let gs_base = guest.offset(PROCESS_VA + STATE_OFF + 0xb0).unwrap();
    let t_scratch = guest.offset(T_TEB + SCRATCH_OFF).unwrap();
    let vm = Vm::new(40).unwrap();
    vm.map(&guest.memory, IPA_BASE, memory::READ | memory::WRITE | memory::EXEC)
        .unwrap();
    let vcpu = Vcpu::new(&vm).unwrap();
    w2m_vm::monitor::set_system_registers(&vcpu, guest.ttbr).unwrap();
    // one call of the image's `function` on this vCPU, as vCPU owner 1 running thread `index` (its TEB `teb`); x0 its
    // answer
    let call = |guest: &Guest, index: u64, teb: u64, function: u64, args: [u64; 2]| {
        let view = guest.memory_view();
        view.write_u64(monitor + LOCK_OWNER, 1);
        view.write_u64(monitor + CURRENT, index);
        view.write_u64(monitor + CURRENT_ENTRY, MONITOR_VA + entry(index));
        view.write_u64(gs_base, teb);
        for (r, v) in [
            (0, args[0]),
            (1, args[1]),
            (15, STACK + 0x10000),
            (22, function),
            (28, MONITOR_VA + MONITOR_DATA),
        ] {
            vcpu.set_reg(Reg::X(r), v).unwrap();
        }
        vcpu.set_reg(Reg::Cpsr, w2m_vm::CPSR_EL1H).unwrap();
        vcpu.set_reg(Reg::Pc, DRIVER).unwrap();
        loop {
            match vcpu.run().unwrap() {
                Exit::VtimerActivated => vcpu.set_vtimer_mask(true).unwrap(),
                e if e.hvc_immediate() == Some(DONE) => return vcpu.reg(Reg::X(0)).unwrap(),
                e => panic!("{e:?} at {:#x} in a call of {function:#x}", vcpu.reg(Reg::Pc).unwrap()),
            }
        }
    };
    // R current and READY on vCPU owner 1; T READY, held by no vCPU, its mark WOKEN; the VM may switch
    let view = guest.memory_view();
    for (index, holder, woken) in [(0, 1, 0), (1, 0, 1)] {
        view.write_u64(entry(index) + E_STATUS, READY);
        view.write_u64(entry(index) + E_VCPU, holder);
        view.write_u64(entry(index) + E_WOKEN, woken);
    }
    view.write_u64(RUN_TABLE + RT_BLOCKED, 0);
    view.write_u64(RUN_TABLE + RT_WAITING, 0);
    view.write_u64(RUN_TABLE + u64::from(w2m_vm::guest::RT_IN_VM), 1);
    view.write_u64(t_scratch, 0);
    view.write_u64(t_scratch + 0x30, 0);
    let switched = call(&guest, 0, PROCESS_VA + TEB_OFF, switch, [SCHEDULE_YIELD, 0]);
    let view = guest.memory_view();
    let (current, holder, loaded_mark) = (
        view.read_u64(monitor + CURRENT),
        view.read_u64(entry(1) + E_VCPU),
        view.read_u64(entry(1) + E_WOKEN),
    );
    // T's wait parks on the vCPU that loaded it: `w2m_block` as T, no deadline, its key
    call(&guest, 1, T_TEB, block, [u64::MAX, KEY]);
    let view = guest.memory_view();
    let status = view.read_u64(entry(1) + E_STATUS);
    eprintln!(
        "the C switch's load: answer {switched}, current {current}, T held by {holder}, T's mark {loaded_mark} after \
         the load; after its w2m_block T's status {status} (READY {READY}, BLOCKED {BLOCKED})"
    );
    assert_eq!(
        (switched, current, holder, loaded_mark, status),
        (1, 1, 1, 0, BLOCKED),
        "the switch's answer, the current thread, T's holder, T's mark after its load, and T's status after its park"
    );
}

/// Sprint 343 (Change 1's heap class): the process heap's `HeapAlloc` and `HeapFree` (`tools/guest_runtime/heap.c`)
/// never hand one block to two vCPUs at once and keep the heap whole, called from two vCPUs together without the
/// runtime word, as their bridges of lock class heap call them. In the form of Sprint 341's ring judge, this drives the
/// image's own `k32_HeapAlloc` and `k32_HeapFree` from a loop on each of two vCPUs. Each round starts the heap as a
/// process starts it, `ready` cleared and the heap's first page zero, so the two loops race its one-time set-up too;
/// they meet at an arrival word, and each makes `PASSES` passes over its `SLOTS` slots: a slot's block has its loop's
/// mark checked at its second and last words and is freed, then a block of 16 to 264 bytes is taken, marked and held
/// there. A mark found wrong stops the loop at its own `hvc`. After each round the host walks the blocks the bump
/// pointer carved, from the heap's start to it, and holds every carved block to one holder: one vCPU's slot, its marks
/// that vCPU's, or the free list, a chain of carved blocks; and the bump pointer inside the arena and the lock free.
#[test]
fn the_process_heap_hands_no_block_to_two_vcpus_at_once() {
    use a64_encoder as a64;
    use a64_encoder::assembler::Assembler;
    use guest_runtime::layout::{HEAP_SIZE, HEAP_VA, PEB_OFF, SAVE_AREA};
    use hypervisor::{Exit, Reg};
    use pe_loader::Memory;
    use std::collections::BTreeMap;
    use std::sync::mpsc;
    const DRIVER: u64 = MONITOR_VA + 0x2400; // the monitor's free area past the flush stub (`FLUSH_STUB_END`)
    const ARRIVAL: u64 = 0x3000_0000;
    const STACK: u64 = 0x4000_0000;
    const SLOT_TABLES: u64 = 0x5000_0000; // vCPU k's at + 0x1000 * k: (block, size) a slot
    const SLOTS: u64 = 32;
    const PASSES: u64 = 64;
    const ROUNDS: u64 = 2_000;
    const MARKS: [u64; 2] = [0xB10C_0000_0000_0001, 0xB10C_0000_0000_0002];
    const DONE: u16 = 0x77;
    const MARK_WRONG: u16 = 0x78;
    const NOTHING_TAKEN: u16 = 0x79;
    const HEADER: u64 = 32; // `struct header` (`heap.c` 38-43)
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let root = root();
    let mut mem = Memory::default();
    mem.map(ARRIVAL, 0x1000, "arrival");
    mem.map(STACK, 0x10000, "stack"); // both loops' stacks, 0x8000 each
    mem.map(SLOT_TABLES, 0x2000, "slot tables");
    let loaded = Loaded::bare(mem, ARRIVAL, STACK + 0x10000);
    let mut guest = guest_with(&root, &loaded, &Start::console("heap.exe"));
    // each loop: sp from x15, x19 its passes left, x20 its slot table, x21 its mark, x22 `k32_HeapAlloc`, x23 the
    // arrival word, x24 the vCPUs, x25 its spins, x26 `k32_HeapFree`, x27 the slot; the AAPCS keeps x19-x28 across a
    // call. A slot's size is 16 + 8 * ((slot + passes left) mod 32), made again after each call
    let mut a = Assembler::new(DRIVER);
    let size_into = |a: &mut Assembler, rd: u32| {
        a.emit(&[a64::add(9, 27, 19, 0, 0), a64::ubfx(9, 9, 0, 5)]);
        a.emit(&a64::mov_imm(rd, 16));
        a.emit(&[a64::add(rd, rd, 9, 0, 3)]);
    };
    let (arrive, wait, spin, pass, slot, take, wrong, nothing) = (
        a.label(),
        a.label(),
        a.label(),
        a.label(),
        a.label(),
        a.label(),
        a.label(),
        a.label(),
    );
    a.emit(&[a64::addi(31, 15, 0)]);
    a.bind(arrive);
    a.emit(&[a64::ldaxr(8, 9, 23), a64::addi(9, 9, 1), a64::stlxr(8, 10, 9, 23)]);
    a.branch(|o| a64::cbnz(10, o), arrive);
    a.bind(wait);
    a.emit(&[a64::ldr(8, 9, 23, 0), a64::sub(9, 9, 24)]);
    a.branch(|o| a64::cbnz(9, o), wait);
    a.bind(spin);
    a.branch(|o| a64::cbz(25, o), pass);
    a.emit(&[a64::subi(25, 25, 1)]);
    a.branch(|o| a64::cbnz(25, o), spin);
    a.bind(pass);
    a.emit(&a64::mov_imm(27, 0));
    a.bind(slot);
    a.emit(&[a64::add(12, 20, 27, 0, 4), a64::ldr(8, 13, 12, 0)]);
    a.branch(|o| a64::cbz(13, o), take);
    a.emit(&[a64::ldr(8, 14, 12, 8), a64::ldr(8, 9, 13, 8), a64::cmp(9, 21)]);
    a.branch(|o| a64::bcond("ne", o), wrong);
    a.emit(&[
        a64::add(10, 13, 14, 0, 0),
        a64::subi(10, 10, 8),
        a64::ldr(8, 9, 10, 0),
        a64::cmp(9, 21),
    ]);
    a.branch(|o| a64::bcond("ne", o), wrong);
    a.emit(&a64::mov_imm(0, HEAP_VA));
    a.emit(&a64::mov_imm(1, 0));
    a.emit(&[a64::mov(2, 13), a64::blr(26)]);
    a.bind(take);
    size_into(&mut a, 2);
    a.emit(&a64::mov_imm(0, HEAP_VA));
    a.emit(&a64::mov_imm(1, 0));
    a.emit(&[a64::blr(22)]);
    a.branch(|o| a64::cbz(0, o), nothing);
    size_into(&mut a, 14);
    a.emit(&[
        a64::str(8, 21, 0, 8),
        a64::add(10, 0, 14, 0, 0),
        a64::subi(10, 10, 8),
        a64::str(8, 21, 10, 0),
        a64::add(12, 20, 27, 0, 4),
        a64::str(8, 0, 12, 0),
        a64::str(8, 14, 12, 8),
        a64::addi(27, 27, 1),
        a64::cmpi(27, SLOTS as u32),
    ]);
    a.branch(|o| a64::bcond("ne", o), slot);
    a.emit(&[a64::subi(19, 19, 1)]);
    a.branch(|o| a64::cbnz(19, o), pass);
    a.emit(&[a64::hvc(DONE.into())]);
    a.bind(wrong);
    a.emit(&[a64::hvc(MARK_WRONG.into())]);
    a.bind(nothing);
    a.emit(&[a64::hvc(NOTHING_TAKEN.into())]);
    let bytes: Vec<u8> = a.finish().iter().flat_map(|w| w.to_le_bytes()).collect();
    assert!(bytes.len() < 0xC00, "the driver fits the monitor's free area");
    let at = guest.offset(DRIVER).unwrap() as usize;
    guest.memory.as_mut_slice()[at..at + bytes.len()].copy_from_slice(&bytes);
    let (alloc, free, ready) = (
        guest.symbols["k32_HeapAlloc"],
        guest.symbols["k32_HeapFree"],
        guest.symbols["ready"],
    );
    let heap = guest.offset(HEAP_VA).unwrap();
    let arena = |va: u64| heap + (va - HEAP_VA);
    let (arrival, ready, tables) = (
        guest.offset(ARRIVAL).unwrap(),
        guest.offset(ready).unwrap(),
        guest.offset(SLOT_TABLES).unwrap(),
    );
    let save = guest.offset(MONITOR_VA + MONITOR_DATA + SAVE_AREA).unwrap();
    let view = guest.memory_view();
    // where the set-up starts the bump pointer (`heap.c` 66-69): past the ProcessParameters the arena's first pages hold
    let params = view.read_u64(guest.offset(PROCESS_VA + PEB_OFF + 0x20).unwrap());
    assert!(
        (HEAP_VA..HEAP_VA + HEAP_SIZE).contains(&params),
        "ProcessParameters in the arena"
    );
    let start = (params + u64::from(view.read_u32(arena(params))) + 15) & !15;
    let end = HEAP_VA + HEAP_SIZE;
    let vm = Vm::new(40).unwrap();
    vm.map(&guest.memory, IPA_BASE, memory::READ | memory::WRITE | memory::EXEC)
        .unwrap();
    let ttbr = guest.ttbr;
    // each vCPU is made once and runs every round: the heap reset here, a wait at `gate`, both loops run to their hvc
    // and say so on `said`, and the heap judged here. A round whose loops have not both said in 60 s has its vCPUs
    // stopped (a free list made a cycle leaves `take` searching it), and the rounds end at the first failure
    let gate = std::sync::Barrier::new(3);
    let stop = std::sync::atomic::AtomicBool::new(false);
    let handles = std::sync::Mutex::new(Vec::new());
    let (said_tx, said) = mpsc::channel::<(usize, Option<String>)>();
    let (mut failure, mut rounds, mut carved_most, mut free_most, mut next_most) = (None::<String>, 0, 0, 0, 0);
    std::thread::scope(|s| {
        for k in 0..2usize {
            let (vm, gate, stop, handles, said_tx, view) = (&vm, &gate, &stop, &handles, said_tx.clone(), &view);
            s.spawn(move || {
                let vcpu = Vcpu::new(vm).unwrap();
                w2m_vm::monitor::set_system_registers(&vcpu, ttbr).unwrap();
                handles.lock().unwrap().push(vcpu.handle());
                for round in 0u64.. {
                    gate.wait();
                    if stop.load(std::sync::atomic::Ordering::Acquire) {
                        return;
                    }
                    for (r, v) in [
                        (15, STACK + 0x8000 * (k as u64 + 1)),
                        (19, PASSES),
                        (20, SLOT_TABLES + 0x1000 * k as u64),
                        (21, MARKS[k]),
                        (22, alloc),
                        (23, ARRIVAL),
                        (24, 2),
                        (25, (round * [37, 61][k]) % 1024),
                        (26, free),
                        (28, MONITOR_VA + MONITOR_DATA),
                    ] {
                        vcpu.set_reg(Reg::X(r), v).unwrap();
                    }
                    vcpu.set_reg(Reg::Cpsr, w2m_vm::CPSR_EL1H).unwrap();
                    vcpu.set_reg(Reg::Pc, DRIVER).unwrap();
                    let wrong = loop {
                        let e = vcpu.run().unwrap();
                        let x = |r: u8| vcpu.reg(Reg::X(r)).unwrap();
                        match e {
                            Exit::VtimerActivated => vcpu.set_vtimer_mask(true).unwrap(),
                            e if e.hvc_immediate() == Some(DONE) => break None,
                            e if e.hvc_immediate() == Some(MARK_WRONG) => {
                                break Some(format!(
                                    "vCPU {k}'s block {:#x} in slot {} read {:#x}, not its mark {:#x}, with {} passes \
                                     left",
                                    x(13),
                                    x(27),
                                    x(9),
                                    MARKS[k],
                                    x(19)
                                ));
                            }
                            e if e.hvc_immediate() == Some(NOTHING_TAKEN) => {
                                break Some(format!("vCPU {k}'s HeapAlloc answered NULL in slot {}", x(27)));
                            }
                            e => {
                                // the exception entry's save area (`runtime.h` `struct w2m_save_area`: ESR, FAR, ELR
                                // at 0x100), which both loops share, one page table serving both
                                let pc = vcpu.reg(Reg::Pc).unwrap();
                                let [esr, far, elr] = [0, 8, 16].map(|o| view.read_u64(save + 0x100 + o));
                                break Some(format!(
                                    "vCPU {k}: {e:?} at {pc:#x}, not the loop's hvc #{DONE:#x}; the save area's ESR \
                                     {esr:#x}, FAR {far:#x}, ELR {elr:#x}"
                                ));
                            }
                        }
                    };
                    said_tx.send((k, wrong)).unwrap();
                }
            });
        }
        drop(said_tx);
        for round in 0..ROUNDS {
            // the heap as a process starts it: `ready` 0 and the heap's first page zero, the arena's first pages
            // the ProcessParameters; and the slots empty
            view.write_u32(ready, 0);
            view.write(heap, &[0u8; 0x20]);
            view.write(tables, &vec![0u8; 0x2000]);
            view.write_u64(arrival, 0);
            gate.wait();
            rounds += 1;
            let (mut wrong, mut heard, mut stopped) = (Vec::new(), 0, false);
            while heard < 2 {
                match said.recv_timeout(Duration::from_secs(60)) {
                    Ok((_, w)) => {
                        heard += 1;
                        wrong.extend(w);
                    }
                    Err(_) if !stopped => {
                        wrong.push("a loop had not reached its hvc in 60 s, both stopped".to_string());
                        stopped = true;
                        for h in handles.lock().unwrap().iter() {
                            h.stop().unwrap();
                        }
                    }
                    Err(_) => {
                        wrong.push("a stopped loop said nothing in 60 s more".to_string());
                        break;
                    }
                }
            }
            failure = (|| {
                if !wrong.is_empty() {
                    return Some(format!("round {round}: {}", wrong.join("; ")));
                }
                let next = view.read_u64(arena(HEAP_VA + 8));
                if !(start..=end).contains(&next) {
                    return Some(format!(
                        "round {round}: the bump pointer {next:#x} outside the arena's blocks, {start:#x} to {end:#x}"
                    ));
                }
                let lock = view.read_u32(arena(HEAP_VA + 0x18));
                if lock != 0 {
                    return Some(format!(
                        "round {round}: the heap's lock word {lock:#x} at the round's end"
                    ));
                }
                // the blocks the bump pointer carved, header to header by capacity (`take`, `heap.c` 110-116)
                let mut carved = BTreeMap::new();
                let mut at = start;
                while at < next {
                    let capacity = view.read_u64(arena(at + 8));
                    if capacity == 0 || !capacity.is_multiple_of(16) || at + HEADER + capacity > next {
                        return Some(format!(
                            "round {round}: the carved block at {at:#x} has capacity {capacity:#x}, the bump pointer \
                             {next:#x}"
                        ));
                    }
                    carved.insert(at, capacity);
                    at += HEADER + capacity;
                }
                let mut holder: BTreeMap<u64, String> = BTreeMap::new();
                for k in 0..2u64 {
                    for i in 0..SLOTS {
                        let entry = tables + 0x1000 * k + 16 * i;
                        let (p, size) = (view.read_u64(entry), view.read_u64(entry + 8));
                        let who = format!("vCPU {k}'s slot {i}");
                        let Some(&capacity) = p.checked_sub(HEADER).and_then(|h| carved.get(&h)) else {
                            return Some(format!("round {round}: {who} holds {p:#x}, not a carved block's"));
                        };
                        let marks = [view.read_u64(arena(p + 8)), view.read_u64(arena(p + size - 8))];
                        if size > capacity || marks != [MARKS[k as usize]; 2] {
                            return Some(format!(
                                "round {round}: {who}'s block {p:#x} of {size} bytes, capacity {capacity}, marks \
                                 {marks:#x?}"
                            ));
                        }
                        if let Some(other) = holder.insert(p - HEADER, who.clone()) {
                            return Some(format!("round {round}: the block {p:#x} held by {other} and by {who}"));
                        }
                    }
                }
                let held = holder.len();
                let mut link = view.read_u64(arena(HEAP_VA + 16));
                while link != 0 {
                    if !carved.contains_key(&link) {
                        return Some(format!(
                            "round {round}: the free list's link {link:#x} after {} blocks, not a carved block's header",
                            holder.len() - held
                        ));
                    }
                    if let Some(other) = holder.insert(link, "the free list".to_string()) {
                        return Some(format!(
                            "round {round}: the block at {link:#x} on the free list, and already {other}'s"
                        ));
                    }
                    link = view.read_u64(arena(link + HEADER));
                }
                if holder.len() != carved.len() {
                    return Some(format!(
                        "round {round}: {} blocks carved, {held} held and {} free: {} lost",
                        carved.len(),
                        holder.len() - held,
                        carved.len() - holder.len()
                    ));
                }
                carved_most = carved_most.max(carved.len());
                free_most = free_most.max(holder.len() - held);
                next_most = next_most.max(next);
                None
            })();
            if failure.is_some() {
                break;
            }
        }
        stop.store(true, std::sync::atomic::Ordering::Release);
        gate.wait();
    });
    if let Some(f) = failure {
        panic!("{f}");
    }
    eprintln!(
        "the process heap on two vCPUs: {rounds} rounds, each from the heap as a process starts it, of {PASSES} passes \
         over {SLOTS} slots a vCPU, {} HeapAlloc and {} HeapFree calls in all; no mark found wrong; at each round's end \
         every carved block held by one slot or on the free list once, none lost (at most {carved_most} carved, \
         {free_most} free), the bump pointer at most {next_most:#x} of the arena's {end:#x}, the lock free",
        rounds * 2 * PASSES * SLOTS,
        rounds * 2 * (PASSES * SLOTS - SLOTS)
    );
}
