// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! One vCPU running one thread of a `Guest` (Sprint 142): `VM.steps` (`src/w2m/vm_backend.py` 4916-5127) for that
//! case, driven through `hypervisor` with no monitor process between. It serves the dispatch misses a bridge answers and
//! stops at the first miss that needs the host's compiler, which the translator's sprints bring. Each function cites
//! the lines it carries.

use std::collections::BTreeMap;
use std::time::Instant;

use guest_runtime::layout::{
    CODE_CACHE, CODE_CACHE_SIZE, DISPATCH_TABLE, MONITOR_DATA, MONITOR_SIZE, MONITOR_VA, PROCESS_VA, RUN_ENTRY,
    RUN_TABLE, RUNTIME_IMAGE, RUNTIME_IMAGE_SIZE, SLICE_UNTIL, STATE_OFF, TEB_OFF, TRANSPORT_OFF, VM_COMPILER, VMC_LOG,
    VMC_LOG_ENTRY,
};
use hypervisor::{Exit, HvError, Reg, Vcpu};
use pe_loader::Loaded;

use crate::CPSR_EL1H;
use crate::compile::HostCompiler;
use crate::guest::{Guest, HVC_SCHEDULE, RT_IN_VM, STATE_SCHEDULE, VMC_COUNT_NAMES, VMC_COUNTS, VMC_ENABLED};
use crate::host_request::{HOST_THREAD, THREAD_END, THREAD_EXIT_CODE, THREAD_EXIT_STUB};
use crate::monitor::set_system_registers;
use crate::monitor_code::{
    EMPTY_TARGET, HVC_CODE_WRITE, HVC_FAULT_GIVE_BACK, LOCK_ENTRIES, LOCK_ENTRY, LOCK_EXITS, LOCK_TAKINGS,
    LOCK_TAKINGS_COUNT, LOCK_TIMING, REG_BUDGET, dispatch_slot,
};
use crate::threads::{STILL_ACTIVE, Switch, Thread, add_thread, slice_until};

/// MONITOR_DATA + these (vm_backend 96, 107, 113, 119).
pub const MXCSR_OFF: u64 = 0x50;
pub const FCW_OFF: u64 = 0x70;
pub const LOCK_OWNER: u64 = 0xB0;
pub const RUN_END: u64 = 0xC8;
/// x24 holds the flags not yet defined (vm_backend 359); at entry every one.
pub const REG_UNDEFINED_FLAGS: u8 = 24;

/// The blocks' instructions in the code cache (vm_backend `tail_hosts`, `tail_left`, 2466-2467): each instruction's
/// first host address, in the order the cache grows, and the instructions after it in its block.
#[derive(Debug, Default, Clone)]
pub struct Tails {
    hosts: Vec<u64>,
    left: Vec<u64>,
}

impl Tails {
    /// `_note_tails` (4644-4647): a new block's instruction marks.
    pub fn note(&mut self, marks: &[u64]) {
        self.hosts.extend(marks);
        self.left.extend((0..marks.len() as u64).rev());
    }

    /// `flush_translations` (2789-2790).
    pub fn clear(&mut self) {
        self.hosts.clear();
        self.left.clear();
    }

    /// `_unrun_after` (4652-4659): the instructions after the one whose code holds host address `pc` in its block.
    pub fn unrun_after(&self, pc: u64) -> u64 {
        if !in_code_cache(pc) {
            return 0;
        }
        let i = self.hosts.partition_point(|h| *h <= pc);
        if i > 0 { self.left[i - 1] } else { 0 }
    }
}

/// `_in_code_cache` (4649-4650).
pub fn in_code_cache(pc: u64) -> bool {
    (MONITOR_VA + CODE_CACHE..MONITOR_VA + CODE_CACHE + CODE_CACHE_SIZE).contains(&pc)
}

/// What an exit gives back to the budget (vm_backend 5202-5206).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GiveBack {
    None,
    /// `hvc #14` (`HVC_FAULT_GIVE_BACK`): x2 the faulting host address; the faulting instruction and those after it
    Fault,
    /// `hvc #10` or `#11`, a model step at host address `pc`: the block's instructions after the step
    Model {
        pc: u64,
    },
}

/// An exit outside the runtime (5198-5208): the budget left in x23, with the exit's give-back, then the count (limit -
/// left) written to the monitor data.
pub fn exit_count(vcpu: &Vcpu<'_>, mem: &mut [u8], tails: &Tails, give_back: GiveBack) -> Result<(), HvError> {
    let md = MONITOR_DATA;
    let limit = read_u64(mem, md + 8);
    let mut left = vcpu.reg(Reg::X(REG_BUDGET as u8))?;
    match give_back {
        GiveBack::None => {}
        GiveBack::Model { pc } => {
            left = left.wrapping_add(tails.unrun_after(pc - 4)); // the block's instructions after the step
            vcpu.set_reg(Reg::X(REG_BUDGET as u8), left)?;
        }
        GiveBack::Fault => {
            let x2 = vcpu.reg(Reg::X(2))?;
            if in_code_cache(x2) {
                left = left.wrapping_add(tails.unrun_after(x2) + 1); // the faulting instruction and those after it
            }
            vcpu.set_reg(Reg::X(REG_BUDGET as u8), left)?;
        }
    }
    write_u64(mem, md, limit.wrapping_sub(left));
    Ok(())
}
/// VM_COMPILER + this: the log's entry count (vm_backend 197).
pub const VMC_LOG_COUNT: u64 = 0x10;

/// `fpcr_for` (vm_backend 1138-1143): FPCR for an MXCSR: AH always; FIZ = DAZ; FZ = FTZ; RMode from RC.
pub fn fpcr_for(mxcsr: u32) -> u64 {
    let rmode = [0, 2, 1, 3][(mxcsr >> 13 & 3) as usize];
    1 << 1 | u64::from(mxcsr >> 6 & 1) | u64::from(mxcsr >> 15 & 1) << 24 | rmode << 22
}

/// Why the run stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stop {
    /// a dispatch miss at `target` that is neither a bridge nor a block: the host's compiler's work
    Declined { target: u64 },
    /// a miss at a target whose bridge was inserted and missed again
    Repeated { target: u64 },
    /// an `hvc` this run does not serve, at `pc`
    Hvc { number: u16, pc: u64 },
    /// a host request (`hvc #8`) this run does not serve: the transport page's op (Sprint 155), and why: the host's
    /// refusal, naming the request (Sprint 156)
    HostRequest { op: u64, pc: u64, why: String },
    /// a host request the host serves that failed in the serving: guest memory or the framework refused (Sprint 210;
    /// reviewer-3's High 2 of its review of Sprint 210: such a failure had been recorded as a service not implemented)
    HostFailure { op: u64, pc: u64, why: String },
    /// the guest ended its process (HOST_EXIT) with `code`
    ExitProcess { code: u32 },
    /// a block the host could not compile (Sprint 155): an instruction the Rust rules do not lift, a CompileError
    Untranslated { target: u64, why: String },
    /// an `hvc #12` hand-over the host could not serve (Sprint 157), and why
    Handover { pc: u64, why: String },
    /// a model exit (hvc 10 or 11) the model could not step: an operation it does not run, or a flag read while
    /// undefined (Sprint 181), at guest `rip`
    Model { rip: u64, why: String },
    /// any other exit
    Exit(String),
}

/// `x86_address_at` (vm_backend 4861-4863): the guest address of the host code at `pc`, from the reconstruction table
/// (the entry with the largest host address at or below `pc`, as `bisect_right` over the sorted marks finds it); 0
/// when none is (Sprint 181). The count is the guest's to write, so a count past the table's `RECON_ENTRIES` or the
/// memory is an error, not a slice past the end (Sprint 188; reviewer-2's Low 4 of its review of 182-184).
pub fn x86_address_at(mem: &[u8], pc: u64) -> Result<u64, String> {
    use guest_runtime::layout::{RECON_COUNT, RECON_ENTRIES, RECON_TABLE, RUN_TABLE};
    recon_lookup(
        mem,
        RECON_TABLE,
        read_u64(mem, RUN_TABLE + RECON_COUNT),
        RECON_ENTRIES,
        pc,
    )
}

/// `bisect_right(pc_map, (pc, M64)) - 1` over the first `n` (host, x86) entries at `table`, kept sorted as the code
/// cache grows (vm_backend 4626-4628): the x86 address of the last entry whose host address is at or below `pc`, 0
/// below the first (Sprint 184).
fn recon_lookup(mem: &[u8], table: u64, n: u64, capacity: u64, pc: u64) -> Result<u64, String> {
    let end = table
        .checked_add(n.saturating_mul(16))
        .filter(|e| *e <= mem.len() as u64);
    let (Some(end), true) = (end, n <= capacity) else {
        return Err(format!(
            "the reconstruction table's count {n} is past its {capacity} entries or the memory"
        ));
    };
    let (entries, _) = mem[table as usize..end as usize].as_chunks::<16>();
    let entry = |e: &[u8; 16]| {
        (
            u64::from_le_bytes(e[..8].try_into().unwrap()),
            u64::from_le_bytes(e[8..].try_into().unwrap()),
        )
    };
    Ok(match entries.partition_point(|e| entry(e) <= (pc, u64::MAX)) {
        0 => 0,
        i => entry(&entries[i - 1]).1,
    })
}

/// The instruction bytes at guest `rip` for the model's lifter: up to 15 bytes of mapped memory, then 16 zeros, as
/// `lift(code + bytes(16), ...)` pads them (Sprint 181).
fn code_at(guest: &Guest, rip: u64) -> Vec<u8> {
    let mut code: Vec<u8> = (0..15u64)
        .map_while(|i| {
            guest
                .layout
                .offset(rip + i)
                .map(|o| guest.memory.as_slice()[o as usize])
        })
        .collect();
    code.extend([0u8; 16]);
    code
}

/// One taking of the runtime word as the guest timed it (Sprint 164; `monitor_code::LOCK_TIMING`): virtual-counter
/// ticks at the wait's start, the taking and the release, its site, and the host's exit count at each of the three.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Taking {
    pub site: u64,
    pub wait_start: u64,
    pub taken: u64,
    pub released: u64,
    pub exits_at_wait_start: u64,
    pub exits_at_taken: u64,
    pub exits_at_released: u64,
}

/// The runtime word's takings the guest timed: how many it counted, and those it wrote (no more than
/// `LOCK_TAKINGS`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeWordTimes {
    pub counted: u64,
    pub takings: Vec<Taking>,
}

/// The timing area as the guest left it.
pub fn read_runtime_word(mem: &[u8]) -> RuntimeWordTimes {
    let counted = read_u64(mem, LOCK_TIMING + LOCK_TAKINGS_COUNT);
    let takings = (0..counted.min(LOCK_TAKINGS))
        .map(|i| {
            let e = LOCK_TIMING + LOCK_ENTRIES + LOCK_ENTRY * i;
            let w = |k: u64| read_u64(mem, e + 8 * k);
            Taking {
                wait_start: w(0),
                taken: w(1),
                released: w(2),
                site: w(3),
                exits_at_wait_start: w(4),
                exits_at_taken: w(5),
                exits_at_released: w(6),
            }
        })
        .collect();
    RuntimeWordTimes { counted, takings }
}

/// What a run did.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub stop: Stop,
    /// the misses a bridge answered, by thunk
    pub bridged: BTreeMap<u64, u64>,
    /// the blocks the C compiler compiled: x86 start -> code VA
    pub blocks: BTreeMap<u64, u64>,
    /// `VMC_COUNTS` at the stop, by name
    pub counts: BTreeMap<&'static str, u64>,
    /// each served exit's host time, from the vCPU's exit to its next run, in ns
    pub serve_ns: Vec<u64>,
    /// the same times by the exit's kind (`hvc <k>`, `vtimer`, or `ec 0x<class>`), for step 3's measure (Sprint 163)
    pub serve_by_kind: BTreeMap<String, Vec<u64>>,
    /// the runtime word's takings when the guest times them (Sprint 164)
    pub runtime_word: RuntimeWordTimes,
    /// each run's time, from `hv_vcpu_run`'s call to its return, in ns
    pub run_ns: Vec<u64>,
    /// the exits, by hvc number (0 for a non-hvc exit)
    pub exits: BTreeMap<u16, u64>,
    /// the instructions executed at the stop, the monitor's count the exit ledger's executed column reads
    /// (`vm.instructions_executed()`; Sprint 210, SERVICE_UNIMPLEMENTED's `instructions_executed`)
    pub executed: u64,
    /// each thread the guest created, as CreateThread's host half left it (Sprint 156)
    pub created: Vec<Created>,
    /// each thread request THREAD_EXIT_STUB, THREAD_END and THREAD_EXIT_CODE the host served, in order (Sprint 157)
    pub thread_requests: Vec<ThreadRequest>,
    /// each `hvc #12` hand-over the host served, in order (Sprint 157)
    pub handovers: Vec<Handover>,
    /// every block translated, the C's and the host's, in the order the Python's `vm.translations` keeps (Sprint 158):
    /// the C's log at each exit (`_vm_compiled` 2577), the host's at its compile (`_compile` 4638)
    pub translations: Vec<Translation>,
}

/// A translated block: its guest range [start, end) and its code address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Translation {
    pub start: u64,
    pub end: u64,
    pub code: u64,
}

/// A thread request the host served: the sub-request, its first argument, the answer and the thread current at it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadRequest {
    pub sub: u64,
    pub a0: u64,
    pub answer: u64,
    pub current: u64,
}

/// A hand-over: state.schedule's bits, the thread that handed vCPU 0 over and the one current after; 0 when the
/// process ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handover {
    pub schedule: u32,
    pub from: u64,
    pub to: u64,
}

/// A created thread at THREAD_CREATE's return: its id, its run table entry and vector slot, and the count and limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Created {
    pub tid: u64,
    pub entry: Vec<u8>,
    pub vectors: Vec<u8>,
    pub count: u64,
    pub limit: u64,
}

/// vm_monitor.c `vswap` (440-482): Q0-Q15, FPCR and FPSR to the outgoing thread's slot at memory offset `save`, then
/// from the incoming thread's slot at `load`. A slot is 16 registers of 16 bytes, then FPCR and FPSR of 8 each.
fn swap_vectors(vcpu: &Vcpu<'_>, mem: &mut [u8], save: u64, load: u64) -> Result<(), HvError> {
    for n in 0..16u8 {
        let at = (save + 16 * u64::from(n)) as usize;
        mem[at..at + 16].copy_from_slice(&vcpu.simd(n)?.to_le_bytes());
    }
    write_u64(mem, save + 256, vcpu.reg(Reg::Fpcr)?);
    write_u64(mem, save + 264, vcpu.reg(Reg::Fpsr)?);
    for n in 0..16u8 {
        let at = (load + 16 * u64::from(n)) as usize;
        vcpu.set_simd(n, u128::from_le_bytes(mem[at..at + 16].try_into().unwrap()))?;
    }
    vcpu.set_reg(Reg::Fpcr, read_u64(mem, load + 256))?;
    vcpu.set_reg(Reg::Fpsr, read_u64(mem, load + 264))
}

fn read_u64(mem: &[u8], off: u64) -> u64 {
    u64::from_le_bytes(mem[off as usize..off as usize + 8].try_into().unwrap())
}

fn write_u64(mem: &mut [u8], off: u64, v: u64) {
    mem[off as usize..off as usize + 8].copy_from_slice(&v.to_le_bytes());
}

/// `_dispatch_insert` (vm_backend 4827-4849): (target, va) in its pair, in a slot that is empty or holds it, else
/// first, the first slot's entry moved to the second. One vCPU: no other reader, so no barriers.
pub fn dispatch_insert(mem: &mut [u8], target: u64, va: u64) {
    let at = DISPATCH_TABLE + 16 * dispatch_slot(target);
    let (t0, v0, t1) = (read_u64(mem, at), read_u64(mem, at + 8), read_u64(mem, at + 16));
    let mut publish = |slot: u64, t: u64, v: u64| {
        write_u64(mem, slot + 8, v);
        write_u64(mem, slot, t);
    };
    if t0 == EMPTY_TARGET || t0 == target {
        publish(at, target, va);
    } else if t1 == EMPTY_TARGET || t1 == target {
        publish(at + 16, target, va);
    } else {
        publish(at + 16, t0, v0);
        publish(at, target, va);
    }
}

/// The process area from `PROCESS_VA` up to the trace (`TRACE_OFF`), the exit ledger's last column (Sprint 209).
fn process_below_trace(mem: &[u8]) -> &[u8] {
    let at = MONITOR_SIZE as usize;
    &mem[at..at + guest_runtime::layout::TRACE_OFF as usize]
}

/// `_vm_compiled` (vm_backend 2559-2580): each block block_compiler.c compiled since the last exit, from its log entry
/// (start, end, code, then u32s first, nmarks, count, cut, live_out, live_used, npages, and the pages at 56), noted as
/// the host notes its own compiles: the marks' tails (`_note_tails`, so a model step's give-back counts the block's
/// unrun instructions; Sprint 207: before, a model step in a block the VM compiled gave nothing back, and the count ran
/// 7 instructions ahead of the frozen host's at the batch run's first `hvc 11`), the block, the pages it covers and the
/// pages it lists, and the translation; then the log emptied.
fn vm_compiled(
    mem: &mut [u8],
    blocks: &mut BTreeMap<u64, u64>,
    translations: &mut Vec<Translation>,
    mut host: Option<&mut HostCompiler>,
) {
    use guest_runtime::layout::{PAGE, RECON_TABLE};
    let n = read_u64(mem, VM_COMPILER + VMC_LOG_COUNT);
    for i in 0..n {
        let at = VM_COMPILER + VMC_LOG + VMC_LOG_ENTRY * i;
        let (start, end, code) = (read_u64(mem, at), read_u64(mem, at + 8), read_u64(mem, at + 16));
        blocks.insert(start, code);
        translations.push(Translation { start, end, code });
        let Some(h) = host.as_deref_mut() else {
            continue;
        };
        let u32_at = |off: u64| {
            u64::from(u32::from_le_bytes(
                mem[(at + off) as usize..(at + off + 4) as usize].try_into().unwrap(),
            ))
        };
        let (first, count, npages) = (u32_at(24), u32_at(32), u32_at(48));
        let hosts: Vec<u64> = (1..=count)
            .map(|k| read_u64(mem, RECON_TABLE + 16 * (first + k)))
            .collect();
        h.tails.note(&hosts);
        h.blocks.insert(start, code);
        let mut page = start & !(PAGE - 1);
        while page < end {
            h.code_pages.entry(page).or_default().insert(start);
            page += PAGE;
        }
        for k in 0..npages {
            h.code_pages
                .entry(read_u64(mem, at + 56 + 8 * k))
                .or_default()
                .insert(start);
        }
        h.translations.push((start, end));
    }
    write_u64(mem, VM_COMPILER + VMC_LOG_COUNT, 0);
}

/// Runs `guest`'s program from its entry on `vcpu`, one thread, until the first miss the host must compile or an exit
/// this run does not serve (`VM.steps` 4964-5095 and `serve_exit` 5197-5243 for one vCPU and one thread).
pub fn run_to_first_decline(
    guest: &mut Guest,
    loaded: &Loaded,
    vcpu: &Vcpu<'_>,
    max_steps: u64,
) -> Result<Outcome, HvError> {
    run(guest, loaded, vcpu, max_steps, None)
}

/// As `run_to_first_decline`, with the host compiling the blocks the VM's C declines (Sprint 155: `serve_exit`'s
/// `_compile`, 5243, and its code-write exit, 5297-5299), until an exit this run does not serve or a block it cannot
/// compile.
/// `_give_back` with `_stop_block_length` (vm_backend 4862-4878): a thread leaving at a block's stop goes on at the
/// block's entry, whose SUBS counts the block again, so the block's instructions come off the count. `after_stop` is
/// the word after the stop's `hvc #4`, the B to the block's body; the SUBS x23, x23, #n is two words before the body.
fn give_back(guest: &mut Guest, after_stop: u64) -> Result<(), String> {
    let mem = guest.memory.as_mut_slice();
    let word_at = |mem: &[u8], off: u64| u32::from_le_bytes(mem[off as usize..off as usize + 4].try_into().unwrap());
    let off = after_stop - MONITOR_VA;
    let imm26 = i64::from(word_at(mem, off) & 0x3FF_FFFF);
    let body = (off as i64 + 4 * if imm26 >> 25 != 0 { imm26 - (1 << 26) } else { imm26 }) as u64;
    let subs = word_at(mem, body - 8);
    let reg = REG_BUDGET;
    if subs & !(0xFFF << 10) != 0xF100_0000 | reg << 5 | reg {
        return Err(format!(
            "the word before the block's body at 0x{body:x} is 0x{subs:08x}, not its SUBS"
        ));
    }
    let count = read_u64(mem, MONITOR_DATA);
    write_u64(mem, MONITOR_DATA, count - u64::from(subs >> 10 & 0xFFF));
    Ok(())
}

/// The process's turn ends (Sprint 197: `VM.steps` yields "idle" or "slice", vm_backend 5066, 5082): the machine runs
/// the others until this process's turn comes back; a pending wake readies its blocked threads, and on the instruction
/// clock it is caught up to the machine's (`_catch_up`). Err: the run stops (a deadlock, a child's stop).
fn hand_over(guest: &mut Guest, message: crate::turns::Message) -> Result<(), String> {
    use crate::turns::{Back, Link};
    let mut state = crate::threads::turn_state(guest);
    state.clock = crate::threads::counter_now(guest);
    let (wake, machine, processes) = match guest.link.take() {
        Some(Link::Supervisor(mut s)) => {
            let mut objects = std::mem::take(&mut guest.kernel);
            let back = s.hand_over(&mut objects, state);
            let processes = 1 + s.children.len();
            guest.kernel = objects;
            guest.link = Some(Link::Supervisor(s));
            match back? {
                Back::Run { wake, machine } | Back::Timed { wake, machine } => (wake, machine, processes),
            }
        }
        Some(Link::Child(c)) => {
            let (wake, machine) = c.yield_turn(message, state);
            guest.link = Some(Link::Child(c));
            (wake, machine, 2)
        }
        None => (false, 0, 1),
    };
    if wake {
        crate::host_request::wake_blocked(guest);
    }
    crate::threads::catch_up(guest, machine, processes);
    Ok(())
}

pub fn run_compiling(
    guest: &mut Guest,
    loaded: &Loaded,
    vcpu: &Vcpu<'_>,
    max_steps: u64,
    host: &mut HostCompiler,
) -> Result<Outcome, HvError> {
    run(guest, loaded, vcpu, max_steps, Some(host))
}

fn run(
    guest: &mut Guest,
    loaded: &Loaded,
    vcpu: &Vcpu<'_>,
    max_steps: u64,
    mut host: Option<&mut HostCompiler>,
) -> Result<Outcome, HvError> {
    let md = MONITOR_DATA;
    let first = Thread {
        // the start's thread id, a child's from the Machine (`_first_tid`, vm_backend 2497; Sprint 206: it was 4)
        tid: guest.start.tid,
        index: 0,
        teb: PROCESS_VA + TEB_OFF,
        stack: guest.process.stack,
        exit_code: STILL_ACTIVE,
        parent: 0,
        start: loaded.entry,
        created_suspended: false,
    };
    let mut out = Outcome {
        stop: Stop::Exit(String::new()),
        bridged: BTreeMap::new(),
        blocks: BTreeMap::new(),
        counts: BTreeMap::new(),
        serve_ns: Vec::new(),
        serve_by_kind: BTreeMap::new(),
        runtime_word: RuntimeWordTimes::default(),
        run_ns: Vec::new(),
        exits: BTreeMap::new(),
        executed: 0,
        created: Vec::new(),
        thread_requests: Vec::new(),
        handovers: Vec::new(),
        translations: Vec::new(),
    };
    if let Err(why) = add_thread(guest, first) {
        out.stop = Stop::Exit(format!("the first thread's entry: {why}")); // a guest run before
        return Ok(out);
    }
    {
        // `_set_budget`: one thread, the run's limit; a child shares the vCPU from its first instruction (Sprint 197)
        let shared = guest.link.is_some();
        let mem = guest.memory.as_mut_slice();
        write_u64(
            mem,
            md + 8,
            if shared {
                max_steps.min(guest_runtime::layout::TIME_SLICE)
            } else {
                max_steps
            },
        );
        write_u64(mem, VM_COMPILER + VMC_ENABLED, 1);
        mem[(md + MXCSR_OFF) as usize..(md + MXCSR_OFF) as usize + 4].copy_from_slice(&0x1F80u32.to_le_bytes());
        write_u64(mem, md + LOCK_OWNER, 1);
        mem[(md + FCW_OFF) as usize..(md + FCW_OFF) as usize + 2].copy_from_slice(&0x27Fu16.to_le_bytes());
    }
    set_system_registers(vcpu, guest.ttbr)?;
    vcpu.set_reg(Reg::Fpcr, fpcr_for(0x1F80))?;
    for r in 0..31u8 {
        vcpu.set_reg(Reg::X(r), 0)?;
    }
    vcpu.set_reg(Reg::X(4), loaded.stack_top - 0x100 - 8)?; // as runtime.Runtime: RSP % 16 == 8
    vcpu.set_reg(Reg::X(17), loaded.entry)?;
    vcpu.set_reg(Reg::X(REG_UNDEFINED_FLAGS), 0x3F)?;
    vcpu.set_reg(Reg::X(28), MONITOR_VA + MONITOR_DATA)?;
    vcpu.set_reg(Reg::Pc, guest.entry_va)?;
    vcpu.set_reg(Reg::Cpsr, CPSR_EL1H)?;

    let runtime = MONITOR_VA + RUNTIME_IMAGE..MONITOR_VA + RUNTIME_IMAGE + RUNTIME_IMAGE_SIZE;
    let mut marked = None::<Instant>;
    let mut kind = String::new(); // the last exit's kind, whose service `marked` times
    let stop = loop {
        // `_before_run` (5129-5141): one vCPU, no other process: the slice's end (`_slice_until`), the VM may switch
        // threads itself; x23 the budget left, outside the runtime's C
        let pc = vcpu.reg(Reg::Pc)?;
        {
            let slice_until = slice_until(guest);
            let mem = guest.memory.as_mut_slice();
            let (count, limit) = (read_u64(mem, md), read_u64(mem, md + 8));
            write_u64(mem, md + SLICE_UNTIL, slice_until);
            write_u64(mem, md + RUN_END, max_steps);
            // the VM switches threads itself unless another process is due (Sprint 197)
            let others_due = guest
                .link
                .as_ref()
                .is_some_and(|l| l.others_due(crate::threads::counter_now(guest)));
            let mem = guest.memory.as_mut_slice();
            write_u64(mem, RUN_TABLE + u64::from(RT_IN_VM), u64::from(!others_due));
            if !runtime.contains(&pc) {
                vcpu.set_reg(Reg::X(REG_BUDGET as u8), limit.wrapping_sub(count))?;
            }
        }
        if let Some(t) = marked {
            let ns = t.elapsed().as_nanos() as u64;
            out.serve_ns.push(ns);
            out.serve_by_kind.entry(kind.clone()).or_default().push(ns);
        }
        let started = Instant::now();
        let exit = vcpu.run()?;
        // the run's time ends here, before the ledger hashes the process area (reviewer-3's Medium 1 of its review of
        // Sprint 210: `run_seconds` had counted the hashing)
        let returned = Instant::now();
        // the exit ledger's row for the last exit, the count and warp as this exit finds them, before its own refresh,
        // as the frozen host's fixture reads them at `serve_exit`'s entry: a bridge stores the count into the word as
        // the guest enters a service, so the word read before the resume lags it (Sprint 207)
        if let Some(l) = &mut guest.exit_ledger {
            let mem = guest.memory.as_slice();
            let warp = read_u64(mem, RUN_TABLE + crate::process::CLOCK_WARP);
            if let Err(why) = l.resume(read_u64(mem, md), warp, process_below_trace(mem)) {
                break Stop::Exit(format!("the exit ledger: {why}"));
            }
        }
        if guest.time_runtime_word {
            // the exit counted where the guest's timing reads it, so a taking that spans an exit says so (Sprint 164)
            let at = (LOCK_TIMING + LOCK_EXITS) as usize;
            let word = &mut guest.memory.as_mut_slice()[at..at + 8];
            let n = u64::from_le_bytes(word.try_into().unwrap()) + 1;
            word.copy_from_slice(&n.to_le_bytes());
        }
        out.run_ns.push((returned - started).as_nanos() as u64);
        // the exit's kind, named before the service is timed, so the naming is not counted in it (reviewer's finding
        // of 2026-10-27: Sprint 163 named it after starting the timer)
        kind = match (exit.hvc_immediate(), &exit) {
            (Some(n), _) => format!("hvc {n}"),
            (None, Exit::VtimerActivated) => "vtimer".to_string(),
            (None, e) => e
                .exception_class()
                .map_or_else(|| format!("{e:?}"), |c| format!("ec 0x{c:02x}")),
        };
        marked = Some(Instant::now());
        let mem = guest.memory.as_mut_slice();
        vm_compiled(mem, &mut out.blocks, &mut out.translations, host.as_deref_mut());
        let pc = vcpu.reg(Reg::Pc)?;
        if !runtime.contains(&pc) {
            // the count, from x23, with a fault's give-back (5199-5208); the VM's own blocks' tails this loop does not
            // note yet (2570), the host's compiles' it does
            let give_back = match exit.hvc_immediate().map(u32::from) {
                Some(HVC_FAULT_GIVE_BACK) => GiveBack::Fault,
                Some(10 | 11) => GiveBack::Model { pc }, // HVC_SLOW_SSE, HVC_SLOW_STEP
                _ => GiveBack::None,
            };
            let none = Tails::default();
            let tails = host.as_deref().map_or(&none, |h| &h.tails);
            exit_count(vcpu, mem, tails, give_back)?;
        }
        if guest.exit_ledger.is_some() {
            // the row: kind, x86 site (the dispatch target for hvc 1, else the hvc's), and a request's op and arguments
            let site = match exit.hvc_immediate() {
                Some(1) => vcpu.reg(Reg::X(17))?,
                Some(_) => x86_address_at(mem, pc.wrapping_sub(4)).unwrap_or(0),
                None => pc,
            };
            let request = (exit.hvc_immediate() == Some(8)).then(|| {
                let t = MONITOR_SIZE + TRANSPORT_OFF;
                let w = |i: u64| read_u64(mem, t + 8 * i);
                [w(0), w(1), w(2), w(3), w(4), 0, 0]
            });
            if let Some(l) = &mut guest.exit_ledger {
                l.exit(crate::exit_ledger::Row {
                    kind: kind.clone(),
                    site,
                    request,
                });
            }
        }
        let mem = guest.memory.as_mut_slice();
        let Some(number) = exit.hvc_immediate() else {
            if matches!(exit, Exit::VtimerActivated) {
                vcpu.set_vtimer_mask(true)?;
                continue;
            }
            *out.exits.entry(0).or_default() += 1;
            if matches!(exit, Exit::Canceled) {
                // `hv_vcpus_exit` from the run's time limit (w2m-vm's GUEST_SECONDS; Sprint 197)
                break Stop::Exit("the run's time limit".into());
            }
            break Stop::Exit(format!("{exit:?} at pc 0x{pc:x}"));
        };
        *out.exits.entry(number).or_default() += 1;
        if number == HVC_CODE_WRITE as u16
            && let Some(h) = host.as_deref_mut()
        {
            // a write faulted: behind translated code? (5297-5299)
            let address = vcpu.reg(Reg::X(17))?;
            match h.code_written(guest, address) {
                Ok(written) => vcpu.set_reg(Reg::X(17), u64::from(written))?,
                Err(why) => break Stop::Untranslated { target: address, why },
            }
            continue; // on at the hvc's return
        }
        if number == 8 {
            let op = read_u64(mem, MONITOR_SIZE + TRANSPORT_OFF);
            if host.is_none() {
                let why = "a run without the host's compiler serves no host request".into();
                break Stop::HostRequest { op, pc, why };
            }
            // the requests the Rust host serves (`_host_request`, 5244-5256): on at the hvc's return, or the end
            let threads = guest.threads.len();
            let (sub, a0, current) = (
                read_u64(mem, MONITOR_SIZE + TRANSPORT_OFF + 8),
                read_u64(mem, MONITOR_SIZE + TRANSPORT_OFF + 16),
                crate::threads::current_tid(guest),
            );
            let served = crate::host_request::serve(guest, vcpu);
            if let Some(l) = &mut guest.exit_ledger {
                let (t, mem) = (MONITOR_SIZE + TRANSPORT_OFF, guest.memory.as_slice());
                l.answered(read_u64(mem, t + 48), read_u64(mem, t + 56));
            }
            if op == HOST_THREAD && matches!(sub, THREAD_EXIT_STUB | THREAD_END | THREAD_EXIT_CODE) && served.is_ok() {
                let answer = read_u64(guest.memory.as_slice(), MONITOR_SIZE + TRANSPORT_OFF + 56);
                out.thread_requests.push(ThreadRequest {
                    sub,
                    a0,
                    answer,
                    current,
                });
            }
            if guest.threads.len() > threads {
                let t = guest.threads.values().next_back().unwrap();
                let (at, mem) = (crate::threads::entry(t.index), guest.memory.as_slice());
                let vslot = read_u64(mem, at + crate::threads::E_VSLOT);
                out.created.push(Created {
                    tid: t.tid,
                    entry: mem[at as usize..(at + RUN_ENTRY) as usize].to_vec(),
                    vectors: mem[vslot as usize..vslot as usize + 272].to_vec(),
                    count: read_u64(mem, md),
                    limit: read_u64(mem, md + 8),
                });
            }
            match served {
                Ok(crate::host_request::Served::Answered) => continue,
                Ok(crate::host_request::Served::Exit(code)) => break Stop::ExitProcess { code },
                // a request refused (the Python's Unimplemented) stops as unserved; a failure in serving one stops as
                // such
                Err(crate::host_request::HostError::Unserved(why)) => break Stop::HostRequest { op, pc, why },
                Err(e) => {
                    break Stop::HostFailure {
                        op,
                        pc,
                        why: e.to_string(),
                    };
                }
            }
        }
        if u32::from(number) == HVC_SCHEDULE && host.is_some() {
            // a service handed the vCPU over, and threads.c `w2m_switch` left it to the host (5300-5303): `_schedule`,
            // then the next thread's registers at the dispatcher (5077-5094), its vectors swapped as the monitor's
            // `vswap` swaps them
            let at = (MONITOR_SIZE + STATE_OFF) as usize + STATE_SCHEDULE as usize;
            let bits = u32::from_le_bytes(mem[at..at + 4].try_into().unwrap());
            mem[at..at + 4].fill(0);
            let mut x = [0u64; 31];
            for (r, v) in x.iter_mut().enumerate() {
                *v = vcpu.reg(Reg::X(r as u8))?;
            }
            let from = crate::threads::current_tid(guest);
            let mut switched = crate::threads::schedule(guest, &mut x, bits);
            // no thread ready in a process of a machine: the turn goes to the others until one is (Sprint 197)
            while switched == Ok(Switch::Idle) {
                switched = hand_over(guest, crate::turns::Message::Idle)
                    .and_then(|()| crate::threads::resume_idle(guest, &mut x, from, bits));
            }
            match switched {
                Err(why) => break Stop::Handover { pc, why },
                Ok(Switch::Idle) => unreachable!(),
                Ok(Switch::Ended(code)) => {
                    out.handovers.push(Handover {
                        schedule: bits,
                        from,
                        to: 0,
                    });
                    break Stop::ExitProcess { code };
                }
                Ok(Switch::Go { vectors }) => {
                    out.handovers.push(Handover {
                        schedule: bits,
                        from,
                        to: crate::threads::current_tid(guest),
                    });
                    if let Some((save, load)) = vectors {
                        swap_vectors(vcpu, guest.memory.as_mut_slice(), save, load)?;
                    }
                    for (r, v) in x.iter().enumerate() {
                        vcpu.set_reg(Reg::X(r as u8), *v)?;
                    }
                    vcpu.set_reg(Reg::Pc, guest.code.dispatch_va)?;
                    vcpu.set_reg(Reg::Cpsr, CPSR_EL1H)?;
                    continue;
                }
            }
        }
        if number == crate::cases::HVC_SLOW_SSE || number == crate::cases::HVC_SLOW_STEP {
            // `serve_exit` (5232, 5270-5271; Sprint 181): the instruction at the hvc's site, one step of the model,
            // on at the dispatcher or the runtime's exception call as the step leaves the PC
            let rip = match x86_address_at(guest.memory.as_slice(), pc - 4) {
                Ok(rip) => rip,
                Err(why) => break Stop::Exit(why),
            };
            let code = code_at(guest, rip);
            match crate::model_step::model_step(guest, vcpu, &code, rip)? {
                crate::model_step::ModelStep::Resumed | crate::model_step::ModelStep::Raised => {
                    vcpu.set_reg(Reg::Cpsr, CPSR_EL1H)?;
                    continue;
                }
                crate::model_step::ModelStep::Refused => {
                    break Stop::Model {
                        rip,
                        why: "an operation the model does not run".into(),
                    };
                }
                crate::model_step::ModelStep::Undefined(f) => {
                    break Stop::Model {
                        rip,
                        why: format!("flag {f} read while undefined"),
                    };
                }
            }
        }
        if number == 4 && guest.link.is_some() {
            // a slice's end in a process of a machine (`serve_exit`, 5280-5293; Sprint 197): at the run's limit a
            // stop; else the next slice's budget and expired waits woken, then the turn to another process that is
            // ready, or the next thread of this one, the stopped block's count given back as the thread goes on at its
            // entry
            let count = read_u64(guest.memory.as_slice(), md);
            if count >= max_steps {
                break Stop::Exit(format!("max_steps: {count} of {max_steps} at pc 0x{pc:x}"));
            }
            crate::threads::set_budget(guest);
            crate::threads::wake_expired(guest);
            let others = guest
                .link
                .as_ref()
                .is_some_and(|l| l.others_ready(crate::threads::counter_now(guest)));
            if !others && guest.threads.len() < 2 {
                continue;
            }
            let at = match x86_address_at(guest.memory.as_slice(), pc - 4) {
                Ok(at) => at,
                Err(why) => break Stop::Exit(why),
            };
            let mut x = [0u64; 31];
            for (r, v) in x.iter_mut().enumerate() {
                *v = vcpu.reg(Reg::X(r as u8))?;
            }
            x[17] = at;
            if let Err(why) = give_back(guest, pc) {
                break Stop::Exit(why);
            }
            if others && let Err(why) = hand_over(guest, crate::turns::Message::Slice) {
                break Stop::Handover { pc, why };
            }
            let from = crate::threads::current_tid(guest);
            let mut switched = crate::threads::schedule(guest, &mut x, 0);
            while switched == Ok(Switch::Idle) {
                switched = hand_over(guest, crate::turns::Message::Idle)
                    .and_then(|()| crate::threads::resume_idle(guest, &mut x, from, 0));
            }
            match switched {
                Err(why) => break Stop::Handover { pc, why },
                Ok(Switch::Idle) => unreachable!(),
                Ok(Switch::Ended(code)) => break Stop::ExitProcess { code },
                Ok(Switch::Go { vectors }) => {
                    if let Some((save, load)) = vectors {
                        swap_vectors(vcpu, guest.memory.as_mut_slice(), save, load)?;
                    }
                    for (r, v) in x.iter().enumerate() {
                        vcpu.set_reg(Reg::X(r as u8), *v)?;
                    }
                    vcpu.set_reg(Reg::Pc, guest.code.dispatch_va)?;
                    vcpu.set_reg(Reg::Cpsr, CPSR_EL1H)?;
                    continue;
                }
            }
        }
        if number != 1 {
            break Stop::Hvc { number, pc };
        }
        let target = vcpu.reg(Reg::X(17))?;
        if host.is_none() && out.bridged.contains_key(&target) {
            // its bridge was inserted at the last miss here: the table should have hit (with the host compiling, a
            // two-way slot may have let it go, and it is inserted again)
            break Stop::Repeated { target };
        }
        let va = match guest.bridges.get(&target) {
            Some(&va) => {
                *out.bridged.entry(target).or_default() += 1;
                va
            }
            None => match (out.blocks.get(&target), host.as_deref_mut()) {
                (Some(&va), _) => va,
                (None, None) => break Stop::Declined { target },
                (None, Some(h)) => match h.blocks.get(&target) {
                    Some(&va) => va,
                    None => match h.compile(guest, target) {
                        Ok(va) => {
                            let (start, end) = *h.translations.last().unwrap();
                            out.translations.push(Translation { start, end, code: va });
                            va
                        }
                        Err(why) => break Stop::Untranslated { target, why },
                    },
                },
            },
        };
        let mem = guest.memory.as_mut_slice();
        dispatch_insert(mem, target, va);
        vcpu.set_reg(Reg::Pc, guest.code.miss_va + 4)?; // ic ialluis; tlbi; dsb; isb; re-dispatch
    };
    // the stop's own row, then the file whole (Sprint 206)
    if let Some(l) = &mut guest.exit_ledger {
        let mem = guest.memory.as_slice();
        let warp = read_u64(mem, RUN_TABLE + crate::process::CLOCK_WARP);
        let _ = l.resume(read_u64(mem, md), warp, process_below_trace(mem));
        let _ = l.flush();
        // the blocks both compilers made (Sprint 207)
        let mut blocks: Vec<(u64, u64)> = out.translations.iter().map(|t| (t.start, t.end)).collect();
        if let Some(h) = host.as_deref() {
            blocks.extend(h.translations.iter().copied());
        }
        let _ = l.blocks(&blocks);
    }
    let mem = guest.memory.as_slice();
    for (i, name) in VMC_COUNT_NAMES.iter().enumerate() {
        out.counts
            .insert(name, read_u64(mem, VM_COMPILER + VMC_COUNTS + 8 * i as u64));
    }
    out.executed = read_u64(mem, md);
    out.stop = stop;
    if guest.time_runtime_word {
        out.runtime_word = read_runtime_word(guest.memory.as_slice()); // Sprint 164: read once, at the stop
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::recon_lookup;

    /// A table of (host, x86) entries at offset 0, as RECON_TABLE holds them.
    fn table(entries: &[(u64, u64)]) -> Vec<u8> {
        entries
            .iter()
            .flat_map(|(h, x)| h.to_le_bytes().into_iter().chain(x.to_le_bytes()))
            .collect()
    }

    /// Two marks at one host address (a block's first instruction emits no words): bisect_right takes the last.
    #[test]
    fn tied_hosts_give_the_last_entry() {
        let t = table(&[(0x100, 0x1000), (0x110, 0x1004), (0x110, 0x1007), (0x120, 0x100a)]);
        assert_eq!(recon_lookup(&t, 0, 4, 8, 0x110), Ok(0x1007));
        assert_eq!(recon_lookup(&t, 0, 4, 8, 0x11c), Ok(0x1007));
        assert_eq!(recon_lookup(&t, 0, 4, 8, 0x120), Ok(0x100a));
    }

    #[test]
    fn a_pc_below_the_first_entry_gives_zero() {
        let t = table(&[(0x100, 0x1000), (0x110, 0x1004)]);
        assert_eq!(recon_lookup(&t, 0, 2, 8, 0xfc), Ok(0));
    }

    /// Entries past RECON_COUNT are not yet published (vm_backend 4855-4859) and are not read.
    #[test]
    fn entries_past_the_count_are_not_read() {
        let t = table(&[(0x100, 0x1000), (0x110, 0x1004), (0x120, 0x100a)]);
        assert_eq!(recon_lookup(&t, 0, 2, 8, 0x130), Ok(0x1004));
        assert_eq!(recon_lookup(&t, 0, 0, 8, 0x130), Ok(0));
    }

    /// The count is the guest's to write (Sprint 188; reviewer-2's Low 4 of its review of 182-184): one past the
    /// table's capacity, or past the memory, is an error naming it, not a panic.
    #[test]
    fn a_count_past_the_table_is_an_error() {
        let t = table(&[(0x100, 0x1000), (0x110, 0x1004)]);
        let past = "the reconstruction table's count 3 is past its 2 entries or the memory";
        assert_eq!(recon_lookup(&t, 0, 3, 2, 0x110), Err(past.to_string()));
        let beyond = "the reconstruction table's count 3 is past its 8 entries or the memory";
        assert_eq!(recon_lookup(&t, 0, 3, 8, 0x110), Err(beyond.to_string()));
        assert!(recon_lookup(&t, 0, u64::MAX, u64::MAX, 0x110).is_err());
    }
}
