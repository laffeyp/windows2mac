// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The five litmus tests on several vCPUs of the Rust host (Sprint 274; census row G1; the frozen host's
//! `tests/test_litmus.py` at `frozen-python-2026-10-29`, Sprint 105). x86-TSO forbids MP's r1=1 r2=0, LB's r1=r2=1,
//! 2+2W's x=1 y=1 at the end and IRIW's readers disagreeing on the order of two independent stores; it allows SB's
//! r1=r2=0, which the store buffer makes (Owens, Sarkar and Sewell, "A Better x86 Memory Model: x86-TSO", TPHOLs 2009).
//! Each test's threads are x86-64 assembled by clang, compiled by the Rust host at their dispatch misses, one vCPU
//! thread a litmus thread against shared words; iteration i uses the i-th word of each array, and the threads meet at
//! an arrival counter before each. A thread ends by jumping to DONE, which no block holds, so its vCPU exits with x17 =
//! DONE. Each thread runs alone once first, so every block exists before the threads run at once.
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use guest_runtime::layout::{
    FCW_OFF, IPA_BASE, LOCK_OWNER, MONITOR_DATA, MONITOR_VA, MXCSR_OFF, RUNTIME_IMAGE_VA, SLICE_UNTIL, VM_COMPILER,
    runtime_defines,
};
use hypervisor::{Reg, Vcpu, Vm, memory, sys_reg};
use pe_loader::{Loaded, Memory};
use w2m_vm::CPSR_EL1H;
use w2m_vm::compile::HostCompiler;
use w2m_vm::concurrent_memory::ConcurrentGuestMemory;
use w2m_vm::guest::{Guest, VMC_ENABLED, vm_compiler_symbols};
use w2m_vm::model_step::{ModelStep, model_step};
use w2m_vm::monitor::set_system_registers;
use w2m_vm::monitor_code::REG_BUDGET;
use w2m_vm::process::Start;
use w2m_vm::run::{REG_UNDEFINED_FLAGS, dispatch_insert, fpcr_for, x86_address_at};

const CODE: u64 = 0x2000_0000;
const STACK: u64 = 0x4000_0000;
const DATA: u64 = 0x3000_0000;
const DONE: u64 = 0x2FFF_0000;
const N: u64 = 100_000;
/// One array: a word an iteration.
const A: u64 = 8 * N;
/// x, y, the arrival counters, then the result arrays R0..R3.
const X: u64 = 0;
const Y: u64 = A;
const ARRIVE: u64 = 2 * A;
const R: u64 = 3 * A;
const DATA_SIZE: u64 = (R + 4 * A + 0xFFF) & !0xFFF;

/// Each thread: rdi the data, rcx the iterations, r9 the threads to wait for; rbx the iteration's byte offset.
const HEAD: &str = ".intel_syntax noprefix
    xor ebx, ebx
1:  lock inc qword ptr [rdi + rbx + ARRIVE]
2:  cmp qword ptr [rdi + rbx + ARRIVE], r9
    jae 3f
    pause
    jmp 2b
3:
";
const TAIL: &str = "    add rbx, 8
    dec rcx
    jnz 1b
    mov rax, DONE
    jmp rax
";

fn store(at: u64, v: u64) -> String {
    format!("    mov qword ptr [rdi + rbx + {at}], {v}\n")
}

fn load(at: u64, r: u64) -> String {
    format!("    mov rax, qword ptr [rdi + rbx + {at}]\n    mov qword ptr [rdi + rbx + {r}], rax\n")
}

/// Each test's thread bodies.
fn bodies(test: &str) -> Vec<String> {
    match test {
        "MP" => vec![store(X, 1) + &store(Y, 1), load(Y, R) + &load(X, R + A)],
        "SB" => vec![store(X, 1) + &load(Y, R), store(Y, 1) + &load(X, R + A)],
        "LB" => vec![load(X, R) + &store(Y, 1), load(Y, R + A) + &store(X, 1)],
        "2+2W" => vec![store(X, 1) + &store(Y, 2), store(Y, 1) + &store(X, 2)],
        "IRIW" => vec![
            store(X, 1),
            store(Y, 1),
            load(X, R) + &load(Y, R + A),
            load(Y, R + 2 * A) + &load(X, R + 3 * A),
        ],
        _ => panic!("no litmus test {test}"),
    }
}

/// Iterations with the outcome x86-TSO forbids (SB: the relaxed outcome it allows, counted instead).
fn forbidden(test: &str, w: &[Vec<u64>]) -> usize {
    let (x, y, r) = (&w[0], &w[1], &w[2..]);
    let pairs =
        |a: &[u64], b: &[u64], f: &dyn Fn(u64, u64) -> bool| a.iter().zip(b).filter(|(p, q)| f(**p, **q)).count();
    match test {
        "MP" => pairs(&r[0], &r[1], &|a, b| a == 1 && b == 0),
        "SB" => pairs(&r[0], &r[1], &|a, b| a == 0 && b == 0),
        "LB" => pairs(&r[0], &r[1], &|a, b| a == 1 && b == 1),
        "2+2W" => pairs(x, y, &|a, b| a == 1 && b == 1),
        _ => (0..N as usize)
            .filter(|&i| r[0][i] == 1 && r[1][i] == 0 && r[2][i] == 1 && r[3][i] == 0)
            .count(),
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// x86-64 bytes for `text`, by clang's assembler and `otool -t -X`, as the frozen judge has it.
fn assemble(text: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("w2m-litmus-{}-{:x}", std::process::id(), text.len()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("t.s"), text).unwrap();
    let ok = Command::new("clang")
        .args(["-c", "-target", "x86_64-apple-macos", "t.s", "-o", "t.o"])
        .current_dir(&dir)
        .status()
        .unwrap();
    assert!(ok.success(), "clang -c t.s");
    let out = Command::new("otool")
        .args(["-t", "-X", "t.o"])
        .current_dir(&dir)
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(&dir);
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .filter(|l| !l.contains("section") && !l.ends_with(':'))
        .flat_map(|l| {
            l.split_once('\t')
                .map_or(l, |(_, rest)| rest)
                .split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|b| b.len() == 2)
        .map(|b| u8::from_str_radix(&b, 16).unwrap())
        .collect()
}

/// The words of `test` after its threads ran at once: x, y and the four result arrays.
fn run_test(test: &str, entso: bool) -> Vec<Vec<u64>> {
    let texts: Vec<String> = bodies(test)
        .iter()
        .map(|body| {
            format!("{HEAD}{body}{TAIL}")
                .replace("ARRIVE", &ARRIVE.to_string())
                .replace("DONE", &DONE.to_string())
        })
        .collect();
    let raw = run_threads(&texts, entso, &[]);
    [X, Y, R, R + A, R + 2 * A, R + 3 * A]
        .iter()
        .map(|&o| (0..N).map(|i| word(&raw, o + 8 * i)).collect())
        .collect()
}

/// The little-endian word at `at` of `raw`.
fn word(raw: &[u8], at: u64) -> u64 {
    u64::from_le_bytes(raw[at as usize..at as usize + 8].try_into().unwrap())
}

/// The data region after `texts`, one thread's x86 program each (rdi the data, rcx the iterations, r9 the threads to
/// wait for, each ending by a jump to DONE), ran once each alone and then all at once for N iterations from the data
/// zeroed and `start`'s words (offset, value) written. Every dispatch miss is compiled at the host and every model
/// step (`hvc #10`, `#11`) stepped there, both under one lock.
fn run_threads(texts: &[String], entso: bool, start: &[(u64, u64)]) -> Vec<u8> {
    let root = root();
    let test = texts.len();
    let threads = texts.len() as u64;
    let mut code = Vec::new();
    let mut entries = Vec::new();
    for text in texts {
        entries.push(CODE + 0x100 + code.len() as u64);
        code.extend(assemble(text));
        code.resize(code.len().next_multiple_of(16), 0x90);
    }
    let (_, image) = guest_runtime::build(&root, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
    let services = guest_runtime::services::services(&guest_runtime::texts(&root).unwrap()).unwrap();
    let names =
        vm_compiler_symbols(&fs::read_to_string(root.join("tools/vm_compiler/stencil_table.h")).unwrap()).unwrap();
    let mut mem = Memory::default();
    mem.map(CODE, 0x10000, "code");
    mem.map(STACK, 0x10000, "stack");
    mem.map(DATA, DATA_SIZE, "data");
    let loaded = Loaded::bare(mem, CODE, STACK + 0x10000);
    let mut guest = Guest::new(
        &loaded,
        &image,
        &services,
        &Start::console("litmus.exe"),
        &BTreeSet::new(),
        &names,
    )
    .unwrap();
    let write = |g: &mut Guest, va: u64, bytes: &[u8]| {
        let at = g.layout.offset(va).unwrap() as usize;
        g.memory.as_mut_slice()[at..at + bytes.len()].copy_from_slice(bytes);
    };
    write(&mut guest, CODE + 0x100, &code);
    {
        let mem = guest.memory.as_mut_slice();
        let put =
            |mem: &mut [u8], off: u64, v: u64| mem[off as usize..off as usize + 8].copy_from_slice(&v.to_le_bytes());
        put(mem, MONITOR_DATA + 8, 1 << 62); // the limit
        put(mem, VM_COMPILER + VMC_ENABLED, 0); // every miss to the host, compiled under one lock
        put(mem, MONITOR_DATA + LOCK_OWNER, 1);
        // no other thread ready: a PAUSE does not leave the VM (Sprint 105's finding)
        put(mem, MONITOR_DATA + SLICE_UNTIL, u64::MAX);
        mem[(MONITOR_DATA + MXCSR_OFF) as usize..(MONITOR_DATA + MXCSR_OFF) as usize + 4]
            .copy_from_slice(&0x1F80u32.to_le_bytes());
        mem[(MONITOR_DATA + FCW_OFF) as usize..(MONITOR_DATA + FCW_OFF) as usize + 2]
            .copy_from_slice(&0x27Fu16.to_le_bytes());
    }
    let vm = Vm::new(40).unwrap();
    vm.map(&guest.memory, IPA_BASE, memory::READ | memory::WRITE | memory::EXEC)
        .unwrap();
    let (ttbr, dispatch_va, miss_va) = (guest.ttbr, guest.code.dispatch_va, guest.code.miss_va);
    // what the host reads and writes while other vCPUs run goes through the view, which makes no reference into the
    // memory (Sprint 276); the compile and the model's step still take slices under the lock (census D7)
    let concurrent = {
        let m = guest.memory.as_mut_slice();
        // SAFETY: the mapping is `guest.memory`'s, which outlives the threads; it does not move with `guest`
        unsafe { ConcurrentGuestMemory::from_raw_parts(m.as_mut_ptr(), m.len()) }
    };
    let shared = Mutex::new((guest, HostCompiler::default()));
    // thread k to its DONE: misses compiled under the lock, slice ends renewed
    let run_thread = |k: usize, n: u64, wait_for: u64| {
        let vcpu = Vcpu::new(&vm).unwrap();
        set_system_registers(&vcpu, ttbr).unwrap();
        if !entso {
            vcpu.set_sys_reg(sys_reg::ACTLR_EL1, 0).unwrap(); // the plant
        }
        vcpu.set_reg(Reg::Fpcr, fpcr_for(0x1F80)).unwrap();
        for r in 0..31u8 {
            vcpu.set_reg(Reg::X(r), 0).unwrap();
        }
        vcpu.set_reg(Reg::X(1), n).unwrap();
        vcpu.set_reg(Reg::X(7), DATA).unwrap();
        vcpu.set_reg(Reg::X(9), wait_for).unwrap();
        vcpu.set_reg(Reg::X(17), entries[k]).unwrap();
        vcpu.set_reg(Reg::X(4), STACK + 0x8000 - 0x100 * k as u64).unwrap();
        vcpu.set_reg(Reg::X(REG_UNDEFINED_FLAGS), 0x3F).unwrap();
        vcpu.set_reg(Reg::X(28), MONITOR_VA + MONITOR_DATA).unwrap();
        vcpu.set_reg(Reg::X(REG_BUDGET as u8), 1 << 62).unwrap();
        vcpu.set_reg(Reg::Pc, dispatch_va).unwrap();
        vcpu.set_reg(Reg::Cpsr, CPSR_EL1H).unwrap();
        loop {
            let exit = vcpu.run().unwrap();
            match exit.hvc_immediate() {
                Some(1) => {
                    let target = vcpu.reg(Reg::X(17)).unwrap();
                    if target == DONE {
                        return;
                    }
                    let mut held = shared.lock().unwrap();
                    let (guest, host) = &mut *held;
                    let va = match host.blocks.get(&target) {
                        Some(va) => *va,
                        None => host
                            .compile(guest, target)
                            .unwrap_or_else(|e| panic!("thread {k} of {test}: {e}")),
                    };
                    dispatch_insert(concurrent, target, va);
                    drop(held);
                    vcpu.set_reg(Reg::Pc, miss_va + 4).unwrap();
                }
                Some(4) => vcpu.set_reg(Reg::X(REG_BUDGET as u8), 1 << 62).unwrap(),
                // a model step, as the run loop serves it (`run.rs`'s `HVC_SLOW_SSE`, `HVC_SLOW_STEP`), under the lock
                Some(10) | Some(11) => {
                    MODEL_STEPS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let pc = vcpu.reg(Reg::Pc).unwrap();
                    let mut held = shared.lock().unwrap();
                    let (guest, _) = &mut *held;
                    let rip = x86_address_at(concurrent, pc - 4).unwrap();
                    let mut code = vec![0u8; 15];
                    concurrent.read(guest.layout.offset(rip).unwrap(), &mut code);
                    let stepped = model_step(guest, &vcpu, &code, rip).unwrap();
                    assert!(
                        matches!(stepped, ModelStep::Resumed),
                        "thread {k} of {test}: the model's step at 0x{rip:x}"
                    );
                    drop(held);
                    vcpu.set_reg(Reg::Cpsr, CPSR_EL1H).unwrap();
                }
                _ => panic!("thread {k} of {test}: {exit:?} at 0x{:x}", vcpu.reg(Reg::Pc).unwrap()),
            }
        }
    };
    // alone, once: every block compiled
    for k in 0..texts.len() {
        std::thread::scope(|s| {
            s.spawn(|| run_thread(k, 1, 1));
        });
    }
    {
        let mut held = shared.lock().unwrap();
        let at = held.0.layout.offset(DATA).unwrap() as usize;
        let mem = held.0.memory.as_mut_slice();
        mem[at..at + DATA_SIZE as usize].fill(0);
        for &(offset, value) in start {
            mem[at + offset as usize..at + offset as usize + 8].copy_from_slice(&value.to_le_bytes());
        }
    }
    std::thread::scope(|s| {
        for k in 0..texts.len() {
            let run = &run_thread;
            s.spawn(move || run(k, N, threads));
        }
    });
    let held = shared.lock().unwrap();
    let at = held.0.layout.offset(DATA).unwrap() as usize;
    held.0.memory.as_slice()[at..at + DATA_SIZE as usize].to_vec()
}

/// Sprint 274 (reviewer-3's Low 1 at 272; Deferred, Sprint 272): two threads exchange their tokens, 1 and 2, with a
/// word four bytes off its natural alignment, starting at 100, N times each at once; the exchange is atomic when the
/// three values, the word's and each thread's last, are still {1, 2, 100}. A misaligned locked access leaves the
/// exclusive pair for the model, which the host steps under the harness's one lock.
fn misaligned_xchg_values() -> Vec<u64> {
    const AT: u64 = 4;
    let text = |token: u64, k: u64| {
        format!(
            ".intel_syntax noprefix
    mov rax, {token}
1:  xchg qword ptr [rdi + {AT}], rax
    dec rcx
    jnz 1b
    mov qword ptr [rdi + {}], rax
    mov rax, {DONE}
    jmp rax
",
            R + 8 * k
        )
    };
    let raw = run_threads(&[text(1, 0), text(2, 1)], true, &[(AT, 100)]);
    let mut values = vec![word(&raw, AT), word(&raw, R), word(&raw, R + 8)];
    values.sort_unstable();
    values
}

#[test]
fn a_misaligned_xchg_raced_on_two_vcpus_keeps_its_values() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let before = MODEL_STEPS.load(std::sync::atomic::Ordering::Relaxed);
    let values = misaligned_xchg_values();
    let steps = MODEL_STEPS.load(std::sync::atomic::Ordering::Relaxed) - before;
    println!("the misaligned XCHG's word and the threads' last values: {values:?}, {steps} model steps");
    // every exchange, two threads of N and their runs alone, stepped in the model
    assert_eq!(steps as u64, 2 * N + 2, "the exchanges did not all reach the model");
    assert_eq!(values, [1, 2, 100], "a value lost or doubled by the exchange");
}

/// One VM a process at a time (`Vm::new`), so the tests take turns.
static ONE_VM: Mutex<()> = Mutex::new(());

/// The model steps the harness served, read by the misaligned XCHG's test to know its exchanges reached the model.
static MODEL_STEPS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[test]
fn x86_tsos_forbidden_outcomes_never_appear() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    for test in ["MP", "LB", "2+2W", "IRIW"] {
        let words = run_test(test, true);
        let n = forbidden(test, &words);
        println!("{test}: forbidden outcome {n} in {N}");
        assert_eq!(n, 0, "{test}'s forbidden outcome appeared");
    }
}

/// SB's r1 = r2 = 0 is x86-TSO's one relaxation: seen, the harness can see a reordering, so 0 elsewhere means the
/// order held. The frozen judge saw it 1 to 36 times in 100,000, and in 0 of 3 runs of 100: up to five runs.
#[test]
fn the_store_buffer_shows_in_sb() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let mut counts = Vec::new();
    while counts.len() < 5 && counts.iter().all(|&c| c == 0) {
        counts.push(forbidden("SB", &run_test("SB", true)));
    }
    println!("SB: relaxed outcome by run {counts:?} in {N} each");
    assert!(
        counts.iter().any(|&c| c > 0),
        "SB's relaxed outcome in none of five runs"
    );
}

/// The plant: ACTLR_EL1.EnTSO clear on every vCPU shows MP's forbidden outcome, in up to ten runs (the frozen judge's
/// three missed about one time in 600, Sprint 110).
#[test]
fn the_planted_entso_off_shows_mps_forbidden_outcome() {
    let _one = ONE_VM.lock().unwrap_or_else(|e| e.into_inner());
    let mut counts = Vec::new();
    while counts.len() < 10 && counts.iter().all(|&c| c == 0) {
        counts.push(forbidden("MP", &run_test("MP", false)));
    }
    println!("MP with EnTSO clear: forbidden outcome by run {counts:?} in {N} each");
    assert!(
        counts.iter().any(|&c| c > 0),
        "MP's forbidden outcome in none of ten runs with EnTSO clear"
    );
}
