// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! One vCPU running one thread of a `Guest` (Sprint 142): `VM.steps` (`src/w2m/vm_backend.py` 4916-5127) for that
//! case, driven through `hypervisor` with no monitor process between. It serves the dispatch misses a bridge answers and
//! stops at the first miss that needs the host's compiler, which the translator's sprints bring. Each function cites
//! the lines it carries.

use std::collections::BTreeMap;
use std::time::Instant;

use guest_runtime::layout::{
    CODE_CACHE, CODE_CACHE_SIZE, CURRENT, CURRENT_ENTRY, DISPATCH_TABLE, MONITOR_DATA, MONITOR_SIZE, MONITOR_VA,
    PROCESS_VA, RT_ENTRIES, RUN_TABLE, RUNTIME_IMAGE, RUNTIME_IMAGE_SIZE, SLICE_UNTIL, TEB_OFF, TRANSPORT_OFF,
    VECTOR_SLOTS, VM_COMPILER, VMC_LOG, VMC_LOG_ENTRY,
};
use hypervisor::{Exit, HvError, Reg, Vcpu};
use pe_loader::Loaded;

use crate::CPSR_EL1H;
use crate::compile::HostCompiler;
use crate::guest::{Guest, RT_IN_VM, VMC_COUNT_NAMES, VMC_COUNTS, VMC_ENABLED};
use crate::monitor::set_system_registers;
use crate::monitor_code::{EMPTY_TARGET, HVC_CODE_WRITE, HVC_FAULT_GIVE_BACK, REG_BUDGET, dispatch_slot};

/// MONITOR_DATA + these (vm_backend 96, 107, 113, 119).
pub const MXCSR_OFF: u64 = 0x50;
pub const FCW_OFF: u64 = 0x70;
pub const LOCK_OWNER: u64 = 0xB0;
pub const RUN_END: u64 = 0xC8;
/// The run table's header and an entry's words (scheduler.py 42-47).
pub const RT_COUNT: u64 = 0x00;
pub const E_TID: u64 = 0x00;
pub const E_STATUS: u64 = 0x08;
pub const E_TEB: u64 = 0x20;
pub const E_VSLOT: u64 = 0x28;
pub const E_VCPU: u64 = 0x138;
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
    /// a host request (`hvc #8`) this run does not serve: the transport page's op (Sprint 155)
    HostRequest { op: u64, pc: u64 },
    /// the guest ended its process (HOST_EXIT) with `code`
    ExitProcess { code: u32 },
    /// a block the host could not compile (Sprint 155): an instruction the Rust rules do not lift, a CompileError
    Untranslated { target: u64, why: String },
    /// any other exit
    Exit(String),
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
    /// each run's time, from `hv_vcpu_run`'s call to its return, in ns
    pub run_ns: Vec<u64>,
    /// the exits, by hvc number (0 for a non-hvc exit)
    pub exits: BTreeMap<u16, u64>,
}

fn read_u64(mem: &[u8], off: u64) -> u64 {
    u64::from_le_bytes(mem[off as usize..off as usize + 8].try_into().unwrap())
}

fn write_u64(mem: &mut [u8], off: u64, v: u64) {
    mem[off as usize..off as usize + 8].copy_from_slice(&v.to_le_bytes());
}

/// `_add_thread` for the first thread (vm_backend 3328-3344; `GuestThread`, scheduler.py 60-71; `_vslot`, 3346-3350;
/// `set_current`, 3323-3326): its run-table entry, ready, its vector slot, current on vCPU 0.
fn add_first_thread(mem: &mut [u8], tid: u64, teb: u64) {
    let at = RUN_TABLE + RT_ENTRIES; // entry 0
    write_u64(mem, at + E_TID, tid);
    write_u64(mem, at + E_STATUS, 0);
    write_u64(mem, at + E_TEB, teb);
    write_u64(mem, at + E_VCPU, 0);
    write_u64(mem, RUN_TABLE + RT_COUNT, 1);
    write_u64(mem, at + E_VSLOT, VECTOR_SLOTS);
    write_u64(mem, MONITOR_DATA + CURRENT, 0);
    write_u64(mem, MONITOR_DATA + CURRENT_ENTRY, MONITOR_VA + at);
    write_u64(mem, at + E_VCPU, 1); // the first thread starts on vCPU 0
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

/// `_vm_compiled` (vm_backend 2559-2580), the blocks only: each log entry's start and code address, then the log
/// emptied.
fn vm_compiled(mem: &mut [u8], blocks: &mut BTreeMap<u64, u64>) {
    let n = read_u64(mem, VM_COMPILER + VMC_LOG_COUNT);
    for i in 0..n {
        let at = VM_COMPILER + VMC_LOG + VMC_LOG_ENTRY * i;
        blocks.insert(read_u64(mem, at), read_u64(mem, at + 16));
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
    {
        let mem = guest.memory.as_mut_slice();
        write_u64(mem, md + 8, max_steps); // `_set_budget`: one thread, the run's limit
        write_u64(mem, VM_COMPILER + VMC_ENABLED, 1);
        add_first_thread(mem, 4, PROCESS_VA + TEB_OFF);
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

    let mut out = Outcome {
        stop: Stop::Exit(String::new()),
        bridged: BTreeMap::new(),
        blocks: BTreeMap::new(),
        counts: BTreeMap::new(),
        serve_ns: Vec::new(),
        run_ns: Vec::new(),
        exits: BTreeMap::new(),
    };
    let runtime = MONITOR_VA + RUNTIME_IMAGE..MONITOR_VA + RUNTIME_IMAGE + RUNTIME_IMAGE_SIZE;
    let mut marked = None::<Instant>;
    let stop = loop {
        // `_before_run` (5129-5141): one thread, no other process: the slice never needs the host, and the VM may
        // switch threads itself; x23 the budget left, outside the runtime's C
        let pc = vcpu.reg(Reg::Pc)?;
        {
            let mem = guest.memory.as_mut_slice();
            let (count, limit) = (read_u64(mem, md), read_u64(mem, md + 8));
            write_u64(mem, md + SLICE_UNTIL, u64::MAX);
            write_u64(mem, md + RUN_END, max_steps);
            write_u64(mem, RUN_TABLE + u64::from(RT_IN_VM), 1);
            if !runtime.contains(&pc) {
                vcpu.set_reg(Reg::X(REG_BUDGET as u8), limit.wrapping_sub(count))?;
            }
        }
        if let Some(t) = marked {
            out.serve_ns.push(t.elapsed().as_nanos() as u64);
        }
        let started = Instant::now();
        let exit = vcpu.run()?;
        out.run_ns.push(started.elapsed().as_nanos() as u64);
        marked = Some(Instant::now());
        let mem = guest.memory.as_mut_slice();
        vm_compiled(mem, &mut out.blocks);
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
        let Some(number) = exit.hvc_immediate() else {
            if matches!(exit, Exit::VtimerActivated) {
                vcpu.set_vtimer_mask(true)?;
                continue;
            }
            *out.exits.entry(0).or_default() += 1;
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
                break Stop::HostRequest { op, pc };
            }
            // the requests the Rust host serves (`_host_request`, 5244-5256): on at the hvc's return, or the end
            match crate::host_request::serve(guest, vcpu) {
                Ok(crate::host_request::Served::Answered) => continue,
                Ok(crate::host_request::Served::Exit(code)) => break Stop::ExitProcess { code },
                Err(_) => break Stop::HostRequest { op, pc },
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
                        Ok(va) => va,
                        Err(why) => break Stop::Untranslated { target, why },
                    },
                },
            },
        };
        let mem = guest.memory.as_mut_slice();
        dispatch_insert(mem, target, va);
        vcpu.set_reg(Reg::Pc, guest.code.miss_va + 4)?; // ic ialluis; tlbi; dsb; isb; re-dispatch
    };
    let mem = guest.memory.as_slice();
    for (i, name) in VMC_COUNT_NAMES.iter().enumerate() {
        out.counts
            .insert(name, read_u64(mem, VM_COMPILER + VMC_COUNTS + 8 * i as u64));
    }
    out.stop = stop;
    Ok(out)
}
