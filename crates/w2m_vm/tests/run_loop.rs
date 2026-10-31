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
use guest_runtime::layout::{
    AP_READ_ONLY, CACHE_CODE_VA, CACHE_FILE_SIZE, CACHE_FILE_VA, CODE_CACHE, CODE_CACHE_SIZE, DISPATCH_TABLE, IPA_BASE,
    MONITOR_DATA, MONITOR_VA, PAGE, PROCESS_VA, RECON_COUNT, RECON_TABLE, RUN_TABLE, RUNTIME_IMAGE_VA, STATE_OFF,
    VM_COMPILER, VMC_LOG, runtime_defines,
};
use hypervisor::{Vcpu, Vm, memory};
use pe_loader::{Loaded, load};
use translator::lower::RuntimeCalls;
use w2m_vm::cache_file::{self, Exits};
use w2m_vm::compile::HostCompiler;
use w2m_vm::guest::{Guest, vm_compiler_symbols};
use w2m_vm::monitor_code::{VMC_INSTALLER, VMC_LOG_COUNT};
use w2m_vm::process::Start;
use w2m_vm::run::{JUDGE_KIND, Outcome, Stop, run_compiling};
use w2m_vm::tables::{block_descriptor, page_descriptor};

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
    let guest = guest_with(root, &loaded, &Start::console("probe.exe"));
    (loaded, guest)
}

/// The guest of `loaded` as `start` asks.
fn guest_with(root: &Path, loaded: &Loaded, start: &Start) -> Guest {
    let (_, image) = guest_runtime::build(root, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
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
    let counter = u64::from_str_radix(String::from_utf8(guest.stdout.clone()).unwrap().trim_end(), 16).unwrap();
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
    let at = guest.layout.offset(CACHE_FILE_VA).expect("no map at CACHE_FILE_VA") as usize;
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
        plain.layout.offset(CACHE_FILE_VA),
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
    let at = guest.layout.offset(code).unwrap() as usize;
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
    assert!(host.code_written(&mut guest, start).unwrap(), "the write was not taken");
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
        .install(&mut guest, entry)
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
        host.code_written(&mut guest, written).unwrap(),
        "the write to 0x{written:x} was not taken"
    );
    let cache = host.cache.as_ref().unwrap();
    assert_eq!(cache.stopped_at, Some(written), "the cache did not stop");
    assert!(!host.blocks.contains_key(&entry), "the installed block was not dropped");
    assert!(
        host.install(&mut guest, other).is_none(),
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
