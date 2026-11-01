// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! One vCPU running one thread of a `Guest` (Sprint 142): `VM.steps` (`src/w2m/vm_backend.py` 4916-5127) for that
//! case, driven through `hypervisor` with no monitor process between. It serves the dispatch misses a bridge answers and
//! stops at the first miss that needs the host's compiler, which the translator's sprints bring. Each function cites
//! the lines it carries.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use guest_runtime::layout::{
    CODE_CACHE, CODE_CACHE_SIZE, DISPATCH_TABLE, MONITOR_DATA, MONITOR_SIZE, MONITOR_VA, PROCESS_VA, RUN_ENTRY,
    RUN_TABLE, RUNTIME_IMAGE, RUNTIME_IMAGE_SIZE, SLICE_UNTIL, TEB_OFF, TRANSPORT_OFF, VM_COMPILER, VMC_LOG,
    VMC_LOG_ENTRY,
};
use hypervisor::{Exit, HvError, Reg, Vcpu, Vm};
use pe_loader::Loaded;

use crate::CPSR_EL1H;
use crate::compile::HostCompiler;
use crate::concurrent_memory::ConcurrentGuestMemory;
use crate::guest::{
    Guest, HVC_SCHEDULE, RT_IN_VM, STATE_SCHEDULE, VMC_COUNT_NAMES, VMC_COUNTS, VMC_ENABLED, VcpuPages,
};
use crate::host_request::{HOST_THREAD, THREAD_END, THREAD_EXIT_CODE, THREAD_EXIT_STUB};
use crate::monitor::set_system_registers;
use crate::monitor_code::{
    EMPTY_TARGET, HVC_CODE_WRITE, HVC_FAULT_GIVE_BACK, HVC_TLB_FLUSHED, LOCK_ENTRIES, LOCK_ENTRY, LOCK_EXITS,
    LOCK_TAKINGS, LOCK_TAKINGS_COUNT, LOCK_TIMING, REG_BUDGET, VMC_LOG_EXECUTED, dispatch_slot,
};
use crate::threads::{STILL_ACTIVE, Switch, Thread, add_thread, slice_until};

pub use guest_runtime::layout::FCW_OFF;
pub use guest_runtime::layout::LOCK_OWNER;
/// MONITOR_DATA + these (vm_backend 96, 107, 113, 119).
pub use guest_runtime::layout::MXCSR_OFF;
pub use guest_runtime::layout::RUN_END;
/// x24 holds the flags not yet defined (vm_backend 359); at entry every one.
pub const REG_UNDEFINED_FLAGS: u8 = translator::lower::REG_UNDEFINED_FLAGS as u8;

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
/// left) written to the vCPU's monitor data page at `md` (Sprint 313).
pub fn exit_count(
    vcpu: &Vcpu<'_>,
    mem: ConcurrentGuestMemory<'_>,
    md: u64,
    tails: &Tails,
    give_back: GiveBack,
) -> Result<(), HvError> {
    let limit = mem.read_u64(md + 8);
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
    mem.write_u64(md, limit.wrapping_sub(left));
    Ok(())
}
/// VM_COMPILER + this: the log's entry count (vm_backend 197).
pub use crate::monitor_code::VMC_LOG_COUNT;

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
    /// the run's time limit, `hv_vcpus_exit` from w2m-vm's GUEST_SECONDS or GAME_SECONDS (Sprint 197)
    TimeLimit,
    /// the VM's step budget spent, `count` of `max` steps, at host `pc` (Sprint 218; reviewer-3's Low 1 of its review of
    /// Sprint 218: it had been an `Exit` read back from its text)
    StepLimit { count: u64, max: u64, pc: u64 },
    /// the batch run's end by its reference's rule (`batch_stop`, Sprint 219), and which part of it held
    ReferenceEnd(String),
    /// a call to an import no service answers (`hvc #9`), named "DLL!Function" by its thunk, at the hvc's `pc`
    /// (Sprint 221)
    Import { name: String, pc: u64 },
    /// any other exit
    Exit(String),
}

impl Stop {
    /// The kind a record's end is read from (`record::RunEnd::of`): a request not served, a failure in serving one
    /// (Other), ExitProcess, the time limit, the step budget.
    pub fn kind(&self) -> crate::record::StopKind {
        use crate::record::StopKind;
        match self {
            Stop::ExitProcess { code } => StopKind::ExitProcess(*code),
            Stop::TimeLimit => StopKind::TimeLimit,
            Stop::StepLimit { .. } => StopKind::StepLimit,
            Stop::ReferenceEnd(_) => StopKind::ReferenceEnd,
            Stop::Import { .. } => StopKind::Unserved,
            Stop::HostRequest { .. } => StopKind::Unserved,
            _ => StopKind::Other,
        }
    }
}

/// `x86_address_at` (vm_backend 4861-4863): the guest address of the host code at `pc`, from the reconstruction table
/// (the entry with the largest host address at or below `pc`, as `bisect_right` over the sorted marks finds it); 0
/// when none is (Sprint 181). The count is the guest's to write, so a count past the table's `RECON_ENTRIES` or the
/// memory is an error, not a slice past the end (Sprint 188; reviewer-2's Low 4 of its review of 182-184).
pub fn x86_address_at(mem: ConcurrentGuestMemory<'_>, pc: u64) -> Result<u64, String> {
    use guest_runtime::layout::{RECON_COUNT, RECON_ENTRIES, RECON_TABLE, RUN_TABLE};
    // the count Acquire, then the entries below it (Sprint 282; the design's table): another vCPU appends an entry and
    // then the count, and on the host's Arm ordering a plain count read could let an entry read pass it
    let count = mem
        .atomic_u64(RUN_TABLE + RECON_COUNT)
        .load(std::sync::atomic::Ordering::Acquire);
    recon_lookup(mem, RECON_TABLE, count, RECON_ENTRIES, pc)
}

/// `bisect_right(pc_map, (pc, M64)) - 1` over the first `n` (host, x86) entries at `table`, kept sorted as the code
/// cache grows (vm_backend 4626-4628): the x86 address of the last entry whose host address is at or below `pc`, 0
/// below the first (Sprint 184).
/// Read through the view (Sprint 276): on several vCPUs the host looks an address up while others run.
fn recon_lookup(mem: ConcurrentGuestMemory<'_>, table: u64, n: u64, capacity: u64, pc: u64) -> Result<u64, String> {
    let end = table
        .checked_add(n.saturating_mul(16))
        .filter(|e| *e <= mem.len() as u64);
    let (Some(_), true) = (end, n <= capacity) else {
        return Err(format!(
            "the reconstruction table's count {n} is past its {capacity} entries or the memory"
        ));
    };
    let entry = |i: u64| (mem.read_u64(table + 16 * i), mem.read_u64(table + 16 * i + 8));
    // the first entry whose host address is past `pc`, by bisection
    let (mut low, mut high) = (0, n);
    while low < high {
        let mid = low + (high - low) / 2;
        if entry(mid) <= (pc, u64::MAX) {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    Ok(match low {
        0 => 0,
        i => entry(i - 1).1,
    })
}

/// The instruction bytes at guest `rip` for the model's lifter: up to 15 bytes of mapped memory, then 16 zeros, as
/// `lift(code + bytes(16), ...)` pads them (Sprint 181).
fn code_at(guest: &Guest, rip: u64) -> Vec<u8> {
    let mem = guest.memory_view();
    let mut code: Vec<u8> = (0..15u64)
        .map_while(|i| {
            guest.layout.offset(rip + i).map(|o| {
                let mut b = [0u8];
                mem.read(o, &mut b);
                b[0]
            })
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
pub fn read_runtime_word(mem: ConcurrentGuestMemory<'_>) -> RuntimeWordTimes {
    let counted = mem.read_u64(LOCK_TIMING + LOCK_TAKINGS_COUNT);
    let takings = (0..counted.min(LOCK_TAKINGS))
        .map(|i| {
            let e = LOCK_TIMING + LOCK_ENTRIES + LOCK_ENTRY * i;
            let w = |k: u64| mem.read_u64(e + 8 * k);
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

/// The kind the whole-block judge's host time is counted under, apart from the exit it follows (Sprint 257): the
/// record's `run_seconds` "host judge".
pub const JUDGE_KIND: &str = "judge";

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
    /// the virtual counter's ticks the miss stub spent calling the in-VM compiler, at the stop (Sprint 247), with the
    /// longest compile and the most in a frame (Sprint 248); 0 unless `Start::time_compile` asked for them
    pub compile_ticks: crate::monitor_code::CompileTicks,
    /// each served exit's host time, from the vCPU's exit to its next run, in ns
    pub serve_ns: Vec<u64>,
    /// the same times by the exit's kind (`hvc <k>`, `vtimer`, or `ec 0x<class>`), for step 3's measure (Sprint 163),
    /// the whole-block judge's part as `JUDGE_KIND` (Sprint 257)
    pub serve_by_kind: BTreeMap<String, Vec<u64>>,
    /// each turn's start, the slice's end and whether another process is due read under the runtime word and the lock
    /// before the vCPU runs, in ns, one a run of the vCPU; not counted in `serve_ns` (Sprint 324: the run-loop design's
    /// measure of its first form against the second)
    pub turn_start_ns: Vec<u64>,
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
    /// the start of each block the in-VM compiler compiled, in its log's order, the host's compiles not among them
    /// (Sprint 249: the compiles by image)
    pub vm_compile_starts: Vec<u64>,
    /// the services that change state two vCPUs share (`hvc #1`, `#4`, `#12`, `#13`) served with the runtime word held
    /// neither by the host nor by the vCPU itself, the word read after the take (Sprint 286 for the miss, 294 for the
    /// rest): 0 when each runs under the word, beside the guest's own services, which hold it
    pub served_outside_word: u64,
    /// the times the vCPU was resumed through `FLUSH_STUB` after a request split a block, each ending at `hvc #15`
    /// (Sprint 304): equal to `guest.served.splits` at the stop, every split followed by its flush
    pub flushes: u64,
    /// the instructions retired at each in-VM compile and install, in its log's order (Sprint 266), from the words
    /// `Timing::instructions_at_compile` writes; all 0 without it
    pub instructions_at: InstructionsAt,
    /// each vCPU's count of runs, `hv_vcpu_run` calls, in the vCPU's order (Sprint 327): a vCPU past the first runs only
    /// once it has taken a thread, so its count above 0 says it ran guest code
    pub runs_by_vcpu: Vec<u64>,
}

impl Outcome {
    /// An outcome with nothing in it yet, the stop to be set at the run's end.
    fn empty() -> Outcome {
        Outcome {
            stop: Stop::Exit(String::new()),
            bridged: BTreeMap::new(),
            blocks: BTreeMap::new(),
            counts: BTreeMap::new(),
            compile_ticks: Default::default(),
            serve_ns: Vec::new(),
            serve_by_kind: BTreeMap::new(),
            turn_start_ns: Vec::new(),
            runtime_word: RuntimeWordTimes::default(),
            run_ns: Vec::new(),
            exits: BTreeMap::new(),
            executed: 0,
            created: Vec::new(),
            thread_requests: Vec::new(),
            handovers: Vec::new(),
            translations: Vec::new(),
            vm_compile_starts: Vec::new(),
            instructions_at: InstructionsAt::default(),
            served_outside_word: 0,
            flushes: 0,
            runs_by_vcpu: Vec::new(),
        }
    }

    /// One vCPU's loop's part, its times, exits and counts, added to the run's (Sprint 327: the merged outcome); the
    /// records the lock holds and the counts read from guest memory at the stop are the run's own, not a vCPU's.
    fn add_vcpu(&mut self, vcpu: Outcome) {
        self.runs_by_vcpu.push(vcpu.run_ns.len() as u64);
        self.serve_ns.extend(vcpu.serve_ns);
        for (kind, ns) in vcpu.serve_by_kind {
            self.serve_by_kind.entry(kind).or_default().extend(ns);
        }
        self.turn_start_ns.extend(vcpu.turn_start_ns);
        self.run_ns.extend(vcpu.run_ns);
        for (number, n) in vcpu.exits {
            *self.exits.entry(number).or_default() += n;
        }
        self.served_outside_word += vcpu.served_outside_word;
        self.flushes += vcpu.flushes;
    }
}

/// The instructions retired at the calls that logged a block inside the VM (Sprint 266), compiles and installs apart.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstructionsAt {
    pub compiles: Vec<u64>,
    pub installs: Vec<u64>,
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
fn swap_vectors(vcpu: &Vcpu<'_>, mem: ConcurrentGuestMemory<'_>, save: u64, load: u64) -> Result<(), HvError> {
    save_vectors(vcpu, mem, save)?;
    load_vectors(vcpu, mem, load)
}

/// `vswap`'s first half: the vCPU's vector registers to the slot at `save`, for a thread put down with no thread loaded
/// after it (Sprint 327: a vCPU that parks).
fn save_vectors(vcpu: &Vcpu<'_>, mem: ConcurrentGuestMemory<'_>, save: u64) -> Result<(), HvError> {
    for n in 0..16u8 {
        mem.write(save + 16 * u64::from(n), &vcpu.simd(n)?.to_le_bytes());
    }
    mem.write_u64(save + 256, vcpu.reg(Reg::Fpcr)?);
    mem.write_u64(save + 264, vcpu.reg(Reg::Fpsr)?);
    Ok(())
}

/// `vswap`'s second half: the slot at `load` to the vCPU's vector registers, for a thread taken by a vCPU that had none
/// (Sprint 327).
fn load_vectors(vcpu: &Vcpu<'_>, mem: ConcurrentGuestMemory<'_>, load: u64) -> Result<(), HvError> {
    for n in 0..16u8 {
        let mut register = [0u8; 16];
        mem.read(load + 16 * u64::from(n), &mut register);
        vcpu.set_simd(n, u128::from_le_bytes(register))?;
    }
    vcpu.set_reg(Reg::Fpcr, mem.read_u64(load + 256))?;
    vcpu.set_reg(Reg::Fpsr, mem.read_u64(load + 264))
}

/// `_dispatch_insert` (vm_backend 4827-4849): (target, va) in its pair, in a slot that is empty or holds it, else
/// first, the first slot's entry moved to the second. Each pair is stored whole after a release fence (Sprint 276):
/// the dispatcher reads a pair by one `LDP` with no lock, so on several vCPUs two stores would let it match a new
/// target with an old block, or the old target with a new one when a filled slot is rewritten. Only the host writes
/// the table, under its compile lock, so its own reads are plain.
pub fn dispatch_insert(mem: ConcurrentGuestMemory<'_>, target: u64, va: u64) {
    let at = DISPATCH_TABLE + 16 * dispatch_slot(target);
    // way 0 read whole, as it may be moved to way 1 (Sprint 286)
    let ((t0, v0), t1) = (mem.load_pair(at), mem.read_u64(at + 16));
    let publish = |slot: u64, t: u64, v: u64| mem.store_pair_release(slot, t, v);
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

/// The code an `hvc 1` hands back for `target` without a compile: with the host's compiler present, its map alone,
/// which holds both compilers' blocks (`vm_compiled` adds the VM's) and from which a guest write drops a page's blocks
/// (`HostCompiler::code_written`); without it, the in-VM compiler's, which no write handler drops from. The frozen host
/// read its one map so (`vm_backend.py` 5242, 4568). Sprint 250: reading the VM's map first handed back a block's old
/// code once a batch run, at 0x1222ee5fff in the run-time code's arena, after its page was written.
fn known_block(vm_blocks: &BTreeMap<u64, u64>, host_blocks: Option<&BTreeMap<u64, u64>>, target: u64) -> Option<u64> {
    host_blocks.unwrap_or(vm_blocks).get(&target).copied()
}

/// `_vm_compiled` (vm_backend 2559-2580): each block block_compiler.c compiled since the last exit, from its log entry
/// (start, end, code, then u32s first, nmarks, count, cut, live_out, live_used, npages, and the pages at 56), noted as
/// the host notes its own compiles: the marks' tails (`_note_tails`, so a model step's give-back counts the block's
/// unrun instructions; Sprint 207: before, a model step in a block the VM compiled gave nothing back, and the count ran
/// 7 instructions ahead of the frozen host's at the batch run's first `hvc 11`), the block, the pages it covers and the
/// pages it lists, and the translation; then the log emptied. Through the view (Sprint 283). The caller holds the
/// runtime word, which every in-VM compile and install appends under (Sprint 275, census D6: on several vCPUs
/// another's append between the read and the reset would be lost), and the run's lock, `records` and `host` being
/// what the lock guards (Sprint 311).
fn vm_compiled(mem: ConcurrentGuestMemory<'_>, records: &mut Records, mut host: Option<&mut HostCompiler>) {
    use guest_runtime::layout::{PAGE, RECON_TABLE};
    let n = mem.read_u64(VM_COMPILER + VMC_LOG_COUNT);
    for i in 0..n {
        let at = VM_COMPILER + VMC_LOG + VMC_LOG_ENTRY * i;
        let (start, end, code) = (mem.read_u64(at), mem.read_u64(at + 8), mem.read_u64(at + 16));
        records.blocks.insert(start, code);
        let u32_at = |off: u64| u64::from(mem.read_u32(at + off));
        let (first, count, npages, installed) = (u32_at(24), u32_at(32), u32_at(48), u32_at(52) == 1);
        let executed = mem.read_u64(at + VMC_LOG_EXECUTED);
        if installed {
            records.instructions_at.installs.push(executed);
        } else {
            records.instructions_at.compiles.push(executed);
        }
        let hosts: Vec<u64> = (1..=count)
            .map(|k| mem.read_u64(RECON_TABLE + 16 * (first + k)))
            .collect();
        if installed {
            // an install by the installer inside the VM (Sprint 263; card 5's V), not a compile; `pad` cleared, the C
            // leaving it unwritten in a slot it reuses
            mem.write_u32(at + 52, 0);
            if let Some(h) = host.as_deref_mut() {
                h.installed_in_vm(mem, start, end, code, &hosts);
            }
            continue;
        }
        records.translations.push(Translation { start, end, code });
        records.vm_compile_starts.push(start);
        let Some(h) = host.as_deref_mut() else {
            continue;
        };
        h.tails.note(&hosts);
        h.blocks.insert(start, code);
        let mut page = start & !(PAGE - 1);
        while page < end {
            h.code_pages.entry(page).or_default().insert(start);
            page += PAGE;
        }
        for k in 0..npages {
            h.code_pages
                .entry(mem.read_u64(at + 56 + 8 * k))
                .or_default()
                .insert(start);
        }
        h.translations.push((start, end));
    }
    mem.write_u64(VM_COMPILER + VMC_LOG_COUNT, 0);
}

/// Runs `guest`'s program from its entry on `vcpu`, one thread, until the first miss the host must compile or an exit
/// this run does not serve (`VM.steps` 4964-5095 and `serve_exit` 5197-5243 for one vCPU and one thread).
pub fn run_to_first_decline(
    guest: &mut Guest,
    loaded: &Loaded,
    vm: &Vm,
    limit: Option<Duration>,
    max_steps: u64,
) -> Result<Outcome, HvError> {
    run(guest, loaded, vm, limit, max_steps, None)
}

/// As `run_to_first_decline`, with the host compiling the blocks the VM's C declines (Sprint 155: `serve_exit`'s
/// `_compile`, 5243, and its code-write exit, 5297-5299), until an exit this run does not serve or a block it cannot
/// compile.
/// `_give_back` with `_stop_block_length` (vm_backend 4862-4878): a thread leaving at a block's stop goes on at the
/// block's entry, whose SUBS counts the block again, so the block's instructions come off the count. `after_stop` is
/// the word after the stop's `hvc #4`, the B to the block's body; the SUBS x23, x23, #n is two words before the body.
fn give_back(guest: &mut Guest, after_stop: u64) -> Result<(), String> {
    let mem = guest.memory_view();
    let off = after_stop - MONITOR_VA;
    let imm26 = i64::from(mem.read_u32(off) & 0x3FF_FFFF);
    let body = (off as i64 + 4 * if imm26 >> 25 != 0 { imm26 - (1 << 26) } else { imm26 }) as u64;
    let subs = mem.read_u32(body - 8);
    let reg = REG_BUDGET;
    if subs & !(0xFFF << 10) != 0xF100_0000 | reg << 5 | reg {
        return Err(format!(
            "the word before the block's body at 0x{body:x} is 0x{subs:08x}, not its SUBS"
        ));
    }
    let count = mem.read_u64(guest.served.vcpu.monitor);
    mem.write_u64(guest.served.vcpu.monitor, count - u64::from(subs >> 10 & 0xFFF));
    Ok(())
}

/// Whether a run may have `n` vCPUs (Sprint 308; the run-loop design's card 4a): one always; more refused, each reason
/// by name: FEAT_LSE2 not reported (the dispatch pair's single-copy atomicity rests on it, the guest-memory design's
/// choice 3; `lse2` is `hypervisor::feature("FEAT_LSE2")`); a diagnostic (each reads every exit in order, which two
/// threads do not have); a child process (one vCPU until its KERNEL 33 wait lets the word go, the design's host-request
/// row); the instruction clock (Sprint 315); more than the layout's vCPUs. The second vCPU itself is built (Sprint 327).
pub fn vcpus_allowed(n: u32, lse2: Option<i32>, diagnostics: &[&str], child: bool, clock: bool) -> Result<(), String> {
    if n <= 1 {
        return Ok(());
    }
    if lse2 != Some(1) {
        return Err(format!("{n} vCPUs refused: FEAT_LSE2 not reported ({lse2:?})"));
    }
    if let Some(d) = diagnostics.first() {
        return Err(format!("{n} vCPUs refused: {d} reads every exit in order"));
    }
    if child {
        return Err(format!("{n} vCPUs refused: a child process runs on one"));
    }
    // the C runtime's clock reads the running vCPU's own count (`process.c` 45; the per-vCPU pages design, "Not decided
    // here"; Sprint 315)
    if clock {
        return Err(format!(
            "{n} vCPUs refused: the instruction clock reads the running vCPU's own count"
        ));
    }
    // the monitor region has pages for this many vCPUs (`MAX_VCPUS`, its eight vCPU areas; Sprint 327)
    if u64::from(n) > guest_runtime::layout::MAX_VCPUS {
        return Err(format!(
            "{n} vCPUs refused: the layout has pages for {}",
            guest_runtime::layout::MAX_VCPUS
        ));
    }
    Ok(())
}

/// What the flush stub's nested runs saw inside a split, which the run loop folds into its outcome (Sprint 305).
#[derive(Default)]
pub struct FlushCounts {
    pub flushes: std::cell::Cell<u64>,
    pub cancelled: std::cell::Cell<bool>,
}

/// A block split's flush (Sprint 305; the design's precondition 4): the requesting vCPU, exited for the request, run by
/// the host from inside the split at `FLUSH_STUB` until its `hvc #15`, which invalidates every vCPU's TLB before the
/// table is written, then given back its PC, PSTATE and general registers as they were. A second, smaller run loop
/// (reviewer-3, by message at 304): a vtimer exit runs the stub again with the timer masked, as the run loop masks it; a
/// cancel, which `hv_vcpus_exit` leaves pending for the next run when the vCPU is out ("it will return immediately
/// without entering the guest", `hv_vcpu.h` 377-378), runs it again too, noted in `counts` and reported after the split,
/// so no block is left invalid; any other exit fails the split. x23, the budget, is not touched.
pub fn flush_through_stub(vcpu: &Vcpu, counts: &FlushCounts) -> Result<(), String> {
    let e = |e: HvError| format!("the flush stub: {e:?}");
    let pc = vcpu.reg(Reg::Pc).map_err(e)?;
    let cpsr = vcpu.reg(Reg::Cpsr).map_err(e)?;
    let mut x = [0u64; 31];
    for (i, v) in x.iter_mut().enumerate() {
        *v = vcpu.reg(Reg::X(i as u8)).map_err(e)?;
    }
    vcpu.set_reg(Reg::Pc, MONITOR_VA + crate::FLUSH_STUB).map_err(e)?;
    loop {
        match vcpu.run().map_err(e)? {
            exit if exit.hvc_immediate() == Some(HVC_TLB_FLUSHED as u16) => break,
            Exit::VtimerActivated => vcpu.set_vtimer_mask(true).map_err(e)?,
            Exit::Canceled => counts.cancelled.set(true),
            other => return Err(format!("the flush stub left by {other:?}, not hvc #15")),
        }
    }
    vcpu.set_reg(Reg::Pc, pc).map_err(e)?;
    vcpu.set_reg(Reg::Cpsr, cpsr).map_err(e)?;
    for (i, v) in x.iter().enumerate() {
        vcpu.set_reg(Reg::X(i as u8), *v).map_err(e)?;
    }
    counts.flushes.set(counts.flushes.get() + 1);
    Ok(())
}

/// The vector slot of thread `tid`, where a thread put down keeps its vector registers (Sprint 327: a vCPU that parks
/// saves them there, as `vswap` does at a switch).
fn vector_slot(guest: &Guest, tid: u64) -> Option<u64> {
    let t = guest.served.threads.get(&tid)?;
    Some(
        guest
            .memory_view()
            .read_u64(crate::threads::entry(t.index) + crate::threads::E_VSLOT),
    )
}

/// The runtime word taken for the host, for a service that changes state two vCPUs share (`hvc #1`, `#4`, `#12`, `#13`;
/// `docs/design-run-loop-on-several-vcpus-2026-10-31.md`, "The exits"), by the vCPU whose monitor data page is at
/// `monitor`; true when the host took it, and so must release it; a vCPU that holds it already (a miss inside the
/// runtime) keeps it. A service found outside it after the take, the word held neither by the host nor by this vCPU, is
/// counted in `outside`.
fn take_word(mem: ConcurrentGuestMemory<'_>, monitor: u64, outside: &mut u64) -> bool {
    let own = crate::monitor_code::vcpu_owner(mem, monitor);
    let took = crate::monitor_code::host_take_runtime_word(mem, own);
    let holder = mem
        .atomic_u32(RUN_TABLE + crate::monitor_code::RUNTIME_LOCK)
        .load(std::sync::atomic::Ordering::Relaxed);
    *outside += u64::from(holder != crate::monitor_code::HOST_OWNER && holder != own);
    took
}

/// What a served exit or the in-VM compiler's drain writes into `Outcome` (the run-loop design's "The run loop's own
/// state"; Sprint 311): behind the run's lock with the guest, moved into `Outcome` at the run's end.
#[derive(Default)]
pub struct Records {
    pub bridged: BTreeMap<u64, u64>,
    pub blocks: BTreeMap<u64, u64>,
    pub translations: Vec<Translation>,
    pub created: Vec<Created>,
    pub thread_requests: Vec<ThreadRequest>,
    pub handovers: Vec<Handover>,
    pub vm_compile_starts: Vec<u64>,
    pub instructions_at: InstructionsAt,
}

/// The run's shared state: the guest, the host's compiler and the records (Sprint 311).
pub struct Shared<'a> {
    pub guest: &'a mut Guest,
    pub host: Option<&'a mut HostCompiler>,
    pub records: Records,
}

/// The run's shared state behind one lock (Sprint 310 the guest; Sprint 311 the host's compiler and the records with
/// it; the run-loop design's "How the threads share `Guest`"): taken once per served exit, inside the runtime word,
/// which `serve` asserts before it locks, in release builds too (card 2's guard; Sprint 311). The lock holds the run's
/// borrows, so `run`'s callers keep theirs and get them back when it is dropped. The word lives in guest memory, so the
/// lock keeps a view of the mapping, made once at its creation from the guest's own `mmap` (`GuestMemory::as_ptr`),
/// not reached through the borrow; the view is `Sync`, so card 4c's vCPU threads can share the lock
/// (`the_lock_is_shared_by_threads`).
pub struct GuestLock<'a> {
    shared: std::sync::Mutex<Shared<'a>>,
    mapping: ConcurrentGuestMemory<'a>,
}

/// Held at compile time: the lock is `Sync`, the vCPU threads sharing it by `&` (Sprint 310).
const _: fn() = || {
    fn the_lock_is_shared_by_threads<T: Sync>() {}
    the_lock_is_shared_by_threads::<GuestLock<'static>>();
};

impl<'a> GuestLock<'a> {
    pub fn new(guest: &'a mut Guest, host: Option<&'a mut HostCompiler>) -> GuestLock<'a> {
        // SAFETY: the guest's own mapping, writable, live while the borrowed guest is, which outlives the lock
        let mapping = unsafe { ConcurrentGuestMemory::from_raw_parts(guest.memory.as_ptr(), guest.memory.len()) };
        let shared = Shared {
            guest,
            host,
            records: Records::default(),
        };
        GuestLock {
            shared: std::sync::Mutex::new(shared),
            mapping,
        }
    }

    /// The mapping through the view, without the lock, live as long as the guest's borrow.
    pub fn view(&self) -> ConcurrentGuestMemory<'a> {
        self.mapping
    }

    /// The shared state for a served exit of the vCPU whose pages are `pages`: the runtime word asserted held, by the
    /// host or by that vCPU, before the lock is taken (the guest-memory design's choice 6: the word outermost); then
    /// the guest's `served.vcpu` set to `pages`, so the service reads that vCPU's own words (Sprint 313). A lock its
    /// holder left poisoned by a panic is taken as that holder left it, the panic reported where it happened.
    pub fn serve(&self, pages: VcpuPages) -> std::sync::MutexGuard<'_, Shared<'a>> {
        let mem = self.view();
        assert!(
            crate::monitor_code::runtime_word_held(mem, crate::monitor_code::vcpu_owner(mem, pages.monitor)),
            "a served exit's lock taken without the runtime word"
        );
        let mut held = self.shared.lock().unwrap_or_else(|e| e.into_inner());
        held.guest.served.vcpu = pages;
        held
    }

    /// The shared state outside a served exit, no other thread able to hold the lock (one vCPU, or the threads joined).
    pub fn get_mut(&mut self) -> &mut Shared<'a> {
        self.shared.get_mut().unwrap_or_else(|e| e.into_inner())
    }

    pub fn into_inner(self) -> Shared<'a> {
        self.shared.into_inner().unwrap_or_else(|e| e.into_inner())
    }
}

/// The process's turn ends (Sprint 197: `VM.steps` yields "idle" or "slice", vm_backend 5066, 5082): the machine runs
/// the others until this process's turn comes back; a pending wake readies its blocked threads, and on the instruction
/// clock it is caught up to the machine's (`_catch_up`). Err: the run stops (a deadlock, a child's stop). The wait holds
/// neither the lock nor the word (the guest-memory design's choice 6; Sprints 294 and 311): the guard let go, then the
/// word if the host took it; after, the word taken again, then the guard, which is handed back. The link and the kernel
/// objects are out of `Guest` across the wait.
fn turn<'l, 'a>(
    lock: &'l GuestLock<'a>,
    mut held: std::sync::MutexGuard<'l, Shared<'a>>,
    message: crate::turns::Message,
    took: bool,
    outside: &mut u64,
) -> (std::sync::MutexGuard<'l, Shared<'a>>, Result<(), String>) {
    use crate::turns::{Back, Link};
    let mut state = crate::threads::turn_state(held.guest);
    state.clock = crate::threads::counter_now(held.guest);
    let mut link = held.guest.served.link.take();
    let mut objects = std::mem::take(&mut held.guest.served.kernel);
    let pages = held.guest.served.vcpu;
    drop(held);
    if took {
        crate::monitor_code::host_release_runtime_word(lock.view());
    }
    assert!(lock.shared.try_lock().is_ok(), "a turn's wait with the run's lock held");
    let back = match &mut link {
        Some(Link::Supervisor(s)) => s.hand_over(&mut objects, state).map(|back| match back {
            Back::Run { wake, machine } | Back::Timed { wake, machine } => (wake, machine, 1 + s.children.len()),
        }),
        Some(Link::Child(c)) => {
            let (wake, machine) = c.yield_turn(message, state);
            Ok((wake, machine, 2))
        }
        None => Ok((false, 0, 1)),
    };
    if took {
        take_word(lock.view(), pages.monitor, outside);
    }
    let mut held = lock.serve(pages);
    held.guest.served.kernel = objects;
    held.guest.served.link = link;
    let handed = back.map(|(wake, machine, processes)| {
        if wake {
            crate::host_request::wake_blocked(held.guest);
        }
        crate::threads::catch_up(held.guest, machine, processes);
    });
    (held, handed)
}

pub fn run_compiling(
    guest: &mut Guest,
    loaded: &Loaded,
    vm: &Vm,
    limit: Option<Duration>,
    max_steps: u64,
    host: &mut HostCompiler,
) -> Result<Outcome, HvError> {
    run(guest, loaded, vm, limit, max_steps, Some(host))
}

/// The run on `guest.start.vcpus` vCPUs, each on a thread of its own inside a scope (Sprint 326 for one, Sprint 327
/// for several; the run-loop design's "The threads"): each thread makes its vCPU on `vm`, where it stays (`Vcpu` is
/// neither `Send` nor `Sync`), and runs it (`run_on_vcpu`). vCPU 0 starts at the program's entry; vCPU `k` from 1 on
/// its own pages and root table (`Guest::private_chain`, Sprint 315) with no thread, taking one when one is ready. The
/// first vCPU to stop gives the run its stop and stops the others (`RunStop`). With a `limit`, one more thread stops
/// the run when the limit passes, as `hv_vcpus_exit` from w2m-vm's GUEST_SECONDS or GAME_SECONDS did (Sprint 197); it
/// waits on a channel the run closes when its vCPUs end, so no stop outlives the run to reach a later vCPU given the
/// same id. The set-up before the threads, and the outcome gathered after, are the run's alone.
fn run(
    guest: &mut Guest,
    loaded: &Loaded,
    vm: &Vm,
    limit: Option<Duration>,
    max_steps: u64,
    mut host: Option<&mut HostCompiler>,
) -> Result<Outcome, HvError> {
    let md = VcpuPages::FIRST.monitor; // vCPU 0's monitor data, which the diagnostics of a one-vCPU run read
    let first = Thread {
        // the start's thread id, a child's from the Machine (`_first_tid`, vm_backend 2497; Sprint 206: it was 4)
        tid: guest.start.tid,
        index: 0,
        teb: PROCESS_VA + TEB_OFF,
        stack: guest.served.process.stack,
        exit_code: STILL_ACTIVE,
        parent: 0,
        start: loaded.entry,
        created_suspended: false,
    };
    let mut out = Outcome::empty();
    if let Err(why) = add_thread(guest, first) {
        out.stop = Stop::Exit(format!("the first thread's entry: {why}")); // a guest run before
        return Ok(out);
    }
    let vcpus = u64::from(guest.start.vcpus.max(1));
    // each vCPU's root table and pages: vCPU 0's the layout's, each other's its private chain (Sprint 315)
    let mut chains = vec![(guest.ttbr, VcpuPages::FIRST)];
    for k in 1..vcpus {
        match guest.private_chain(k) {
            Ok(chain) => chains.push(chain),
            Err(why) => {
                out.stop = Stop::Exit(format!("vCPU {k}'s pages: {why}"));
                return Ok(out);
            }
        }
    }
    {
        // `_set_budget`: one thread, the run's limit; a child shares the vCPU from its first instruction (Sprint 197)
        let budget = if guest.served.link.is_some() {
            max_steps.min(guest_runtime::layout::TIME_SLICE)
        } else {
            max_steps
        };
        let mem = guest.memory_view();
        mem.write_u64(VM_COMPILER + VMC_ENABLED, 1);
        for (k, (_, pages)) in chains.iter().enumerate() {
            let md = pages.monitor;
            mem.write_u64(md + 8, budget);
            mem.write_u32(md + MXCSR_OFF, 0x1F80);
            // the vCPU's owner, its index + 1, which its takes of the word and its claims of threads write (Sprint
            // 316's Observation 2: a private chain copies vCPU 0's page, and with it owner 1)
            mem.write_u64(md + LOCK_OWNER, k as u64 + 1);
            mem.write(md + FCW_OFF, &0x27Fu16.to_le_bytes());
        }
        // each vCPU past the first starts with no thread, counted in the run table's `waiting` until `threads::take`
        // gives it one, as a parked vCPU is (Sprint 319)
        mem.write_u64(
            RUN_TABLE + crate::threads::RT_WAITING,
            mem.read_u64(RUN_TABLE + crate::threads::RT_WAITING) + (vcpus - 1),
        );
    }
    // the Load facts the loop reads outside a service, copied before it, as each vCPU thread keeps its own (the run-loop
    // design's "How the threads share `Guest`"; Sprint 324)
    let facts = LoadFacts {
        dispatch_va: guest.code.dispatch_va,
        miss_va: guest.code.miss_va,
        time_runtime_word: guest.time_runtime_word,
        compiling: host.is_some(),
    };
    // where vCPU 0 starts: as runtime.Runtime, RSP % 16 == 8; the program's entry in X17; the runtime's entry
    let entry = Entry {
        stack: loaded.stack_top - 0x100 - 8,
        program: loaded.entry,
        runtime: guest.entry_va,
    };
    let sched = std::sync::Arc::clone(&guest.sched);
    // the run's shared state behind one lock (Sprints 310 and 311), which each vCPU's loop holds by `&` alone (Sprint
    // 324); the vCPUs' threads end inside the scope, before the lock gives the borrows back
    let lock = GuestLock::new(&mut *guest, host.as_deref_mut());
    let stopping = RunStop::default();
    let (ended, end) = std::sync::mpsc::channel::<()>();
    let ran: Vec<Result<Outcome, HvError>> = std::thread::scope(|scope| {
        let threads: Vec<_> = chains
            .iter()
            .enumerate()
            .map(|(k, &(ttbr, pages))| {
                let place = Place {
                    k: k as u64,
                    vcpus,
                    pages,
                    stopping: &stopping,
                    sched: &sched,
                };
                let lock = &lock;
                scope.spawn(move || run_on_vcpu(lock, vm, ttbr, (k == 0).then_some(entry), place, max_steps, facts))
            })
            .collect();
        if let Some(limit) = limit {
            let (stopping, sched) = (&stopping, &sched);
            scope.spawn(move || {
                if end.recv_timeout(limit) == Err(std::sync::mpsc::RecvTimeoutError::Timeout) {
                    stopping.stop(Stop::TimeLimit, sched);
                }
            });
        }
        let ran = threads
            .into_iter()
            .map(|t| t.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic)))
            .collect();
        drop(ended); // the timer leaves
        ran
    });
    for mine in ran {
        out.add_vcpu(mine?);
    }
    out.stop = stopping
        .first()
        .unwrap_or_else(|| Stop::Exit("the run's vCPUs ended with no stop".into()));
    // the records into the outcome, the borrows given back (Sprint 311)
    let records = lock.into_inner().records;
    out.bridged = records.bridged;
    out.blocks = records.blocks;
    out.translations = records.translations;
    out.created = records.created;
    out.thread_requests = records.thread_requests;
    out.handovers = records.handovers;
    out.vm_compile_starts = records.vm_compile_starts;
    out.instructions_at = records.instructions_at;
    // the stop's own row, then the file whole (Sprint 206)
    if let Some(l) = &mut guest.exit_ledger {
        let view = crate::guest::memory_view(&guest.memory);
        let warp = view.read_u64(RUN_TABLE + crate::process::CLOCK_WARP);
        // the ledger's process area as a slice, as at each exit (census D7)
        let _ = l.resume(view.read_u64(md), warp, process_below_trace(guest.memory.as_slice()));
        let _ = l.flush();
        // the blocks both compilers made (Sprint 207)
        let mut blocks: Vec<(u64, u64)> = out.translations.iter().map(|t| (t.start, t.end)).collect();
        if let Some(h) = host.as_deref() {
            blocks.extend(h.translations.iter().copied());
        }
        let _ = l.blocks(&blocks);
    }
    let mem = guest.memory_view();
    for (i, name) in VMC_COUNT_NAMES.iter().enumerate() {
        out.counts
            .insert(name, mem.read_u64(VM_COMPILER + VMC_COUNTS + 8 * i as u64));
    }
    out.compile_ticks = crate::monitor_code::CompileTicks::read(|slot| mem.read_u64(VM_COMPILER + slot));
    // every vCPU's count of instructions run, each in its own monitor data (Sprint 327)
    out.executed = chains.iter().map(|(_, pages)| mem.read_u64(pages.monitor)).sum();
    if guest.time_runtime_word {
        out.runtime_word = read_runtime_word(guest.memory_view()); // Sprint 164: read once, at the stop
    }
    Ok(out)
}

/// Where vCPU 0 starts: the guest's stack pointer, the program's entry (X17) and the runtime's entry (the PC).
#[derive(Clone, Copy)]
struct Entry {
    stack: u64,
    program: u64,
    runtime: u64,
}

/// One vCPU of the run (Sprint 327): its index, the run's count of vCPUs, its pages, and what the vCPUs share outside
/// the lock: the run's stop and the scheduler's condition a vCPU with no thread parks on.
#[derive(Clone, Copy)]
struct Place<'r> {
    k: u64,
    vcpus: u64,
    pages: VcpuPages,
    stopping: &'r RunStop,
    sched: &'r crate::threads::Sched,
}

/// The run's stop on several vCPUs (Sprint 327; the run-loop design's "The stop"): the first vCPU to stop records its
/// `Stop`, which is the run's, sets the flag, stops every vCPU through its handle and notifies the scheduler, so a
/// running vCPU leaves its run cancelled and a parked one wakes; each finds the flag and ends recording nothing.
#[derive(Default)]
struct RunStop {
    stopped: std::sync::atomic::AtomicBool,
    first: std::sync::Mutex<Option<Stop>>,
    handles: std::sync::Mutex<Vec<hypervisor::VcpuHandle>>,
}

impl RunStop {
    fn stopped(&self) -> bool {
        self.stopped.load(std::sync::atomic::Ordering::Acquire)
    }

    /// A vCPU made: its handle kept for the stop, and stopped at once if the run stopped before it was made.
    fn add(&self, handle: hypervisor::VcpuHandle) {
        self.handles.lock().unwrap_or_else(|e| e.into_inner()).push(handle);
        if self.stopped() {
            let _ = handle.stop();
        }
    }

    /// `stop` the run's, unless another vCPU's came first; then every vCPU stopped.
    fn stop(&self, stop: Stop, sched: &crate::threads::Sched) {
        self.first.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert(stop);
        self.halt(sched);
    }

    /// Every vCPU stopped, the flag set first so a vCPU cancelled by it finds it: at a stop, and when a vCPU's thread
    /// ends by an error or a panic, which would leave the others running.
    fn halt(&self, sched: &crate::threads::Sched) {
        self.stopped.store(true, std::sync::atomic::Ordering::Release);
        for handle in self.handles.lock().unwrap_or_else(|e| e.into_inner()).iter() {
            let _ = handle.stop();
        }
        sched.notify();
    }

    fn first(&self) -> Option<Stop> {
        self.first.lock().unwrap_or_else(|e| e.into_inner()).take()
    }
}

/// Halts the run when a vCPU's thread ends, by its stop, an error or a panic (Sprint 327).
struct HaltAtEnd<'r>(&'r RunStop, &'r crate::threads::Sched);

impl Drop for HaltAtEnd<'_> {
    fn drop(&mut self) {
        self.0.halt(self.1);
    }
}

/// One vCPU's thread (Sprint 327): its vCPU made here, where it stays; its registers set, vCPU 0's at the program's
/// entry, another's with no thread; its loop run; its stop, if it is the run's, given to the run; its guest's hold of
/// the runtime word let go, as that guest never runs again.
fn run_on_vcpu(
    lock: &GuestLock<'_>,
    vm: &Vm,
    ttbr: u64,
    entry: Option<Entry>,
    place: Place<'_>,
    max_steps: u64,
    facts: LoadFacts,
) -> Result<Outcome, HvError> {
    let _halt = HaltAtEnd(place.stopping, place.sched);
    let vcpu = Vcpu::new(vm)?;
    place.stopping.add(vcpu.handle());
    set_system_registers(&vcpu, ttbr)?;
    vcpu.set_reg(Reg::Fpcr, fpcr_for(0x1F80))?;
    for r in 0..31u8 {
        vcpu.set_reg(Reg::X(r), 0)?;
    }
    vcpu.set_reg(Reg::X(REG_UNDEFINED_FLAGS), 0x3F)?;
    vcpu.set_reg(Reg::X(28), MONITOR_VA + MONITOR_DATA)?;
    vcpu.set_reg(Reg::Cpsr, CPSR_EL1H)?;
    if let Some(entry) = entry {
        vcpu.set_reg(Reg::X(4), entry.stack)?;
        vcpu.set_reg(Reg::X(17), entry.program)?;
        vcpu.set_reg(Reg::Pc, entry.runtime)?;
    }
    let mut mine = Outcome::empty();
    let stop = vcpu_loop(lock, &vcpu, place, max_steps, facts, &mut mine)?;
    if let Some(stop) = stop {
        place.stopping.stop(stop, place.sched);
    }
    crate::monitor_code::release_ended_vcpus_word(lock.view(), place.k as u32 + 1);
    Ok(mine)
}

/// How a vCPU with no thread came out of `take_a_thread` (Sprint 327).
enum Taken {
    /// a thread loaded, its registers set, the vCPU to go on at the dispatcher
    Thread,
    /// the run stopped: the vCPU ends, recording nothing
    Ended,
    /// the run's stop, from this vCPU
    Stopped(Stop),
}

/// A vCPU with no thread takes one (Sprint 327; the run-loop design's "A vCPU with no thread", the Python host's
/// Sprints 118 to 121): under the word and the lock, expired waits made ready and `threads::take` asked for a ready
/// thread no other vCPU holds; with one, its registers and vectors loaded and the vCPU on at the dispatcher; with none,
/// the vCPU parks on the scheduler, holding neither the word nor the lock, until a change, the earliest blocked
/// thread's deadline, or a deadlock of every vCPU, which stops the run as one vCPU's does (`threads::DEADLOCK`).
fn take_a_thread(
    lock: &GuestLock<'_>,
    vcpu: &Vcpu<'_>,
    place: Place<'_>,
    dispatch_va: u64,
    out: &mut Outcome,
) -> Result<Taken, HvError> {
    let md = place.pages.monitor;
    loop {
        if place.stopping.stopped() {
            return Ok(Taken::Ended);
        }
        // read before the take, so a thread readied after it moves the count and the park returns at once
        let seen = place.sched.changes();
        let took = take_word(lock.view(), md, &mut out.served_outside_word);
        let (taken, deadline) = {
            let mut held = lock.serve(place.pages);
            crate::threads::wake_expired(held.guest);
            let mut x = [0u64; 31];
            let taken = crate::threads::take(held.guest, &mut x).map(|t| t.map(|(_, vslot)| (vslot, x)));
            (taken, crate::threads::park_deadline(held.guest))
        };
        if took {
            crate::monitor_code::host_release_runtime_word(lock.view());
        }
        match taken {
            Err(why) => return Ok(Taken::Stopped(Stop::Handover { pc: dispatch_va, why })),
            Ok(Some((vslot, x))) => {
                load_vectors(vcpu, lock.view(), vslot)?;
                for (r, v) in x.iter().enumerate() {
                    vcpu.set_reg(Reg::X(r as u8), *v)?;
                }
                vcpu.set_reg(Reg::Pc, dispatch_va)?;
                vcpu.set_reg(Reg::Cpsr, CPSR_EL1H)?;
                return Ok(Taken::Thread);
            }
            Ok(None) => {
                if place.sched.park(seen, deadline, place.vcpus) == crate::threads::Woken::Deadlock {
                    let why = crate::threads::DEADLOCK.into();
                    return Ok(Taken::Stopped(Stop::Handover { pc: dispatch_va, why }));
                }
            }
        }
    }
}

/// What the run loop reads of the loaded guest outside a service, copied before it (the run-loop design's "How the
/// threads share `Guest`"; Sprint 324): fixed for the run, so each vCPU thread keeps its own.
#[derive(Clone, Copy)]
struct LoadFacts {
    dispatch_va: u64,
    miss_va: u64,
    time_runtime_word: bool,
    compiling: bool,
}

/// One vCPU's run loop, to the run's stop (Sprint 324; reviewer-3's Low 1 at 324): it holds the run's lock by `&`, as
/// each vCPU thread will, so the run's shared state is reached only through `GuestLock::serve`, under the runtime word;
/// `GuestLock::get_mut`, which needs `&mut`, does not compile here or in anything the loop hands the lock to (E0596).
/// None when another vCPU's stop ended the run first (Sprint 327): a cancel, a park's wake or a turn's start finds
/// the run's flag set, and the vCPU ends recording nothing.
fn vcpu_loop(
    lock: &GuestLock<'_>,
    vcpu: &Vcpu<'_>,
    place: Place<'_>,
    max_steps: u64,
    facts: LoadFacts,
    out: &mut Outcome,
) -> Result<Option<Stop>, HvError> {
    let LoadFacts {
        dispatch_va,
        miss_va,
        time_runtime_word,
        compiling,
    } = facts;
    let pages = place.pages;
    let md = pages.monitor;
    // a vCPU past the first starts with no thread and takes one (Sprint 327)
    if place.k > 0 {
        match take_a_thread(lock, vcpu, place, dispatch_va, out)? {
            Taken::Thread => {}
            Taken::Ended => return Ok(None),
            Taken::Stopped(stop) => return Ok(Some(stop)),
        }
    }
    let runtime = MONITOR_VA + RUNTIME_IMAGE..MONITOR_VA + RUNTIME_IMAGE + RUNTIME_IMAGE_SIZE;
    let mut marked = None::<Instant>;
    let mut kind = String::new(); // the last exit's kind, whose service `marked` times
    let mut judge_ns = 0u64; // the whole-block judge's part of that service
    // what the flush stub's nested runs inside a split saw, folded into the outcome at each turn (Sprint 305)
    let counts = FlushCounts::default();
    let stop = loop {
        let flushed = counts.flushes.take();
        out.flushes += flushed;
        if flushed > 0 {
            *out.exits.entry(HVC_TLB_FLUSHED as u16).or_default() += flushed;
        }
        // a cancel that came while a split's flush ran, reported once the split was done (Sprint 305); another vCPU's
        // stop, or the run's, found at the turn's start (Sprint 327)
        if place.stopping.stopped() {
            return Ok(None);
        }
        if counts.cancelled.take() {
            break Stop::TimeLimit;
        }
        // `_before_run` (5129-5141): one vCPU, no other process: the slice's end (`_slice_until`), the VM may switch
        // threads itself; x23 the budget left, outside the runtime's C and the flush stub
        let pc = vcpu.reg(Reg::Pc)?;
        {
            // the turn's start reads the threads and the link, which another vCPU's services change, under the word and
            // the lock (the design's first form; Sprint 324), and is timed against an exit's service
            let started = Instant::now();
            let mem = lock.view();
            let took = take_word(mem, md, &mut out.served_outside_word);
            let held = lock.serve(pages);
            let guest = &*held.guest;
            let slice_until = slice_until(guest);
            let (count, limit) = (mem.read_u64(md), mem.read_u64(md + 8));
            mem.write_u64(md + SLICE_UNTIL, slice_until);
            mem.write_u64(md + RUN_END, max_steps);
            // the VM switches threads itself unless another process is due (Sprint 197)
            let others_due = guest
                .served
                .link
                .as_ref()
                .is_some_and(|l| l.others_due(crate::threads::counter_now(guest)));
            mem.write_u64(RUN_TABLE + u64::from(RT_IN_VM), u64::from(!others_due));
            drop(held);
            if took {
                crate::monitor_code::host_release_runtime_word(mem);
            }
            out.turn_start_ns.push(started.elapsed().as_nanos() as u64);
            if !runtime.contains(&pc) {
                vcpu.set_reg(Reg::X(REG_BUDGET as u8), limit.wrapping_sub(count))?;
            }
        }
        if let Some(t) = marked {
            let ns = t.elapsed().as_nanos() as u64;
            out.serve_ns.push(ns);
            // the whole-block judge's share its own kind, not the exit's (reviewer-3's Medium 1 at 257)
            out.serve_by_kind.entry(kind.clone()).or_default().push(ns - judge_ns);
            if judge_ns > 0 {
                out.serve_by_kind
                    .entry(JUDGE_KIND.to_string())
                    .or_default()
                    .push(judge_ns);
            }
        }
        let started = Instant::now();
        let exit = vcpu.run()?;
        // the run's time ends here, before the ledger hashes the process area (reviewer-3's Medium 1 of its review of
        // Sprint 210: `run_seconds` had counted the hashing)
        let returned = Instant::now();
        let mem = lock.view();
        let pc = vcpu.reg(Reg::Pc)?;
        // the exit's diagnostics, the in-VM compiler's log drained and the exit counted with the tails the drain notes,
        // under the runtime word and the lock, taken once (Sprints 275, 311 and 324): another vCPU's compile appends to
        // the log and the tails under the word. The word is in the monitor region (`RUN_TABLE`), below the process area
        // the ledger hashes, so the take leaves the hash as it was. A stop leaves the section before the word is let go
        let took = crate::monitor_code::host_take_runtime_word(mem, crate::monitor_code::vcpu_owner(mem, md));
        let served: Result<Result<(), HvError>, Stop> = 'exit: {
            let mut held = lock.serve(pages);
            let Shared { guest, host, records } = &mut *held;
            let guest: &mut Guest = guest;
            // the exit ledger's row for the last exit, the count and warp as this exit finds them, before its own
            // refresh, as the frozen host's fixture reads them at `serve_exit`'s entry: a bridge stores the count into
            // the word as the guest enters a service, so the word read before the resume lags it (Sprint 207). A
            // split's flush runs nested inside its service and never reaches this loop, so it has no row, the frozen
            // host never taking it
            if let Some(l) = &mut guest.exit_ledger {
                let view = crate::guest::memory_view(&guest.memory);
                let warp = view.read_u64(RUN_TABLE + crate::process::CLOCK_WARP);
                // the ledger hashes the process area from a slice: a diagnostic of one-vCPU runs, where the one vCPU is
                // stopped here (census D7; a copy through the view would copy the area at every exit)
                if let Err(why) = l.resume(view.read_u64(md), warp, process_below_trace(guest.memory.as_slice())) {
                    break 'exit Err(Stop::Exit(format!("the exit ledger: {why}")));
                }
            }
            if time_runtime_word {
                // the exit counted where the guest's timing reads it, so a taking that spans an exit says so (Sprint 164)
                let at = LOCK_TIMING + LOCK_EXITS;
                mem.write_u64(at, mem.read_u64(at) + 1);
            }
            out.run_ns.push((returned - started).as_nanos() as u64);
            // the exit's kind, named before the service is timed, so the naming is not counted in it (reviewer's
            // finding of 2026-10-27: Sprint 163 named it after starting the timer)
            kind = match (exit.hvc_immediate(), &exit) {
                (Some(n), _) => format!("hvc {n}"),
                (None, Exit::VtimerActivated) => "vtimer".to_string(),
                (None, e) => e
                    .exception_class()
                    .map_or_else(|| format!("{e:?}"), |c| format!("ec 0x{c:02x}")),
            };
            marked = Some(Instant::now());
            // the in-VM compiles judged against the Rust translator's, before the drain empties the log (Sprint 252)
            judge_ns = 0;
            if let Some(mut judge) = guest.block_judge.take() {
                let started = Instant::now();
                let judged = judge.judge(guest);
                judge_ns = started.elapsed().as_nanos() as u64;
                guest.block_judge = Some(judge);
                if let Err(why) = judged {
                    break 'exit Err(Stop::Exit(format!("the block judge: {why}")));
                }
            }
            vm_compiled(mem, records, host.as_deref_mut());
            let counted = if runtime.contains(&pc) {
                Ok(())
            } else {
                // the count, from x23, with a fault's give-back (5199-5208); the VM's own blocks' tails this loop does
                // not note yet (2570), the host's compiles' it does
                let give_back = match exit.hvc_immediate().map(u32::from) {
                    Some(HVC_FAULT_GIVE_BACK) => GiveBack::Fault,
                    Some(10 | 11) => GiveBack::Model { pc }, // HVC_SLOW_SSE, HVC_SLOW_STEP
                    _ => GiveBack::None,
                };
                let none = Tails::default();
                let tails = host.as_deref().map_or(&none, |h| &h.tails);
                exit_count(vcpu, mem, md, tails, give_back)
            };
            if counted.is_ok() && guest.exit_ledger.is_some() {
                // the row: kind, x86 site (the dispatch target for hvc 1, else the hvc's), and a request's op and
                // arguments
                let site = match exit.hvc_immediate() {
                    Some(1) => match vcpu.reg(Reg::X(17)) {
                        Ok(target) => target,
                        Err(e) => break 'exit Ok(Err(e)),
                    },
                    Some(_) => x86_address_at(mem, pc.wrapping_sub(4)).unwrap_or(0),
                    None => pc,
                };
                let request = (exit.hvc_immediate() == Some(8)).then(|| {
                    let t = MONITOR_SIZE + TRANSPORT_OFF;
                    let w = |i: u64| mem.read_u64(t + 8 * i);
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
            Ok(counted)
        };
        if took {
            crate::monitor_code::host_release_runtime_word(mem);
        }
        match served {
            Err(stop) => break stop,
            Ok(counted) => counted?,
        }
        let Some(number) = exit.hvc_immediate() else {
            if matches!(exit, Exit::VtimerActivated) {
                vcpu.set_vtimer_mask(true)?;
                continue;
            }
            *out.exits.entry(0).or_default() += 1;
            if matches!(exit, Exit::Canceled) {
                // `hv_vcpus_exit` from the run's stop, another vCPU's or the time limit's (w2m-vm's GUEST_SECONDS;
                // Sprint 197), which sets the flag first (Sprint 327); without it, a cancel no stop explains
                if place.stopping.stopped() {
                    return Ok(None);
                }
                break Stop::TimeLimit;
            }
            break Stop::Exit(format!("{exit:?} at pc 0x{pc:x}"));
        };
        *out.exits.entry(number).or_default() += 1;
        // a guest fault left its block (vm_backend 5294-5296; Sprint 215): the unrun instructions were given back with
        // the count above (`GiveBack::Fault`); on at the hvc's return, to the runtime's exception path
        if u32::from(number) == HVC_FAULT_GIVE_BACK {
            continue;
        }
        if number == HVC_CODE_WRITE as u16 && compiling {
            // a write faulted: behind translated code? (5297-5299) Under the runtime word (Sprint 294): it changes the host
            // compiler's tables, and whether this vCPU holds the word here is not established
            let address = vcpu.reg(Reg::X(17))?;
            let took = take_word(lock.view(), md, &mut out.served_outside_word);
            let written = {
                let mut held = lock.serve(pages);
                let s = &mut *held;
                let Some(h) = s.host.as_deref_mut() else {
                    unreachable!("a run compiling holds the host's compiler")
                };
                h.code_written(s.guest, address, &mut || flush_through_stub(vcpu, &counts))
            };
            if took {
                crate::monitor_code::host_release_runtime_word(lock.view());
            }
            match written {
                Ok(written) => vcpu.set_reg(Reg::X(17), u64::from(written))?,
                Err(why) => break Stop::Untranslated { target: address, why },
            }
            continue; // on at the hvc's return
        }
        if number == 8 {
            let op = mem.read_u64(MONITOR_SIZE + TRANSPORT_OFF);
            if !compiling {
                let why = "a run without the host's compiler serves no host request".into();
                break Stop::HostRequest { op, pc, why };
            }
            // the requests the Rust host serves (`_host_request`, 5244-5256): on at the hvc's return, or the end; the
            // guest's own hold of the word
            let mut held = lock.serve(pages);
            let Shared { guest, records, .. } = &mut *held;
            let guest: &mut Guest = guest;
            let threads = guest.served.threads.len();
            let (sub, a0, current) = (
                mem.read_u64(MONITOR_SIZE + TRANSPORT_OFF + 8),
                mem.read_u64(MONITOR_SIZE + TRANSPORT_OFF + 16),
                crate::threads::current_tid(guest),
            );
            let served = crate::host_request::serve(guest, vcpu, &counts);
            if let Some(l) = &mut guest.exit_ledger {
                let (t, mem) = (MONITOR_SIZE + TRANSPORT_OFF, crate::guest::memory_view(&guest.memory));
                l.answered(mem.read_u64(t + 48), mem.read_u64(t + 56));
            }
            if op == HOST_THREAD && matches!(sub, THREAD_EXIT_STUB | THREAD_END | THREAD_EXIT_CODE) && served.is_ok() {
                let answer = guest.memory_view().read_u64(MONITOR_SIZE + TRANSPORT_OFF + 56);
                records.thread_requests.push(ThreadRequest {
                    sub,
                    a0,
                    answer,
                    current,
                });
            }
            if guest.served.threads.len() > threads {
                let t = guest.served.threads.values().next_back().unwrap();
                let (at, mem) = (crate::threads::entry(t.index), guest.memory_view());
                let vslot = mem.read_u64(at + crate::threads::E_VSLOT);
                records.created.push(Created {
                    tid: t.tid,
                    entry: {
                        let mut entry = vec![0; RUN_ENTRY as usize];
                        mem.read(at, &mut entry);
                        entry
                    },
                    vectors: {
                        let mut vectors = vec![0; 272];
                        mem.read(vslot, &mut vectors);
                        vectors
                    },
                    count: mem.read_u64(md),
                    limit: mem.read_u64(md + 8),
                });
            }
            // the batch run's end, asked at each answered request, a point on the instruction clock (Sprint 219)
            if matches!(served, Ok(crate::host_request::Served::Answered))
                && let Some(mut end) = guest.batch_stop.take()
            {
                let asked = end.check(guest, guest.memory_view().read_u64(md));
                guest.batch_stop = Some(end);
                match asked {
                    Ok(None) => {}
                    Ok(Some(why)) => break Stop::ReferenceEnd(why),
                    Err(why) => break Stop::Exit(format!("the batch run's end: {why}")),
                }
            }
            match served {
                Ok(crate::host_request::Served::Answered) => continue,
                Ok(crate::host_request::Served::Exit(code)) => break Stop::ExitProcess { code },
                // a request w2m does not serve stops as unserved; a failure in serving one, as a failure (reviewer-3's
                // High 1 of its review of Sprint 211)
                Err(crate::host_request::HostError::NotServed(why)) => break Stop::HostRequest { op, pc, why },
                Err(e) => {
                    break Stop::HostFailure {
                        op,
                        pc,
                        why: e.to_string(),
                    };
                }
            }
        }
        if u32::from(number) == HVC_SCHEDULE && compiling {
            // a service handed the vCPU over, and threads.c `w2m_switch` left it to the host (5300-5303): `_schedule`,
            // then the next thread's registers at the dispatcher (5077-5094), its vectors swapped as the monitor's
            // `vswap` swaps them
            let at = pages.state + u64::from(STATE_SCHEDULE);
            let bits = mem.read_u32(at);
            mem.write_u32(at, 0);
            let mut x = [0u64; 31];
            for (r, v) in x.iter_mut().enumerate() {
                *v = vcpu.reg(Reg::X(r as u8))?;
            }
            // under the runtime word (Sprint 294): the switch stub released it before `hvc #12`, and the scheduler
            // changes the threads and the turn, which another vCPU's services change under it
            let took = take_word(lock.view(), md, &mut out.served_outside_word);
            let mut held = lock.serve(pages);
            let from = crate::threads::current_tid(held.guest);
            let from_vectors = vector_slot(held.guest, from);
            let mut switched = crate::threads::schedule(held.guest, &mut x, bits);
            // no thread ready in a process of a machine: the turn goes to the others until one is (Sprint 197)
            while switched == Ok(Switch::Idle) {
                let handed;
                (held, handed) = turn(
                    lock,
                    held,
                    crate::turns::Message::Idle,
                    took,
                    &mut out.served_outside_word,
                );
                switched = handed.and_then(|()| crate::threads::resume_idle(held.guest, &mut x, from, bits));
            }
            let to = crate::threads::current_tid(held.guest);
            // the hand-over recorded under the lock (Sprint 311): an ended process's to 0
            if let Ok(s) = &switched {
                let to = if matches!(s, Switch::Ended(_)) { 0 } else { to };
                held.records.handovers.push(Handover {
                    schedule: bits,
                    from,
                    to,
                });
            }
            drop(held); // the lock let go before the word, the word outermost
            if took {
                crate::monitor_code::host_release_runtime_word(lock.view());
            }
            match switched {
                Err(why) => break Stop::Handover { pc, why },
                Ok(Switch::Idle) => unreachable!(),
                Ok(Switch::Parked) => {
                    // the thread put down by the switch, its vectors saved here, then the vCPU takes another or ends
                    // (Sprint 327)
                    if let Some(slot) = from_vectors {
                        save_vectors(vcpu, lock.view(), slot)?;
                    }
                    match take_a_thread(lock, vcpu, place, dispatch_va, out)? {
                        Taken::Thread => continue,
                        Taken::Ended => return Ok(None),
                        Taken::Stopped(stop) => break stop,
                    }
                }
                Ok(Switch::Ended(code)) => break Stop::ExitProcess { code },
                Ok(Switch::Go { vectors }) => {
                    if let Some((save, load)) = vectors {
                        swap_vectors(vcpu, lock.view(), save, load)?;
                    }
                    for (r, v) in x.iter().enumerate() {
                        vcpu.set_reg(Reg::X(r as u8), *v)?;
                    }
                    vcpu.set_reg(Reg::Pc, dispatch_va)?;
                    vcpu.set_reg(Reg::Cpsr, CPSR_EL1H)?;
                    continue;
                }
            }
        }
        if number == crate::cases::HVC_SLOW_SSE || number == crate::cases::HVC_SLOW_STEP {
            // `serve_exit` (5232, 5270-5271; Sprint 181): the instruction at the hvc's site, one step of the model,
            // on at the dispatcher or the runtime's exception call as the step leaves the PC
            let rip = match x86_address_at(lock.view(), pc - 4) {
                Ok(rip) => rip,
                Err(why) => break Stop::Exit(why),
            };
            // under the runtime word and the lock, as the other services (Sprint 324): the step reads the vCPU's own
            // pages, which `serve` names; a slow step comes from a translated block, which holds nothing
            let took = take_word(lock.view(), md, &mut out.served_outside_word);
            let stepped = {
                let mut held = lock.serve(pages);
                let guest = &mut *held.guest;
                let code = code_at(guest, rip);
                crate::model_step::model_step(guest, vcpu, &code, rip)
            };
            if took {
                crate::monitor_code::host_release_runtime_word(lock.view());
            }
            match stepped? {
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
        // whether the process is one of a machine, read under the word and the lock (Sprint 324): a request makes the
        // supervisor's link mid-run (`host_request::serve`), and none removes a link once made (a turn takes it out and
        // puts it back under the lock), so the answer holds when the word is taken again below
        let linked = number == 4 && {
            let took = take_word(lock.view(), md, &mut out.served_outside_word);
            let linked = lock.serve(pages).guest.served.link.is_some();
            if took {
                crate::monitor_code::host_release_runtime_word(lock.view());
            }
            linked
        };
        if linked {
            // a slice's end in a process of a machine (`serve_exit`, 5280-5293; Sprint 197): at the run's limit a
            // stop; else the next slice's budget and expired waits woken, then the turn to another process that is
            // ready, or the next thread of this one, the stopped block's count given back as the thread goes on at its
            // entry
            let count = lock.view().read_u64(md);
            if count >= max_steps {
                break Stop::StepLimit {
                    count,
                    max: max_steps,
                    pc,
                };
            }
            let mut x = [0u64; 31];
            for (r, v) in x.iter_mut().enumerate() {
                *v = vcpu.reg(Reg::X(r as u8))?;
            }
            // under the runtime word (Sprint 294): a slice's end comes from a translated block, which holds nothing,
            // and the budget, the wakes and the scheduler change the threads and the turn; released once, after
            let took = take_word(lock.view(), md, &mut out.served_outside_word);
            let mut held = lock.serve(pages);
            let mut from_vectors = None; // the outgoing thread's vector slot, for a park (Sprint 327)
            let switched: Result<Option<Switch>, Stop> = 'slice: {
                crate::threads::set_budget(held.guest);
                crate::threads::wake_expired(held.guest);
                let now = crate::threads::counter_now(held.guest);
                let others = held.guest.served.link.as_ref().is_some_and(|l| l.others_ready(now));
                if !others && held.guest.served.threads.len() < 2 {
                    break 'slice Ok(None);
                }
                x[17] = match x86_address_at(lock.view(), pc - 4) {
                    Ok(at) => at,
                    Err(why) => break 'slice Err(Stop::Exit(why)),
                };
                if let Err(why) = give_back(held.guest, pc) {
                    break 'slice Err(Stop::Exit(why));
                }
                if others {
                    let handed;
                    (held, handed) = turn(
                        lock,
                        held,
                        crate::turns::Message::Slice,
                        took,
                        &mut out.served_outside_word,
                    );
                    if let Err(why) = handed {
                        break 'slice Err(Stop::Handover { pc, why });
                    }
                }
                let from = crate::threads::current_tid(held.guest);
                from_vectors = vector_slot(held.guest, from);
                let mut switched = crate::threads::schedule(held.guest, &mut x, 0);
                while switched == Ok(Switch::Idle) {
                    let handed;
                    (held, handed) = turn(
                        lock,
                        held,
                        crate::turns::Message::Idle,
                        took,
                        &mut out.served_outside_word,
                    );
                    switched = handed.and_then(|()| crate::threads::resume_idle(held.guest, &mut x, from, 0));
                }
                switched.map(Some).map_err(|why| Stop::Handover { pc, why })
            };
            drop(held);
            if took {
                crate::monitor_code::host_release_runtime_word(lock.view());
            }
            match switched {
                Err(stop) => break stop,
                Ok(None) => continue,
                Ok(Some(Switch::Idle)) => unreachable!(),
                Ok(Some(Switch::Parked)) => {
                    // as at `hvc #12` (Sprint 327)
                    if let Some(slot) = from_vectors {
                        save_vectors(vcpu, lock.view(), slot)?;
                    }
                    match take_a_thread(lock, vcpu, place, dispatch_va, out)? {
                        Taken::Thread => continue,
                        Taken::Ended => return Ok(None),
                        Taken::Stopped(stop) => break stop,
                    }
                }
                Ok(Some(Switch::Ended(code))) => break Stop::ExitProcess { code },
                Ok(Some(Switch::Go { vectors })) => {
                    if let Some((save, load)) = vectors {
                        swap_vectors(vcpu, lock.view(), save, load)?;
                    }
                    for (r, v) in x.iter().enumerate() {
                        vcpu.set_reg(Reg::X(r as u8), *v)?;
                    }
                    vcpu.set_reg(Reg::Pc, dispatch_va)?;
                    vcpu.set_reg(Reg::Cpsr, CPSR_EL1H)?;
                    continue;
                }
            }
        }
        // `hvc #9` (vm_backend 5255-5263): a call to an import no service answers; X17 holds its thunk, which names
        // it, and the guest's RSP (X4) the call's return address; both kept for the record, as `self.served.unimplemented`
        // (Sprint 221: 34 family runs stopped on a bare "Hvc 9", which named nothing)
        if number == 9 {
            let thunk = vcpu.reg(Reg::X(17))?;
            let rsp = vcpu.reg(Reg::X(4))?;
            // the record written under the word and the lock (Sprint 311), the run stopping after
            let took = take_word(lock.view(), md, &mut out.served_outside_word);
            let stop = {
                let mut held = lock.serve(pages);
                let guest = &mut *held.guest;
                let name = guest
                    .served
                    .thunks
                    .get(&thunk)
                    .cloned()
                    .unwrap_or_else(|| format!("{thunk:#x}"));
                match guest.layout.offset(rsp) {
                    None => Stop::Exit(format!("import {name}: the guest's RSP 0x{rsp:x} is not mapped")),
                    Some(at) => {
                        guest.served.unimplemented = Some((name.clone(), guest.memory_view().read_u64(at)));
                        Stop::Import { name, pc }
                    }
                }
            };
            if took {
                crate::monitor_code::host_release_runtime_word(lock.view());
            }
            break stop;
        }
        if number != 1 {
            break Stop::Hvc { number, pc };
        }
        let target = vcpu.reg(Reg::X(17))?;
        // the install or compile and the dispatch insert under the runtime word (Sprint 286): they write the code cursor,
        // the reconstruction table and the dispatch table, which the in-VM compiler and the installer write holding the
        // word, and the miss stub released it before `hvc 1`; released on every path before the loop goes on or stops
        let took = take_word(lock.view(), md, &mut out.served_outside_word);
        let mut held = lock.serve(pages);
        let Shared { guest, host, records } = &mut *held;
        let guest: &mut Guest = guest;
        let served: Result<u64, Stop> = 'serve: {
            if !compiling && records.bridged.contains_key(&target) {
                // its bridge was inserted at the last miss here: the table should have hit (with the host compiling, a
                // two-way slot may have let it go, and it is inserted again)
                break 'serve Err(Stop::Repeated { target });
            }
            let va = match guest.served.bridges.get(&target) {
                Some(&va) => {
                    *records.bridged.entry(target).or_default() += 1;
                    va
                }
                None => match (
                    known_block(&records.blocks, host.as_deref().map(|h| &h.blocks), target),
                    host.as_deref_mut(),
                ) {
                    (Some(va), _) => va,
                    (None, None) => break 'serve Err(Stop::Declined { target }),
                    (None, Some(h)) => match h.install(guest, target, &mut || flush_through_stub(vcpu, &counts)) {
                        // a cached block installed (Sprint 257): counted apart, not one of the compiles
                        Some(Ok(va)) => va,
                        Some(Err(why)) => break 'serve Err(Stop::Untranslated { target, why }),
                        None => match h.compile(guest, target, &mut || flush_through_stub(vcpu, &counts)) {
                            Ok(va) => {
                                let (start, end) = *h.translations.last().unwrap();
                                records.translations.push(Translation { start, end, code: va });
                                va
                            }
                            Err(why) => break 'serve Err(Stop::Untranslated { target, why }),
                        },
                    },
                },
            };
            dispatch_insert(guest.memory_view(), target, va);
            Ok(va)
        };
        drop(held);
        if took {
            crate::monitor_code::host_release_runtime_word(lock.view());
        }
        if let Err(stop) = served {
            break stop;
        }
        vcpu.set_reg(Reg::Pc, miss_va + 4)?; // ic ialluis; tlbi; dsb; isb; re-dispatch
    };
    Ok(Some(stop))
}

#[cfg(test)]
mod tests {
    use super::known_block;

    /// Each reason against a second vCPU refused by its own words, and one vCPU allowed whatever holds (Sprint 308).
    #[test]
    fn each_reason_against_a_second_vcpu_is_refused_by_name() {
        use super::vcpus_allowed;
        let lse2 = Some(1);
        assert_eq!(vcpus_allowed(1, None, &["--exit-ledger"], true, false), Ok(()));
        let refused = |r: Result<(), String>| r.expect_err("allowed");
        assert!(refused(vcpus_allowed(2, None, &[], false, false)).contains("FEAT_LSE2 not reported (None)"));
        assert!(refused(vcpus_allowed(2, Some(0), &[], false, false)).contains("FEAT_LSE2 not reported (Some(0))"));
        assert!(
            refused(vcpus_allowed(2, lse2, &["--exit-ledger"], false, false))
                .contains("--exit-ledger reads every exit")
        );
        assert!(refused(vcpus_allowed(2, lse2, &[], true, false)).contains("a child process runs on one"));
        assert!(refused(vcpus_allowed(2, lse2, &[], false, true)).contains("the instruction clock reads the running"));
        // the second vCPU is built (Sprint 327); above the layout's eight, refused
        assert_eq!(vcpus_allowed(2, lse2, &[], false, false), Ok(()));
        assert_eq!(vcpus_allowed(8, lse2, &[], false, false), Ok(()));
        assert!(refused(vcpus_allowed(9, lse2, &[], false, false)).contains("the layout has pages for 8"));
    }

    /// `super::recon_lookup` over a copy of `table`'s bytes, through the view.
    fn recon_lookup(table: &[u8], at: u64, n: u64, capacity: u64, pc: u64) -> Result<u64, String> {
        let mut bytes = table.to_vec();
        super::recon_lookup(
            crate::concurrent_memory::ConcurrentGuestMemory::new(&mut bytes),
            at,
            n,
            capacity,
            pc,
        )
    }
    use std::collections::BTreeMap;

    /// Sprint 250: with the host's compiler present, a block a write dropped from its map is unknown though the
    /// in-VM compiler's map still holds it, and a host-compiled block is known; without it, the VM's map answers.
    #[test]
    fn a_block_a_write_dropped_is_not_handed_back() {
        let vm: BTreeMap<u64, u64> = [(0x1222ee5fff, 0x7f00_0006_0000)].into();
        let host: BTreeMap<u64, u64> = [(0x1_8000_1000, 0x7f00_0007_0000)].into();
        assert_eq!(
            known_block(&vm, Some(&host), 0x1222ee5fff),
            None,
            "the dropped block's old code"
        );
        assert_eq!(known_block(&vm, Some(&host), 0x1_8000_1000), Some(0x7f00_0007_0000));
        assert_eq!(known_block(&vm, None, 0x1222ee5fff), Some(0x7f00_0006_0000));
    }

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

    /// A served exit's lock is taken only under the runtime word (Sprints 310 and 311): with the word free, `serve`
    /// fails its assertion before it locks, and the lock is left free; with the host holding the word, it gives the
    /// run's shared state. The drain of the in-VM compiler's log takes the lock so (Sprint 311), its wait for another
    /// vCPU's hold of the word being the host's take's (`monitor_code`'s test of it, Sprints 275 and 276).
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn a_served_exit_takes_the_lock_only_under_the_word() {
        use crate::monitor_code::{host_release_runtime_word, host_take_runtime_word};

        let (mut g, tid) = crate::threads::tests::guest_with_a_thread();
        let lock = super::GuestLock::new(&mut g, None);
        host_release_runtime_word(lock.view());
        let free = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            drop(lock.serve(crate::guest::VcpuPages::FIRST))
        }));
        let why = free.expect_err("a served exit with the word free");
        assert_eq!(
            why.downcast_ref::<&str>(),
            Some(&"a served exit's lock taken without the runtime word")
        );
        assert!(lock.shared.try_lock().is_ok(), "the lock left taken by a refused serve");
        assert!(host_take_runtime_word(lock.view(), 0));
        assert!(
            lock.serve(crate::guest::VcpuPages::FIRST)
                .guest
                .served
                .threads
                .contains_key(&tid)
        );
    }
}
