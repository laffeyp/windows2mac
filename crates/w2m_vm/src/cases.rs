// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The CPU cases run in a Rust VM, one instruction per block, and each completed case judged against the result
//! Windows gave (Sprint 145): `scripts/vm_oracle.py` `run_cases` and `judge_done`. The instruction's words come from a
//! compiler the caller gives, today the C emitter built for the host, until the Rust lowering takes its place. A case
//! Windows ended in a fault is judged through the guest's exception records (Sprint 147: `judge_fault`). Every judged
//! case is judged on its whole recorded state, MXCSR, XMM, the YMM upper halves and the x87 registers with it (Sprint
//! 148). Each function cites the lines it carries.

use std::collections::BTreeMap;
use std::time::Instant;

use cpu_cases::{
    CODE_VA, Case, CaseResult, DATA_OFF, DATA_VA, DF_BIT, EDGE, EDGE_OFF, FLAG_BITS, RET_TARGET, STACK_OFF, STACK_SIZE,
    STACK_VA, WINDOW, X87State,
};
use guest_runtime::layout::{
    DISPATCH_ENTRIES, DISPATCH_TABLE, MONITOR_DATA, MONITOR_SIZE, MONITOR_VA, RECON_COUNT, RECON_TABLE, RUN_TABLE,
    STATE_OFF, TRANSPORT_OFF, VECTOR_SLOTS, VM_COMPILER,
};
use hypervisor::{Exit, HvError, Reg, Vcpu};
use pe_loader::{Loaded, Memory};

use crate::CPSR_EL1H;
use crate::assembler::{Assembler, Target};
use crate::guest::{Exception, Guest, VMC_CURSOR, VMC_ENABLED};
use crate::host_request::{self, Served};
use crate::monitor::set_system_registers;
use crate::monitor_code::{EMPTY_TARGET, REG_BUDGET};
use crate::run::{FCW_OFF, LOCK_OWNER, MXCSR_OFF, REG_UNDEFINED_FLAGS, dispatch_insert, fpcr_for};

/// x19-x23 hold CF, ZF, SF, OF and PF (vm_backend 358, 371).
pub const FLAG_REG: [(char, u8); 5] = [('C', 19), ('Z', 20), ('S', 21), ('O', 22), ('P', 23)];
/// REG_UNDEFINED_FLAGS' bits (vm_backend 369).
pub const FLAG_BIT: [(char, u32); 6] = [('C', 0), ('Z', 1), ('S', 2), ('O', 3), ('P', 4), ('A', 5)];
/// MONITOR_DATA + these: the lazy AF operands (a, b, r; r is PF's result) and DF (vm_backend 89, 93-94).
pub const AF_OPERANDS: u64 = 0x20;
pub const PF_RESULT: u64 = AF_OPERANDS + 16;
pub const DF_BYTE: u64 = 0x38;
/// MONITOR_DATA + this: YMM0-15 bits 255:128 (vm_backend 106).
pub const YMM_HI_OFF: u64 = 0x400;
/// The state block's x87 status word (the abridged tags after it) and ST0-ST7, 16 bytes apart (vm_backend 108).
pub const X87_SW_OFF: u64 = 158;
pub const X87_ST_OFF: u64 = 768;
/// The model's exits: an SSE instruction and one instruction run whole (vm_backend 1118, 1125).
pub const HVC_SLOW_SSE: u16 = 10;
pub const HVC_SLOW_STEP: u16 = 11;
/// A guest fault left its block (vm_backend 1128).
pub const HVC_FAULT_GIVE_BACK: u16 = 14;
/// `Tally::host_requests`' key for the give-backs: above every transport op.
pub const HOST_GIVE_BACK: u64 = 1 << 32;

/// A block's words and its marks (host VA, x86 address).
type Block = (Vec<u32>, Vec<(u64, u64)>);

/// One instruction's words from the caller's compiler, and whether it ends its block (a branch).
pub struct Compiled {
    pub words: Vec<u32>,
    pub ends: bool,
}

/// The instruction compiler: (its bytes, its address, the host address its words go to, the address a taken branch
/// exits to) -> its words, or why it refuses.
pub type Compile<'a> = &'a dyn Fn(&[u8], u64, u64, u64) -> Result<Compiled, String>;

/// The three regions of a case (vm_oracle 179-183).
pub fn case_memory() -> Loaded {
    let mut mem = Memory::default();
    mem.map(CODE_VA, 0x1000, "code");
    mem.map(DATA_VA, 0x1000, "data");
    mem.map(STACK_VA, STACK_SIZE, "stack");
    Loaded::bare(mem, CODE_VA, STACK_VA + STACK_SIZE)
}

fn read_u64(mem: &[u8], off: u64) -> u64 {
    u64::from_le_bytes(mem[off as usize..off as usize + 8].try_into().unwrap())
}

fn write_u64(mem: &mut [u8], off: u64, v: u64) {
    mem[off as usize..off as usize + 8].copy_from_slice(&v.to_le_bytes());
}

/// `parity_even` (vm_backend 564-566).
fn parity_even(v: u64) -> bool {
    (v & 0xFF).count_ones().is_multiple_of(2)
}

/// What the runner did.
#[derive(Debug, Default)]
pub struct Tally {
    pub judged: usize,
    /// cases Windows ended in a fault, and of them those judged at the VM's fault
    pub windows_faults: usize,
    pub faults_judged: usize,
    /// the host requests served (`hvc #8`), by op
    pub host_requests: BTreeMap<u64, usize>,
    /// cases whose instruction the VM ran in the model (hvc #10 or #11): counted, not judged here
    pub model: usize,
    /// refusals of the compiler, by cause
    pub refused: BTreeMap<String, usize>,
    pub undefined_memory: usize,
    /// difference -> (count, an example)
    pub differences: BTreeMap<String, (usize, String)>,
    pub seconds: f64,
    /// cases judged on their whole state, by iclass, and of them those with YMM upper halves (format 3) and with x87
    /// state (format 4) in the Windows result
    pub by_class: BTreeMap<String, usize>,
    pub ymm_judged: usize,
    pub x87_judged: usize,
}

impl Tally {
    /// A case judged on its whole recorded state (at completion or at a fault).
    fn judged_whole(&mut self, iclass: &str, r: &CaseResult) {
        self.judged += 1;
        *self.by_class.entry(iclass.to_string()).or_default() += 1;
        self.ymm_judged += usize::from(r.ymm_hi.is_some());
        self.x87_judged += usize::from(r.x87.is_some());
    }

    fn note(&mut self, iclass: &str, key: &str, example: String) {
        let e = self
            .differences
            .entry(format!("{iclass} {key}"))
            .or_insert((0, example));
        e.0 += 1;
    }
}

/// `assemble_block` (vm_backend 922-970) for one instruction, at `base`: the budget's SUBS and its branch, the
/// instruction's words (compiled for their own address, `base + 8`), the fall-through exit to `exit_va` with x17 the next
/// address, and the budget's stop. Returns the words and the marks (host VA, x86 address).
fn assemble_one(
    compile: Compile<'_>,
    code: &[u8],
    address: u64,
    length: u64,
    base: u64,
    exit_va: u64,
    slice_va: u64,
) -> Result<Block, String> {
    let mut a = Assembler::new(base);
    let mut marks = vec![(a.here(), address)]; // the block entry: a guest boundary
    let (stop, body) = (a.label(), a.label());
    a.emit(&[0xF100_0000 | 1 << 10 | REG_BUDGET << 5 | REG_BUDGET]); // SUBS x23, x23, #1
    a.branch(|off| a64_encoder::bcond("mi", off), stop);
    a.bind(body);
    marks.push((a.here(), address));
    let c = compile(code, address, a.here(), exit_va)?;
    a.emit(&c.words);
    if !c.ends {
        a.emit(&a64_encoder::mov_imm(17, address + length));
        a.branch(a64_encoder::b, Target::Va(exit_va));
    }
    a.bind(stop); // an exit whose address maps to the block's first instruction
    marks.push((a.here(), address));
    a.branch(a64_encoder::bl, Target::Va(slice_va));
    a.emit(&[a64_encoder::hvc(4)]);
    a.branch(a64_encoder::b, body);
    Ok((a.finish(), marks))
}

/// vm_oracle 52: the EFlags bits a fault's CONTEXT is judged on: CF, bit 1, PF, AF, ZF, SF, IF, DF, OF.
pub const EFLAGS_FAULT_MASK: u64 = 0x1 | 0x2 | 0x4 | 0x10 | 0x40 | 0x80 | 0x200 | 0x400 | 0x800;

/// `windows` (vm_oracle 159-167): the data, stack and edge windows against Windows's.
fn windows(guest: &Guest, r: &CaseResult) -> Vec<(String, String)> {
    let mem = guest.memory.as_slice();
    let at = |va: u64| guest.layout.offset(va).unwrap() as usize;
    let mut out = Vec::new();
    let o = at(DATA_VA + DATA_OFF);
    if mem[o..o + WINDOW] != r.data[..] {
        out.push(("data window".into(), "differs".into()));
    }
    let s = at(STACK_VA + STACK_OFF);
    if mem[s..s + WINDOW] != r.stack[..] {
        out.push(("stack window".into(), "differs".into()));
    }
    let e = at(DATA_VA + EDGE_OFF);
    if mem[e..e + EDGE] != r.edge[..] {
        out.push(("edge window".into(), "differs (a store before the fault)".into()));
    }
    out
}

/// vm_backend 1117: (FPSR bit, MXCSR bit): IOC IE, DZC ZE, OFC OE, UFC UE, IXC PE, IDC DE.
const FPSR_TO_MXCSR: [(u32, u32); 6] = [(0, 0), (1, 2), (2, 3), (3, 4), (4, 5), (7, 1)];

/// `mxcsr_flags_from_fpsr` (vm_backend 1146-1147).
fn mxcsr_flags_from_fpsr(fpsr: u64) -> u64 {
    FPSR_TO_MXCSR
        .iter()
        .filter(|(f, _)| fpsr >> f & 1 == 1)
        .map(|(_, m)| 1u64 << m)
        .sum()
}

/// `Vcpu.ymm_hi`: YMM0-15 bits 255:128, the monitor data page's slots (Sprint 049).
fn vm_ymm_hi(guest: &Guest) -> [u128; 16] {
    let mem = guest.memory.as_slice();
    std::array::from_fn(|i| {
        let at = (MONITOR_DATA + YMM_HI_OFF) as usize + 16 * i;
        u128::from_le_bytes(mem[at..at + 16].try_into().unwrap())
    })
}

/// `Vcpu.x87`: FCW from the monitor data page, FSW, the abridged tags and ST0-ST7 in stack order from the state block
/// (Sprint 065).
fn vm_x87(guest: &Guest) -> X87State {
    let mem = guest.memory.as_slice();
    let md = (MONITOR_DATA + FCW_OFF) as usize;
    let st = (MONITOR_SIZE + STATE_OFF) as usize;
    let sw = st + X87_SW_OFF as usize;
    X87State {
        cw: u16::from_le_bytes([mem[md], mem[md + 1]]),
        sw: u16::from_le_bytes([mem[sw], mem[sw + 1]]),
        tags: mem[sw + 2],
        st: std::array::from_fn(|i| {
            let at = st + X87_ST_OFF as usize + 16 * i;
            let mut b = [0u8; 16];
            b[..10].copy_from_slice(&mem[at..at + 10]);
            u128::from_le_bytes(b)
        }),
    }
}

/// semantics_differential 997-1001: FSW's condition codes each class leaves undefined.
const C0: u16 = 1 << 8;
const C1: u16 = 1 << 9;
const C2: u16 = 1 << 10;
const C3: u16 = 1 << 14;

fn x87_undefined(label: &str) -> u16 {
    match label {
        "FLD" | "FSTP" | "FMUL" | "FADD" | "FIADD" | "FIDIVR" | "FSQRT" | "FISTP" | "FISTTP" => C0 | C2 | C3,
        "FSIN" | "FCOS" => C0 | C3,
        "FLDCW" | "FNCLEX" => C0 | C1 | C2 | C3,
        _ => 0,
    }
}

/// `judge_x87` (semantics_differential 1009-1024): FCW, FSW without the condition codes the class leaves undefined,
/// the abridged tags, and ST0-ST7 (an empty register too). Keys without the label, as `judge_done` uses them.
fn judge_x87(label: &str, c: &Case, m: &X87State, r: &X87State) -> Vec<(String, String)> {
    let mut diffs = Vec::new();
    if m.cw != r.cw {
        diffs.push(("FCW".into(), format!("model 0x{:x}, cpu 0x{:x}", m.cw, r.cw)));
    }
    let keep = !x87_undefined(label);
    if m.sw & keep != r.sw & keep {
        let before = c.x87.map(|x| format!(" (in 0x{:x})", x.sw)).unwrap_or_default();
        diffs.push(("FSW".into(), format!("model 0x{:x}, cpu 0x{:x}{before}", m.sw, r.sw)));
    }
    if m.tags != r.tags {
        diffs.push(("FTW".into(), format!("model 0x{:02x}, cpu 0x{:02x}", m.tags, r.tags)));
    }
    for i in 0..8 {
        if m.st[i] != r.st[i] {
            diffs.push((
                format!("st{i}"),
                format!("model {:020x}, cpu {:020x}", m.st[i], r.st[i]),
            ));
        }
    }
    diffs
}

/// `judge_done`'s and `judge_fault`'s format 3 and 4 parts (vm_oracle 150-155, 116-121): the YMM upper halves and the
/// x87 state, each key with `prefix`.
fn judge_ymm_x87(guest: &Guest, label: &str, c: &Case, r: &CaseResult, prefix: &str) -> Vec<(String, String)> {
    let mut diffs = Vec::new();
    if let Some(want) = r.ymm_hi {
        for (i, (vm, cpu)) in vm_ymm_hi(guest).iter().zip(want).enumerate() {
            if *vm != cpu {
                diffs.push((format!("{prefix}ymm{i}.hi"), format!("vm {vm:032x}, cpu {cpu:032x}")));
            }
        }
    }
    if let Some(want) = &r.x87 {
        for (k, d) in judge_x87(label, c, &vm_x87(guest), want) {
            diffs.push((format!("{prefix}{k}"), d));
        }
    }
    diffs
}

/// `judge_fault` (vm_oracle 96-123): a fault the VM's guest runtime recorded against the CONTEXT Windows gave its
/// vectored handler, and the YMM upper halves and x87 state as the fault left them.
fn judge_fault(guest: &Guest, label: &str, c: &Case, r: &CaseResult, exc: &Exception) -> Vec<(String, String)> {
    if r.marker != 2 {
        return vec![(
            "vm fault, cpu completed".into(),
            format!("vm 0x{:x} at 0x{:x}", exc.code, exc.rip),
        )];
    }
    let mut diffs: Vec<(String, String)> = Vec::new();
    if u64::from(exc.code) != r.fault {
        diffs.push(("fault code".into(), format!("vm 0x{:x}, cpu 0x{:x}", exc.code, r.fault)));
    }
    if exc.rip != r.fault_rip {
        diffs.push((
            "fault rip".into(),
            format!("vm 0x{:x}, cpu 0x{:x}", exc.rip, r.fault_rip),
        ));
    }
    if exc.address != exc.rip {
        diffs.push((
            "fault address".into(),
            format!("ExceptionAddress 0x{:x}, Rip 0x{:x}", exc.address, exc.rip),
        ));
    }
    for (k, (vm, cpu)) in exc.gpr.iter().zip(r.regs).enumerate() {
        if *vm != cpu {
            diffs.push((format!("fault r{k}"), format!("vm 0x{vm:x}, cpu 0x{cpu:x}")));
        }
    }
    if (u64::from(exc.eflags) ^ r.rflags) & EFLAGS_FAULT_MASK != 0 {
        diffs.push((
            "fault eflags".into(),
            format!("vm 0x{:x}, cpu 0x{:x}", exc.eflags, r.rflags),
        ));
    }
    if u64::from(exc.mxcsr) != r.mxcsr {
        diffs.push((
            "fault MXCSR".into(),
            format!("vm 0x{:x}, cpu 0x{:x}", exc.mxcsr, r.mxcsr),
        ));
    }
    for (k, (vm, cpu)) in exc.xmm.iter().zip(r.xmm).enumerate() {
        if *vm != cpu {
            diffs.push((format!("fault xmm{k}"), format!("vm {vm:032x}, cpu {cpu:032x}")));
        }
    }
    diffs.extend(judge_ymm_x87(guest, label, c, r, "fault "));
    diffs.extend(windows(guest, r));
    diffs
}

/// Runs `cases` with their Windows `results` in one VM on `vcpu`, judging each completed case the compiler takes
/// (vm_oracle 175-258 for completed cases). `guest` is a `Guest` built on `case_memory()`.
pub fn run_cases(
    guest: &mut Guest,
    vcpu: &Vcpu<'_>,
    cases: &[Case],
    results: &[CaseResult],
    compile: Compile<'_>,
) -> Result<Tally, HvError> {
    let started = Instant::now();
    let md = MONITOR_DATA;
    let mut tally = Tally::default();
    {
        let mem = guest.memory.as_mut_slice();
        write_u64(mem, md + 8, 1 << 40); // the run's limit, one thread
        write_u64(mem, VM_COMPILER + VMC_ENABLED, 0); // lockstep: every miss reaches the host
        write_u64(mem, md + LOCK_OWNER, 1);
        mem[(md + MXCSR_OFF) as usize..(md + MXCSR_OFF) as usize + 4].copy_from_slice(&0x1F80u32.to_le_bytes());
        mem[(md + FCW_OFF) as usize..(md + FCW_OFF) as usize + 2].copy_from_slice(&0x27Fu16.to_le_bytes());
    }
    set_system_registers(vcpu, guest.ttbr)?;
    vcpu.set_reg(Reg::Fpcr, fpcr_for(0x1F80))?;
    vcpu.set_reg(Reg::X(28), MONITOR_VA + MONITOR_DATA)?;
    // A stub that loads Q0-Q15, FPCR and FPSR from the thread's vector slot and enters the dispatcher's re-dispatch:
    // the vector half of the switch stub's load (vm_backend 2690-2694). Hypervisor.framework sets a vector register only
    // by a by-value SIMD argument, which stable Rust cannot pass, so the guest loads its own (`set_mxcsr`, 2072-2075).
    // the code cache and reconstruction table as this call found them, put back at its end, so every call lays its
    // blocks out alike
    let found = {
        let mem = guest.memory.as_slice();
        (
            read_u64(mem, VM_COMPILER + VMC_CURSOR),
            read_u64(mem, RUN_TABLE + RECON_COUNT),
        )
    };
    let vectors_va = {
        let mem = guest.memory.as_mut_slice();
        let off = read_u64(mem, VM_COMPILER + VMC_CURSOR);
        let mut a = Assembler::new(MONITOR_VA + off);
        a.emit(&a64_encoder::mov_imm(16, MONITOR_VA + VECTOR_SLOTS));
        for q in (0..16).step_by(2) {
            a.emit(&[a64_encoder::ldp_q(q, q + 1, 16, 16 * i64::from(q))]);
        }
        a.emit(&[
            a64_encoder::ldp(18, 25, 16, 256),
            a64_encoder::msr(a64_encoder::FPCR, 18),
            a64_encoder::msr(a64_encoder::FPSR, 25),
        ]);
        a.branch(a64_encoder::b, Target::Va(guest.code.miss_va + 4));
        let bytes: Vec<u8> = a.finish().iter().flat_map(|w| w.to_le_bytes()).collect();
        mem[off as usize..off as usize + bytes.len()].copy_from_slice(&bytes);
        write_u64(mem, VM_COMPILER + VMC_CURSOR, off + bytes.len() as u64);
        MONITOR_VA + off
    };
    // the flush point (vm_backend 2511, 2786-2796): the code cache and reconstruction table after construction
    let (cursor0, recon0) = {
        let mem = guest.memory.as_slice();
        (
            read_u64(mem, VM_COMPILER + VMC_CURSOR),
            read_u64(mem, RUN_TABLE + RECON_COUNT),
        )
    };
    let offset = |g: &Guest, va: u64| g.layout.offset(va).unwrap() as usize;
    // the empty dispatch table (2795-2796), built once and copied whole per case: written entry by entry, an
    // unoptimized build took over an hour for the cases a release build runs in 33 s
    let empty_table: Vec<u8> = (0..DISPATCH_ENTRIES)
        .flat_map(|_| {
            let mut e = [0u8; 16];
            e[..8].copy_from_slice(&EMPTY_TARGET.to_le_bytes());
            e[8..].copy_from_slice(&guest.code.miss_va.to_le_bytes());
            e
        })
        .collect();
    for (i, (c, r)) in cases.iter().zip(results).enumerate() {
        let d = match x86_decoder::decode(&c.code, None) {
            Ok(d) => d,
            Err(e) => {
                *tally.refused.entry(format!("decode: {e}")).or_default() += 1;
                continue;
            }
        };
        if cpu_cases::undefined_memory(c).is_some() {
            tally.undefined_memory += 1;
            continue;
        }
        if r.marker == 2 {
            tally.windows_faults += 1;
        }
        // `flush_translations` (2782-2797): the code cache, the reconstruction table and the dispatch table as built
        let block = {
            let mem = guest.memory.as_mut_slice();
            write_u64(mem, VM_COMPILER + VMC_CURSOR, cursor0);
            write_u64(mem, RUN_TABLE + RECON_COUNT, recon0);
            let at = DISPATCH_TABLE as usize;
            mem[at..at + empty_table.len()].copy_from_slice(&empty_table);
            match assemble_one(
                compile,
                &c.code,
                CODE_VA,
                d.length as u64,
                MONITOR_VA + cursor0,
                guest.code.miss_va,
                guest.slice_va,
            ) {
                Ok(b) => b,
                Err(why) => {
                    *tally.refused.entry(why).or_default() += 1;
                    continue;
                }
            }
        };
        // `start_next` (208-234): the case's code, data, edge and stack, then its registers and flags
        {
            let (code_at, data_at, stack_at) =
                (offset(guest, CODE_VA), offset(guest, DATA_VA), offset(guest, STACK_VA));
            let mem = guest.memory.as_mut_slice();
            mem[code_at..code_at + 0x1000].fill(0);
            mem[code_at..code_at + c.code.len()].copy_from_slice(&c.code);
            mem[data_at..data_at + 0x1000].fill(0);
            let (o, e) = (data_at + DATA_OFF as usize, data_at + EDGE_OFF as usize);
            mem[o..o + WINDOW].copy_from_slice(&c.data);
            mem[e..e + EDGE].copy_from_slice(&c.edge);
            mem[stack_at..stack_at + STACK_SIZE as usize].fill(0);
            let s = stack_at + STACK_OFF as usize;
            mem[s..s + WINDOW].copy_from_slice(&c.stack);
            let (words, marks) = &block;
            let at = cursor0 as usize;
            let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
            mem[at..at + bytes.len()].copy_from_slice(&bytes);
            write_u64(mem, VM_COMPILER + VMC_CURSOR, cursor0 + bytes.len() as u64);
            for (k, (host, x86)) in marks.iter().enumerate() {
                let slot = RECON_TABLE + 16 * (recon0 + k as u64);
                write_u64(mem, slot, *host);
                write_u64(mem, slot + 8, *x86);
            }
            write_u64(mem, RUN_TABLE + RECON_COUNT, recon0 + marks.len() as u64);
            dispatch_insert(mem, CODE_VA, MONITOR_VA + cursor0);
            // set_af_df (2089-2092): lazy operands that read back as AF and PF, and DF
            let bit = |f: char| c.rflags >> FLAG_BITS.iter().find(|(k, _)| *k == f).unwrap().1 & 1;
            write_u64(mem, md + AF_OPERANDS, if bit('A') == 1 { 0x10 } else { 0 });
            write_u64(mem, md + AF_OPERANDS + 8, 0);
            write_u64(mem, md + PF_RESULT, if bit('P') == 1 { 0 } else { 1 });
            mem[(md + DF_BYTE) as usize] = (c.rflags >> DF_BIT & 1) as u8;
            write_u64(mem, md, 0); // the count
            // set_mxcsr (2072-2075): MXCSR stored, XMM0-15 and FPCR from it into the vector slot the stub loads
            mem[(md + MXCSR_OFF) as usize..(md + MXCSR_OFF) as usize + 4]
                .copy_from_slice(&((c.mxcsr & 0xFFFF) as u32).to_le_bytes());
            for (k, x) in c.xmm.iter().enumerate() {
                let at = (VECTOR_SLOTS + 16 * k as u64) as usize;
                mem[at..at + 16].copy_from_slice(&x.to_le_bytes());
            }
            write_u64(mem, VECTOR_SLOTS + 256, fpcr_for(c.mxcsr as u32));
            write_u64(mem, VECTOR_SLOTS + 264, 0);
            // set_ymm_hi (2047-2049)
            for (k, y) in c.ymm_hi.iter().enumerate() {
                let at = (md + YMM_HI_OFF + 16 * k as u64) as usize;
                mem[at..at + 16].copy_from_slice(&y.to_le_bytes());
            }
            // set_x87 (2060-2065): FCW in the monitor data, the status word, tags and ST0-ST7 in the state block
            if let Some(x) = c.x87 {
                mem[(md + FCW_OFF) as usize..(md + FCW_OFF) as usize + 2].copy_from_slice(&x.cw.to_le_bytes());
                let st = (MONITOR_SIZE + STATE_OFF) as usize;
                mem[st + X87_SW_OFF as usize..st + X87_SW_OFF as usize + 2].copy_from_slice(&x.sw.to_le_bytes());
                mem[st + X87_SW_OFF as usize + 2] = x.tags;
                for (k, r) in x.st.iter().enumerate() {
                    let at = st + X87_ST_OFF as usize + 16 * k;
                    mem[at..at + 10].copy_from_slice(&r.to_le_bytes()[..10]);
                }
            }
        }
        for (k, v) in c.regs.iter().enumerate() {
            vcpu.set_reg(Reg::X(k as u8), *v)?;
        }
        vcpu.set_reg(Reg::X(17), CODE_VA)?;
        for (f, reg) in FLAG_REG {
            let b = FLAG_BITS.iter().find(|(k, _)| *k == f).unwrap().1;
            vcpu.set_reg(Reg::X(reg), c.rflags >> b & 1)?;
        }
        vcpu.set_reg(Reg::X(REG_UNDEFINED_FLAGS), 0)?;
        vcpu.set_reg(Reg::X(REG_BUDGET as u8), 1 << 40)?;
        vcpu.set_reg(Reg::Cpsr, CPSR_EL1H)?;
        // the vectors loaded, then into the block through the dispatcher's re-dispatch, past the miss stub's hvc (its
        // cache maintenance first)
        vcpu.set_reg(Reg::Pc, vectors_va)?;
        let iclass = d.iclass.as_str();
        // the run to the case's end; a fault's walk asks the host through `hvc #8` until the process ends
        // (vm_backend 5246-5256)
        let exit = loop {
            let e = match vcpu.run()? {
                Exit::VtimerActivated => {
                    vcpu.set_vtimer_mask(true)?;
                    continue;
                }
                e => e,
            };
            if e.hvc_immediate() == Some(HVC_FAULT_GIVE_BACK) {
                // a guest fault left its block (5205-5206, 5294-5296): the faulting instruction and those after it go
                // back to the budget, then on to w2m_exception. A case's block holds one instruction, so
                // `_unrun_after` is 0 and one goes back.
                let left = vcpu.reg(Reg::X(REG_BUDGET as u8))?;
                vcpu.set_reg(Reg::X(REG_BUDGET as u8), left.wrapping_add(1))?;
                *tally.host_requests.entry(HOST_GIVE_BACK).or_default() += 1;
                continue;
            }
            if e.hvc_immediate() != Some(8) {
                break Ok(e);
            }
            let op = read_u64(guest.memory.as_slice(), MONITOR_SIZE + TRANSPORT_OFF);
            *tally.host_requests.entry(op).or_default() += 1;
            match host_request::serve(guest, vcpu) {
                Ok(Served::Answered) => {}
                Ok(Served::Exit(code)) => break Err(Ok(code)),
                Err(e) => break Err(Err(e.to_string())),
            }
        };
        let pc = vcpu.reg(Reg::Pc)?;
        let exit = match exit {
            Ok(e) => e,
            Err(ended) => {
                // `on_exception` (243-248): the records judged, then forgotten, and the next case
                let diffs = match (ended, guest.exception()) {
                    (Ok(_), Some(exc)) => judge_fault(guest, iclass, c, r, &exc),
                    (Ok(code), None) => vec![("vm exit".into(), format!("ExitProcess({code}) with no exception"))],
                    (Err(why), _) => vec![("host request".into(), why)],
                };
                guest.clear_exception();
                tally.judged_whole(iclass, r);
                tally.faults_judged += usize::from(r.marker == 2);
                for (k, detail) in diffs {
                    tally.note(iclass, &k, format!("case {i}: {:02x?} {detail}", c.code));
                }
                continue;
            }
        };
        if r.marker == 2 && exit.hvc_immediate() == Some(1) {
            // `judge_done` 127-128
            tally.judged += 1;
            tally.note(
                iclass,
                "vm completed, cpu fault",
                format!("case {i}: {:02x?} cpu 0x{:x}", c.code, r.fault),
            );
            continue;
        }
        match exit.hvc_immediate() {
            Some(1) => {}
            Some(HVC_SLOW_SSE | HVC_SLOW_STEP) => {
                tally.model += 1; // the model is step 4's: counted, not judged
                continue;
            }
            other => {
                // the VM stopped otherwise where Windows completed: a fault, an undefined flag read, a slice
                tally.note(
                    iclass,
                    "vm stopped",
                    format!("case {i}: {:02x?} {other:?} at pc 0x{pc:x}", c.code),
                );
                tally.judged += 1;
                continue;
            }
        }
        // `judge_done` (126-156)
        tally.judged_whole(iclass, r);
        let x17 = vcpu.reg(Reg::X(17))?;
        let path = match x17 {
            v if v == CODE_VA + c.code.len() as u64 => Some(0),
            v if v == CODE_VA + c.code.len() as u64 + 14 || v == RET_TARGET => Some(1),
            _ => None,
        };
        let example = |detail: String| format!("case {i}: {:02x?} {detail}", c.code);
        if path != Some(r.marker) {
            tally.note(iclass, "rip", example(format!("vm 0x{x17:x}, cpu path {}", r.marker)));
        }
        for k in 0..16 {
            let v = vcpu.reg(Reg::X(k))?;
            if v != r.regs[usize::from(k)] {
                let cpu = r.regs[usize::from(k)];
                tally.note(iclass, &format!("r{k}"), example(format!("vm 0x{v:x}, cpu 0x{cpu:x}")));
            }
        }
        let undefined = vcpu.reg(Reg::X(REG_UNDEFINED_FLAGS))?;
        let mem = guest.memory.as_slice();
        for (f, b) in FLAG_BITS {
            let u = FLAG_BIT.iter().find(|(k, _)| *k == f).unwrap().1;
            if undefined >> u & 1 == 1 {
                continue;
            }
            let vm = match f {
                'A' => {
                    let (a, b2, r2) = (
                        read_u64(mem, md + AF_OPERANDS),
                        read_u64(mem, md + AF_OPERANDS + 8),
                        read_u64(mem, PF_RESULT + md),
                    );
                    (a ^ b2 ^ r2) & 0x10 != 0
                }
                'P' => parity_even(read_u64(mem, md + PF_RESULT)),
                _ => vcpu.reg(Reg::X(FLAG_REG.iter().find(|(k, _)| *k == f).unwrap().1))? != 0,
            };
            let cpu = r.rflags >> b & 1 == 1;
            if vm != cpu {
                tally.note(
                    iclass,
                    &format!("{f}F"),
                    example(format!("vm {}, cpu {}", u8::from(vm), u8::from(cpu))),
                );
            }
        }
        let df = mem[(md + DF_BYTE) as usize] != 0;
        if df != (r.rflags >> DF_BIT & 1 == 1) {
            tally.note(iclass, "DF", example(format!("vm {}", u8::from(df))));
        }
        // MXCSR (the stored word with FPSR's flags folded in) and XMM0-15 (judge_done 143-149)
        let stored = u64::from(u32::from_le_bytes(
            mem[(md + MXCSR_OFF) as usize..(md + MXCSR_OFF) as usize + 4]
                .try_into()
                .unwrap(),
        ));
        let mx = stored | mxcsr_flags_from_fpsr(vcpu.reg(Reg::Fpsr)?);
        if mx != r.mxcsr {
            let detail = format!("vm 0x{mx:x}, cpu 0x{:x} (in 0x{:x})", r.mxcsr, c.mxcsr);
            tally.note(iclass, "MXCSR", example(detail));
        }
        for k in 0..16u8 {
            let (vm, cpu) = (vcpu.simd(k)?, r.xmm[usize::from(k)]);
            if vm != cpu {
                tally.note(
                    iclass,
                    &format!("xmm{k}"),
                    example(format!("vm {vm:032x}, cpu {cpu:032x}")),
                );
            }
        }
        for (k, detail) in judge_ymm_x87(guest, iclass, c, r, "") {
            tally.note(iclass, &k, example(detail));
        }
        for (k, detail) in windows(guest, r) {
            tally.note(iclass, &k, example(detail));
        }
    }
    let mem = guest.memory.as_mut_slice();
    write_u64(mem, VM_COMPILER + VMC_CURSOR, found.0);
    write_u64(mem, RUN_TABLE + RECON_COUNT, found.1);
    tally.seconds = started.elapsed().as_secs_f64();
    Ok(tally)
}
