// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Sprint 145's judge: the CPU cases run in a Rust VM, one instruction per block, each completed case judged against
//! the Intel run Windows recorded for its set. The instruction's words come from the VM's C emitter, built for the host
//! by clang and called through `dlopen` (`vmc_decode`, `vmc_emit`, `vmc_ends`). Runs where clang is found and the result
//! files were downloaded (`build/probe-runs`, matching their recorded hashes).
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use cpu_cases::{read_cases, read_results, recorded_sha256, sha256};
use guest_runtime::layout::{IPA_BASE, RUNTIME_IMAGE_VA, runtime_defines};
use hypervisor::{Vcpu, Vm, memory};
use w2m_vm::cases::{Compiled, case_memory, run_cases};
use w2m_vm::guest::{Guest, vm_compiler_symbols};
use w2m_vm::process::Start;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

unsafe extern "C" {
    fn dlopen(path: *const c_char, mode: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlerror() -> *const c_char;
}

/// vm_compiler.h `struct vmc_insn`.
#[repr(C)]
struct Insn {
    form: *const c_void,
    address: u64,
    values: [u64; 24],
    nvalues: c_int,
    length: c_int,
}

/// vm_compiler.h `struct vmc_flags`.
#[repr(C)]
struct Flags {
    nzcv_from: c_int,
    live_after: c_int,
    nzcv_out: c_int,
}

type Decode = unsafe extern "C" fn(*const u8, c_int, u64, *mut Insn) -> c_int;
type Emit = unsafe extern "C" fn(*const Insn, *const Flags, u64, u64, *const u64, *mut u32, c_int) -> c_int;
type Ends = unsafe extern "C" fn(*const Insn) -> c_int;

struct C {
    decode: Decode,
    emit: Emit,
    ends: Ends,
}

fn load_c() -> Option<C> {
    let found = Command::new("xcrun").args(["--find", "clang"]).output().ok()?;
    if !found.status.success() {
        return None;
    }
    let dir = root().join("target/case_runner");
    fs::create_dir_all(&dir).unwrap();
    let lib = dir.join(format!("libvmc-{}.dylib", std::process::id()));
    let vmc = root().join("tools/vm_compiler");
    let built = Command::new("xcrun")
        .args(["clang", "-shared", "-O2", "-I"])
        .arg(&vmc)
        .args(["x86_decode.c", "vm_compiler.c", "a64_encoder.c"].map(|f| vmc.join(f)))
        .arg("-o")
        .arg(&lib)
        .output()
        .unwrap();
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    let path = CString::new(lib.to_str().unwrap()).unwrap();
    unsafe {
        let h = dlopen(path.as_ptr(), 2);
        assert!(!h.is_null(), "{:?}", CStr::from_ptr(dlerror()));
        let sym = |n: &str| {
            let p = dlsym(h, CString::new(n).unwrap().as_ptr());
            assert!(!p.is_null(), "{n}");
            p
        };
        Some(C {
            decode: std::mem::transmute::<*mut c_void, Decode>(sym("vmc_decode")),
            emit: std::mem::transmute::<*mut c_void, Emit>(sym("vmc_emit")),
            ends: std::mem::transmute::<*mut c_void, Ends>(sym("vmc_ends")),
        })
    }
}

/// The case files by their SHA-256, and each downloaded Intel result whose hash and set are recorded: (run, set file,
/// result bytes).
fn runs() -> Vec<(String, PathBuf, Vec<u8>)> {
    let mut sets: BTreeMap<String, PathBuf> = BTreeMap::new();
    for e in fs::read_dir(root().join("probes")).unwrap() {
        let p = e.unwrap().path().join("cases.bin");
        if p.exists()
            && p.parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("semantics")
        {
            sets.insert(sha256(&fs::read(&p).unwrap()), p);
        }
    }
    let mut out = Vec::new();
    let Ok(dirs) = fs::read_dir(root().join("build/probe-runs")) else {
        return out;
    };
    for e in dirs {
        let dir = e.unwrap().path();
        let run = dir.file_name().unwrap().to_string_lossy().to_string();
        let store = root().join("store/probe-runs").join(&run);
        let (Ok(raw), Ok(sums), Ok(case_sums), Ok(cpu)) = (
            fs::read(dir.join("result-1.bin")),
            fs::read_to_string(store.join("results.sha256")),
            fs::read_to_string(store.join("cases.sha256")),
            fs::read_to_string(store.join("cpu.txt")),
        ) else {
            continue;
        };
        if !cpu.to_lowercase().contains("intel") || recorded_sha256(&sums, "result-1.bin") != Some(sha256(&raw)) {
            continue;
        }
        if let Some(set) = recorded_sha256(&case_sums, "cases.bin").and_then(|h| sets.get(&h)) {
            out.push((run, set.clone(), raw));
        }
    }
    out.sort();
    out
}

#[test]
fn completed_cases_agree_with_windows() {
    let Some(c) = load_c() else {
        eprintln!("clang not found: the case runner is not run here");
        return;
    };
    let runs = runs();
    if runs.is_empty() {
        eprintln!("no downloaded Intel results: the case runner is not run here");
        return;
    }
    let root = root();
    let (_, image) = guest_runtime::build(&root, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    let services = guest_runtime::services::services(&guest_runtime::texts(&root).unwrap()).unwrap();
    let table = fs::read_to_string(root.join("tools/vm_compiler/stencil_table.h")).unwrap();
    let names = vm_compiler_symbols(&table).unwrap();
    let symbols: Vec<u64> = names.iter().map(|n| image.symbols[n]).collect();
    let loaded = case_memory();
    let mut guest = Guest::new(
        &loaded,
        &image,
        &services,
        &Start::console("app.exe"),
        &BTreeSet::new(),
        &names,
    )
    .unwrap();
    let vm = Vm::new(40).unwrap();
    vm.map(&guest.memory, IPA_BASE, memory::READ | memory::WRITE | memory::EXEC)
        .unwrap();
    let vcpu = Vcpu::new(&vm).unwrap();

    // The frozen Python lowering's words for every case instruction (`build/case_runner/python-words.txt`, made by
    // running `compile_insn(Assembler(BODY), lift(code + bytes(16), CODE_VA), EXIT, symbols, None, ALL_FLAGS,
    // False)` as the Python's lockstep compile does, vm_backend 4611-4618): the compiler that judges the runner itself,
    // until the Rust lowering takes its place. The C emitter below compiles none of the cases at their own address: the
    // stencil key carries the code address's value class, and the table's keys are the game's, above 4 GB.
    // the body of every case's block (its base plus the SUBS and the B.MI) and the miss stub
    const FIXTURE_BASE: u64 = 0x7f00_0006_2170;
    const FIXTURE_EXIT: u64 = 0x7f00_0000_1928;
    let fixture: BTreeMap<Vec<u8>, Option<(bool, Vec<u32>)>> =
        match fs::read_to_string(root.join("build/case_runner/python-words.txt")) {
            Ok(text) => text
                .lines()
                .map(|l| {
                    let p: Vec<&str> = l.split_whitespace().collect();
                    let code = (0..p[0].len())
                        .step_by(2)
                        .map(|i| u8::from_str_radix(&p[0][i..i + 2], 16).unwrap());
                    let words = (p[1] == "ok").then(|| {
                        (
                            p[2] == "1",
                            p[3..].iter().map(|w| u32::from_str_radix(w, 16).unwrap()).collect(),
                        )
                    });
                    (code.collect(), words)
                })
                .collect(),
            Err(_) => BTreeMap::new(),
        };
    let python = |code: &[u8], _address: u64, base: u64, exit_va: u64| -> Result<Compiled, String> {
        assert_eq!(
            (base, exit_va),
            (FIXTURE_BASE, FIXTURE_EXIT),
            "the fixture's words were made for another layout"
        );
        match fixture.get(code) {
            Some(Some((ends, words))) => Ok(Compiled {
                words: words.clone(),
                ends: *ends,
            }),
            Some(None) => Err("the Python refused it".into()),
            None => Err("not in the fixture".into()),
        }
    };
    if fixture.is_empty() {
        eprintln!("no Python words fixture (build/case_runner/python-words.txt): the C emitter compiles");
    }

    // the C emitter for a lone instruction: no NZCV source, every flag live after, no NZCV out
    let compile = |code: &[u8], address: u64, base: u64, exit_va: u64| -> Result<Compiled, String> {
        let mut insn = std::mem::MaybeUninit::<Insn>::zeroed();
        let r = unsafe { (c.decode)(code.as_ptr(), code.len() as c_int, address, insn.as_mut_ptr()) };
        if r < 0 {
            return Err(format!("C decode status {r}"));
        }
        let insn = unsafe { insn.assume_init() };
        let flags = Flags {
            nzcv_from: 0,
            live_after: 63,
            nzcv_out: 0,
        };
        let mut words = [0u32; 512];
        let n = unsafe {
            (c.emit)(
                &insn,
                &flags,
                base,
                exit_va,
                symbols.as_ptr(),
                words.as_mut_ptr(),
                words.len() as c_int,
            )
        };
        if n < 0 {
            return Err(format!("C emit status {n}"));
        }
        Ok(Compiled {
            words: words[..n as usize].to_vec(),
            ends: unsafe { (c.ends)(&insn) } != 0,
        })
    };

    let mut differences = 0;
    for (run, set, raw) in &runs {
        let (fmt, cases) = read_cases(&fs::read(set).unwrap()).unwrap();
        let results = read_results(raw, fmt).unwrap();
        let t = if fixture.is_empty() {
            run_cases(&mut guest, &vcpu, &cases, &results, &compile).unwrap()
        } else {
            run_cases(&mut guest, &vcpu, &cases, &results, &python).unwrap()
        };
        let refused: usize = t.refused.values().sum();
        println!(
            "run {run} ({}): {} cases; judged {}; Windows faults {}; model {}; refused {refused}; undefined memory {}; \
             {} difference classes; {:.1} s",
            set.parent().unwrap().file_name().unwrap().to_string_lossy(),
            cases.len(),
            t.judged,
            t.windows_faults,
            t.model,
            t.undefined_memory,
            t.differences.len(),
            t.seconds
        );
        println!("    refused by cause: {:?}", t.refused);
        for (k, (n, example)) in t.differences.iter().take(12) {
            println!("    {n:6} {k}: {example}");
        }
        differences += t.differences.len();
    }
    assert_eq!(differences, 0, "difference classes across the runs");
}
