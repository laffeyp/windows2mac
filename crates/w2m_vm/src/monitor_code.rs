// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The monitor code the Python host writes at a VM's start, written from Rust (Sprint 136): the vectors, the
//! dispatcher, the empty dispatch table and the exception entry (`src/w2m/vm_backend.py` `_write_monitor`,
//! 4207-4267, and `_write_exception_entry`, 4343-4400), with the runtime lock and budget helpers they use (521-557).
//! Each function carries the lines it names, word for word in the Python's order.

use std::collections::BTreeMap;

use a64_encoder as a64;
use guest_runtime::layout::{
    DISPATCH_BITS, DISPATCH_ENTRIES, DISPATCH_TABLE, LOCK_OWNER, MAX_VCPUS, MONITOR_DATA, MONITOR_SIZE, MONITOR_VA,
    PAGE, PROCESS_VA, RUN_TABLE, RUNTIME_STACK, RUNTIME_STACK_SIZE, SAVE_AREA, STATE_OFF, VCPU_AREA, VCPU_STRIDE,
    VM_COMPILER,
};

use crate::assembler::{Assembler, Target};
use crate::{CPSR_EL1H, DISPATCHER, VECTORS};

/// VM_COMPILER + this (Sprint 247): the virtual counter's ticks the miss stub spent calling `w2m_vm_compile`, summed
/// when `Timing::compile` asks; past the C compiler's 23 counts at 0x100 (`guest::VMC_COUNTS`, which `guest.rs` asserts
/// at compile time) and before `VMC_SAVE`. Here, not in `guest`, since every target builds the monitor code.
pub const VMC_COMPILE_TICKS: u64 = 0x1B8;
/// VM_COMPILER + these, after the sum (Sprint 248), in counter ticks: the longest one compile; the open frame's start
/// and its compile ticks; the most compile ticks any closed frame held. A frame is 1/`FRAMES_A_SECOND` s.
pub const VMC_COMPILE_LONGEST: u64 = 0x1C0;
pub const VMC_FRAME_START: u64 = 0x1C8;
pub const VMC_FRAME_TICKS: u64 = 0x1D0;
pub const VMC_FRAME_MOST: u64 = 0x1D8;
/// A 60 Hz display's frames (Sprint 248): the span whose compiling the probe's frames sum.
pub const FRAMES_A_SECOND: u64 = 60;

/// The compile probe's figures at a stop, in counter ticks; all 0 unless `Timing::compile` asked for them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CompileTicks {
    pub total: u64,
    pub longest: u64,
    pub most_in_a_frame: u64,
}

impl CompileTicks {
    /// The figures from the slots past `VM_COMPILER`, read by `slot`; the frame still open at the stop is closed, its
    /// sum folded into the most.
    pub fn read(slot: impl Fn(u64) -> u64) -> CompileTicks {
        CompileTicks {
            total: slot(VMC_COMPILE_TICKS),
            longest: slot(VMC_COMPILE_LONGEST),
            most_in_a_frame: slot(VMC_FRAME_MOST).max(slot(VMC_FRAME_TICKS)),
        }
    }
}

/// The exception entry's area (vm_backend 129-130).
pub const EXCEPTION_ENTRY: u64 = 0x01C00;
pub const EXCEPTION_ENTRY_END: u64 = 0x02000;
/// MONITOR_DATA + this: x17 while the exception entry asks about a write to code (vm_backend 111).
pub const CODE_WRITE_SPILL: u64 = 0x88;
/// The save area's vectors, Q0-Q15 (vm_backend 125).
pub const SAVE_VECTORS: u64 = 0x200;
/// RUN_TABLE + this: the runtime lock (vm_backend 518).
pub const RUNTIME_LOCK: u64 = 0x80;
/// x23 holds the step budget left (vm_backend 368).
pub const REG_BUDGET: u32 = 23;
/// The in-VM compiler's words (vm_backend 197, 204).
pub const VMC_ENABLED: u64 = 0x00;
pub const VMC_SAVE: u64 = 0x200;
/// hvc numbers (vm_backend 1127-1128).
pub const HVC_CODE_WRITE: u32 = 13;
pub const HVC_FAULT_GIVE_BACK: u32 = 14;
/// A level-3 permission fault on a write from the current EL (vm_backend 1131-1132).
pub const CODE_WRITE_ESR_MASK: u64 = (0x3F << 26) | (1 << 6) | 0x3F;
pub const CODE_WRITE_ESR: u64 = (0x25 << 26) | (1 << 6) | 0x0F;
/// The state block's `redirect_rip` (vm_backend 1133).
pub const STATE_REDIRECT: u32 = 0xE8;
/// An empty dispatch slot's target: no x86 address (`M64`).
pub const EMPTY_TARGET: u64 = u64::MAX;

/// `dispatch_slot` (vm_backend 158-162): the even index of a target's pair.
pub fn dispatch_slot(target: u64) -> u64 {
    (target ^ target >> DISPATCH_BITS ^ target >> (2 * DISPATCH_BITS)) & (DISPATCH_ENTRIES - 2)
}

/// `lock_runtime` (vm_backend 521-541).
pub fn lock_runtime(a: &mut Assembler) {
    let (spin, wait, got) = (a.label(), a.label(), a.label());
    a.bind(spin);
    a.emit(&a64::mov_imm(18, MONITOR_VA + MONITOR_DATA + LOCK_OWNER));
    a.emit(&[a64::ldr(4, 18, 18, 0)]);
    a.emit(&a64::mov_imm(16, MONITOR_VA + RUN_TABLE + RUNTIME_LOCK));
    a.emit(&[
        a64::CASA_WZR_W18_X16,
        a64::ldr(4, 16, 16, 0),
        a64::eor(16, 16, 18, a64::LSL, 0),
    ]);
    a.branch(|off| a64::cbz(16, off), got);
    a.emit(&a64::mov_imm(16, MONITOR_VA + RUN_TABLE + RUNTIME_LOCK));
    a.bind(wait);
    a.emit(&[a64::ldr(4, 18, 16, 0)]);
    a.branch(|off| a64::cbnz(18, off), wait);
    a.branch(a64::b, spin);
    a.bind(got);
}

/// `unlock_runtime` (vm_backend 544-546).
pub fn unlock_runtime(a: &mut Assembler) {
    a.emit(&a64::mov_imm(16, MONITOR_VA + RUN_TABLE + RUNTIME_LOCK));
    a.emit(&[a64::STLR_WZR_X16]);
}

/// MONITOR_VA + this: the runtime word's timing area (Sprint 164), in the monitor region past its eight vCPU areas,
/// where the region's rounding up to a 2 MB block leaves 1,331,200 bytes no one uses (`VCPU_AREA + 8 * VCPU_STRIDE` is
/// 0xA8BB000, `MONITOR_SIZE` 0xAA00000). The header: the taking in hand's wait start at +0, taken time at +8, and the
/// exit count at each at +16 and +24 (counter ticks; exits as the host counts them at +40); the takings at +32,
/// counted even past the area's end; then `LOCK_TAKINGS` entries of 64 bytes from `LOCK_ENTRIES`, one per taking:
/// wait start, taken, released, site (`Site`), and the exit count at each of the three. The run reads them at its
/// stop.
pub const LOCK_TIMING: u64 = (VCPU_AREA + MAX_VCPUS * VCPU_STRIDE + PAGE - 1) & !(PAGE - 1);
pub const LOCK_TAKINGS_COUNT: u64 = 32;
pub const LOCK_EXITS: u64 = 40;
/// the release time while the entry is written
const LOCK_RELEASED: u64 = 48;
pub const LOCK_ENTRIES: u64 = 0x40;
pub const LOCK_ENTRY: u64 = 64;
pub const LOCK_TAKINGS: u64 = 16384;
const _: () = assert!(LOCK_TIMING + LOCK_ENTRIES + LOCK_ENTRY * LOCK_TAKINGS <= MONITOR_SIZE);

/// Where the runtime word was taken (the entry's site word).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Site {
    /// the switch stub, from the switch to the register reload (`guest::switch_stub`)
    Switch = 0,
    /// a bridge, around a C service (`guest::bridge`)
    Bridge = 1,
    /// the dispatcher, around the VM compiler's call
    Compile = 2,
    /// the exception entry, around `w2m_exception`
    Exception = 3,
}

impl Site {
    pub const ALL: [Site; 4] = [Site::Switch, Site::Bridge, Site::Compile, Site::Exception];

    pub fn name(self) -> &'static str {
        match self {
            Site::Switch => "switch",
            Site::Bridge => "bridge",
            Site::Compile => "compile",
            Site::Exception => "exception",
        }
    }
}

/// The virtual counter and the host's exit count to the header at `time_off` and `exits_off`, through x16 and x18.
/// The ISB keeps the counter's read from moving ahead of the instructions before it (Arm ARM: a CNTVCT_EL0 read may
/// occur out of order; Linux reads it as `isb; mrs`, `arch/arm64/include/asm/arch_timer.h`
/// `arch_timer_read_cntvct_el0`, read 2026-10-28).
fn stamp(a: &mut Assembler, time_off: u64, exits_off: u64) {
    a.emit(&[a64::ISB, a64::mrs(18, a64::CNTVCT_EL0)]);
    a.emit(&a64::mov_imm(16, MONITOR_VA + LOCK_TIMING));
    a.emit(&[
        a64::str(8, 18, 16, time_off as u32),
        a64::ldr(8, 18, 16, LOCK_EXITS as u32),
        a64::str(8, 18, 16, exits_off as u32),
    ]);
}

/// The runtime word taken, timed when `timed`: the counter and the exit count before the first attempt (the wait's
/// start) and after the taking. `lock_runtime` clobbers x16 and x18 already; so does this.
pub fn take_runtime(a: &mut Assembler, timed: bool) {
    if timed {
        stamp(a, 0, 16);
    }
    lock_runtime(a);
    if timed {
        stamp(a, 8, 24);
    }
}

/// The taking written to the next entry with the release time and exit count, and counted, through three registers
/// the site has free (`base`, `r1`, `r2`). Past `LOCK_TAKINGS` the taking is counted and not written, and the run's
/// reader refuses the record.
pub fn record_taking(a: &mut Assembler, site: Site, base: u32, r1: u32, r2: u32) {
    let full = a.label();
    a.emit(&[a64::ISB, a64::mrs(r2, a64::CNTVCT_EL0)]);
    a.emit(&a64::mov_imm(base, MONITOR_VA + LOCK_TIMING));
    a.emit(&[
        a64::str(8, r2, base, LOCK_RELEASED as u32),
        a64::ldr(8, r1, base, LOCK_TAKINGS_COUNT as u32),
        a64::movz(r2, LOCK_TAKINGS, 0),
        a64::cmp(r1, r2),
    ]);
    a.branch(|off| a64::bcond("hs", off), full);
    let e = LOCK_ENTRIES as u32;
    a.emit(&[
        a64::add(r1, base, r1, a64::LSL, LOCK_ENTRY.ilog2()),
        a64::ldr(8, r2, base, LOCK_RELEASED as u32),
        a64::str(8, r2, r1, e + 16), // released
        a64::ldr(8, r2, base, 0),
        a64::str(8, r2, r1, e), // wait start
        a64::ldr(8, r2, base, 8),
        a64::str(8, r2, r1, e + 8), // taken
        a64::movz(r2, site as u64, 0),
        a64::str(8, r2, r1, e + 24),
        a64::ldr(8, r2, base, 16),
        a64::str(8, r2, r1, e + 32), // exits at the wait start
        a64::ldr(8, r2, base, 24),
        a64::str(8, r2, r1, e + 40), // exits at the taking
        a64::ldr(8, r2, base, LOCK_EXITS as u32),
        a64::str(8, r2, r1, e + 48), // exits at the release
    ]);
    a.bind(full);
    a.emit(&[
        a64::ldr(8, r2, base, LOCK_TAKINGS_COUNT as u32),
        a64::addi(r2, r2, 1),
        a64::str(8, r2, base, LOCK_TAKINGS_COUNT as u32),
    ]);
}

/// The runtime word released, the taking recorded first when `timed`, through `free` (three registers the site does
/// not need).
pub fn release_runtime(a: &mut Assembler, timed: bool, site: Site, free: (u32, u32, u32)) {
    if timed {
        record_taking(a, site, free.0, free.1, free.2);
    }
    unlock_runtime(a);
}

/// `count_from_budget` (vm_backend 549-551).
pub fn count_from_budget() -> [u32; 3] {
    [
        a64::ldr(8, 17, 28, 8),
        a64::sub(17, 17, REG_BUDGET),
        a64::str(8, 17, 28, 0),
    ]
}

/// `budget_from_count` (vm_backend 554-556).
pub fn budget_from_count() -> [u32; 2] {
    [a64::ldp(16, 17, 28, 0), a64::sub(REG_BUDGET, 17, 16)]
}

/// The vectors (vm_backend 4209-4216): the synchronous current-EL slot (VBAR + 0x200) branches to the exception
/// entry; every other slot is `hvc #2`, then spins.
pub fn vectors() -> Vec<u32> {
    let mut vec = Assembler::new(MONITOR_VA + VECTORS);
    for slot in 0..16 {
        if slot == 4 {
            vec.branch(a64::b, Target::Va(MONITOR_VA + EXCEPTION_ENTRY));
            vec.emit(&[a64::b(0); 31]);
        } else {
            vec.emit(&[a64::hvc(2)]);
            vec.emit(&[a64::b(0); 31]);
        }
    }
    vec.finish()
}

/// What the monitor code times; the default times nothing and writes the Python's words.
#[derive(Debug, Clone, Copy, Default)]
pub struct Timing {
    /// the runtime word's takings (Sprint 164)
    pub runtime_word: bool,
    /// the miss stub's call of `w2m_vm_compile`, its ticks summed at `VM_COMPILER + VMC_COMPILE_TICKS` (Sprint 247),
    /// the longest and the most in a frame beside them (Sprint 248)
    pub compile: bool,
}

/// How the monitor code is written: what it times, and the targets its miss stub sends to the host before the in-VM
/// compiler (Sprint 257: the translation cache's image, `[base, base + size)`, which the host installs from the cache).
/// The default writes the Python's words.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    pub timing: Timing,
    pub host_range: Option<(u64, u64)>,
}

/// After the call of `w2m_vm_compile`, x26 holding the counter at the call: the compile's ticks added to the sum
/// (Sprint 247), kept if the longest, and added to the open frame's (Sprint 248). A frame opens at a compile's start and
/// holds every compile that ends within 1/`FRAMES_A_SECOND` s of it; the first to end past it folds the frame's sum
/// into the most and opens the next at its own start. x16, x18 and x26 are free here, as after the restore, and x9 to
/// x13, saved at `VMC_SAVE` before the call, until the restore.
fn compile_probe(d: &mut Assembler) {
    const LONGEST: u32 = (VMC_COMPILE_LONGEST - VMC_COMPILE_TICKS) as u32;
    const START: u32 = (VMC_FRAME_START - VMC_COMPILE_TICKS) as u32;
    const FRAME: u32 = (VMC_FRAME_TICKS - VMC_COMPILE_TICKS) as u32;
    const MOST: u32 = (VMC_FRAME_MOST - VMC_COMPILE_TICKS) as u32;
    d.emit(&[a64::ISB, a64::mrs(9, a64::CNTVCT_EL0), a64::sub(18, 9, 26)]);
    d.emit(&a64::mov_imm(16, MONITOR_VA + VM_COMPILER + VMC_COMPILE_TICKS));
    d.emit(&[
        a64::ldr(8, 10, 16, 0),
        a64::add(10, 10, 18, 0, 0),
        a64::str(8, 10, 16, 0),
    ]);
    let shorter = d.label();
    d.emit(&[a64::ldr(8, 10, 16, LONGEST), a64::cmp(18, 10)]);
    d.branch(|off| a64::bcond("ls", off), shorter);
    d.emit(&[a64::str(8, 18, 16, LONGEST)]);
    d.bind(shorter);
    // past the frame when (end - start) * frames a second reaches the frequency
    let within = d.label();
    let fewer = d.label();
    d.emit(&[
        a64::ldr(8, 11, 16, START),
        a64::sub(11, 9, 11),
        a64::mrs(12, a64::CNTFRQ_EL0),
    ]);
    d.emit(&a64::mov_imm(13, FRAMES_A_SECOND));
    d.emit(&[a64::mul(11, 11, 13), a64::cmp(11, 12)]);
    d.branch(|off| a64::bcond("lo", off), within);
    d.emit(&[a64::ldr(8, 11, 16, FRAME), a64::ldr(8, 12, 16, MOST), a64::cmp(11, 12)]);
    d.branch(|off| a64::bcond("ls", off), fewer);
    d.emit(&[a64::str(8, 11, 16, MOST)]);
    d.bind(fewer);
    d.emit(&[a64::str(8, 26, 16, START), a64::str(8, 31, 16, FRAME)]);
    d.bind(within);
    d.emit(&[
        a64::ldr(8, 11, 16, FRAME),
        a64::add(11, 11, 18, 0, 0),
        a64::str(8, 11, 16, FRAME),
    ]);
}

/// The dispatcher (vm_backend 4217-4258): its words and the miss stub's address, written as `options` asks.
pub fn dispatcher(symbols: &BTreeMap<String, u64>, options: Options) -> (Vec<u32>, u64) {
    let timing = options.timing;
    let timed = timing.runtime_word;
    let mut d = Assembler::new(MONITOR_VA + DISPATCHER);
    let (top, miss) = (d.label(), d.label());
    d.bind(top);
    let second = d.label();
    let bits = DISPATCH_BITS as u32;
    d.emit(&[
        a64::ldr(8, 16, 28, 16),
        a64::eor(18, 17, 17, a64::LSR, bits),
        a64::eor(18, 18, 17, a64::LSR, 2 * bits),
        a64::ubfx(18, 18, 1, bits - 1),
        a64::add(16, 16, 18, a64::LSL, 5), // the pair: 32 bytes
        a64::ldp(18, 25, 16, 0),
        a64::cmp(18, 17),
    ]);
    d.branch(|off| a64::bcond("ne", off), second);
    d.emit(&[a64::br(25)]);
    d.bind(second);
    d.emit(&[a64::ldp(18, 25, 16, 16), a64::cmp(18, 17)]);
    d.branch(|off| a64::bcond("ne", off), miss);
    d.emit(&[a64::br(25)]);
    d.bind(miss);
    // the compiler inside the VM first (Sprint 083; VMC_ENABLED)
    let host = d.label();
    // a target in the host's range goes to the host first, which installs it from the translation cache (Sprint 257);
    // x16 is free here
    if let Some((lo, hi)) = options.host_range {
        let outside = d.label();
        d.emit(&a64::mov_imm(16, lo));
        d.emit(&[a64::cmp(17, 16)]);
        d.branch(|off| a64::bcond("lo", off), outside);
        d.emit(&a64::mov_imm(16, hi));
        d.emit(&[a64::cmp(17, 16)]);
        d.branch(|off| a64::bcond("lo", off), host);
        d.bind(outside);
    }
    d.emit(&a64::mov_imm(16, MONITOR_VA + VM_COMPILER));
    d.emit(&[a64::ldr(8, 16, 16, VMC_ENABLED as u32)]);
    d.branch(|off| a64::cbz(16, off), host);
    take_runtime(&mut d, timed);
    d.emit(&a64::mov_imm(16, MONITOR_VA + VM_COMPILER + VMC_SAVE));
    for r in (0..16).step_by(2) {
        d.emit(&[a64::stp(r, r + 1, 16, 8 * i64::from(r))]);
    }
    d.emit(&[a64::stp(17, 30, 16, 128)]);
    d.emit(&a64::mov_imm(16, MONITOR_VA + RUNTIME_STACK + RUNTIME_STACK_SIZE));
    d.emit(&[a64::addi(31, 16, 0), a64::mov(0, 17)]);
    if timing.compile {
        // x26, dead between blocks and kept by the C call (AAPCS64), holds the counter at the call (Sprint 247)
        d.emit(&[a64::ISB, a64::mrs(26, a64::CNTVCT_EL0)]);
    }
    d.emit(&a64::mov_imm(16, symbols["w2m_vm_compile"]));
    d.emit(&[a64::blr(16), a64::mov(25, 0)]);
    if timing.compile {
        compile_probe(&mut d);
    }
    d.emit(&a64::mov_imm(16, MONITOR_VA + VM_COMPILER + VMC_SAVE));
    for r in (0..16).step_by(2) {
        d.emit(&[a64::ldp(r, r + 1, 16, 8 * i64::from(r))]);
    }
    d.emit(&[a64::ldp(17, 30, 16, 128)]);
    // x25 holds the compiled code; x16, x18 and x26 are free (x18 and x26 are temporaries, dead between blocks)
    release_runtime(&mut d, timed, Site::Compile, (16, 18, 26));
    d.branch(|off| a64::cbz(25, off), host);
    d.emit(&[a64::br(25)]);
    d.bind(host);
    let miss_va = d.here();
    // drop stale TLB entries too, on every vCPU (Sprint 104)
    d.emit(&[
        a64::hvc(1),
        a64::IC_IALLUIS,
        a64::TLBI_VMALLE1IS,
        a64::DSB_ISH,
        a64::ISB,
    ]);
    d.branch(a64::b, top);
    let words = d.finish();
    assert!(
        4 * words.len() as u64 <= EXCEPTION_ENTRY - DISPATCHER,
        "the dispatcher overflows into the exception entry"
    );
    (words, miss_va)
}

/// The exception entry (vm_backend 4343-4400): its words and the call stub's address (`exception_call_va`).
/// `resume_va` is where it goes on after the runtime dispatched the exception: the dispatcher. `timed`: the runtime
/// word's takings timed (Sprint 164).
pub fn exception_entry(symbols: &BTreeMap<String, u64>, resume_va: u64, timed: bool) -> (Vec<u32>, u64) {
    let mut e = Assembler::new(MONITOR_VA + EXCEPTION_ENTRY);
    // a write to a page behind translated code: ask the host, then return to the store (x16 waits in SP, x17 in the
    // monitor data)
    let (other, back) = (e.label(), e.label());
    e.emit(&[a64::addi(31, 16, 0)]);
    e.emit(&a64::mov_imm(16, MONITOR_VA + MONITOR_DATA + CODE_WRITE_SPILL));
    e.emit(&[a64::str(8, 17, 16, 0), a64::mrs(17, a64::ESR_EL1)]);
    e.emit(&a64::mov_imm(16, CODE_WRITE_ESR_MASK));
    e.emit(&[a64::and(17, 17, 16)]);
    e.emit(&a64::mov_imm(16, CODE_WRITE_ESR));
    e.emit(&[a64::cmp(17, 16)]);
    e.branch(|off| a64::bcond("ne", off), other);
    e.emit(&[a64::mrs(17, a64::FAR_EL1), a64::hvc(HVC_CODE_WRITE)]); // x17: 1 when the host dropped blocks
    e.branch(|off| a64::cbz(17, off), other);
    e.emit(&[a64::TLBI_VMALLE1IS, a64::DSB_ISH, a64::ISB]);
    e.branch(a64::b, back);
    e.bind(other);
    e.emit(&a64::mov_imm(16, MONITOR_VA + MONITOR_DATA + CODE_WRITE_SPILL));
    e.emit(&[a64::ldr(8, 17, 16, 0), a64::addi(16, 31, 0)]);
    e.emit(&a64::mov_imm(16, MONITOR_VA + MONITOR_DATA + SAVE_AREA));
    for i in (0..30).step_by(2) {
        e.emit(&[a64::stp(i, i + 1, 16, 8 * i64::from(i))]);
    }
    e.emit(&[a64::str(8, 30, 16, 8 * 30)]);
    e.emit(&[
        a64::mrs(0, a64::ESR_EL1),
        a64::mrs(1, a64::FAR_EL1),
        a64::mrs(2, a64::ELR_EL1),
        a64::stp(0, 1, 16, 0x100),
        a64::str(8, 2, 16, 0x110),
        a64::stp(31, 31, 16, 0x118),
    ]);
    // the block's entry counted every instruction in it; the host takes back the unrun ones off x23
    e.emit(&[a64::hvc(HVC_FAULT_GIVE_BACK)]);
    for i in (0..16).step_by(2) {
        e.emit(&[a64::stp_q(i, i + 1, 16, (SAVE_VECTORS + 16 * u64::from(i)) as i64)]);
    }
    let cont = e.here() + 4 * 8; // four words of MOV, then MSR, MOVZ, MSR, ERET
    let mut words = vec![a64::movz(17, cont & 0xFFFF, 0)];
    words.extend((1..4).map(|h| a64::movk(17, (cont >> (16 * h)) & 0xFFFF, h)));
    words.extend([
        a64::msr(a64::ELR_EL1, 17),
        a64::movz(17, CPSR_EL1H, 0),
        a64::msr(a64::SPSR_EL1, 17),
        a64::ERET,
    ]);
    e.emit(&words);
    assert_eq!(e.here(), cont);
    let exception_call_va = cont;
    e.emit(&count_from_budget());
    take_runtime(&mut e, timed);
    e.emit(&a64::mov_imm(16, MONITOR_VA + RUNTIME_STACK + RUNTIME_STACK_SIZE));
    e.emit(&[a64::addi(31, 16, 0)]);
    e.emit(&a64::mov_imm(16, symbols["w2m_exception"]));
    e.emit(&[a64::blr(16)]);
    e.emit(&budget_from_count());
    // x0-x15 are reloaded from the state block after: x1 and x2 are free, with x16
    release_runtime(&mut e, timed, Site::Exception, (16, 1, 2));
    e.emit(&a64::mov_imm(16, PROCESS_VA + STATE_OFF));
    for i in (0..16).step_by(2) {
        e.emit(&[a64::ldp(i, i + 1, 16, 8 * i64::from(i))]);
    }
    e.emit(&[
        a64::ldr(8, 17, 16, STATE_REDIRECT),
        a64::str(8, a64::ZR, 16, STATE_REDIRECT),
    ]);
    e.branch(a64::b, Target::Va(resume_va));
    e.bind(back);
    e.emit(&a64::mov_imm(16, MONITOR_VA + MONITOR_DATA + CODE_WRITE_SPILL));
    e.emit(&[a64::ldr(8, 17, 16, 0), a64::addi(16, 31, 0), a64::ERET]);
    let words = e.finish();
    assert!(
        4 * words.len() as u64 <= EXCEPTION_ENTRY_END - EXCEPTION_ENTRY,
        "the exception entry overflows its area"
    );
    (words, exception_call_va)
}

fn write_words(mem: &mut [u8], off: u64, words: &[u32]) {
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    mem[off as usize..off as usize + bytes.len()].copy_from_slice(&bytes);
}

/// Where the monitor code put its entry points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonitorCode {
    pub dispatch_va: u64,
    pub miss_va: u64,
    pub exception_call_va: u64,
}

/// Writes the monitor code into the monitor area (`mem` starts at `MONITOR_VA`): `_write_monitor`, vCPU 0's monitor
/// data header (count 0, limit 0, the dispatch table's address), the vectors, the dispatcher, the empty table and the
/// exception entry, timed as `timing` asks (the default writes the Python's words).
pub fn write(mem: &mut [u8], symbols: &BTreeMap<String, u64>, options: Options) -> MonitorCode {
    let timed = options.timing.runtime_word;
    let header: Vec<u8> = [0u64, 0, MONITOR_VA + DISPATCH_TABLE]
        .iter()
        .flat_map(|q| q.to_le_bytes())
        .collect();
    mem[MONITOR_DATA as usize..MONITOR_DATA as usize + 24].copy_from_slice(&header);
    write_words(mem, VECTORS, &vectors());
    let (d, miss_va) = dispatcher(symbols, options);
    write_words(mem, DISPATCHER, &d);
    let mut empty = [0u8; 16];
    empty[..8].copy_from_slice(&EMPTY_TARGET.to_le_bytes());
    empty[8..].copy_from_slice(&miss_va.to_le_bytes());
    for i in 0..DISPATCH_ENTRIES {
        let at = (DISPATCH_TABLE + 16 * i) as usize;
        mem[at..at + 16].copy_from_slice(&empty);
    }
    let dispatch_va = MONITOR_VA + DISPATCHER;
    let (e, exception_call_va) = exception_entry(symbols, dispatch_va, timed);
    write_words(mem, EXCEPTION_ENTRY, &e);
    MonitorCode {
        dispatch_va,
        miss_va,
        exception_call_va,
    }
}

/// Puts `(target, code)` in a target's pair: its first slot, or its second (vm_backend's two-way table).
pub fn insert(mem: &mut [u8], target: u64, code: u64, second: bool) {
    let at = (DISPATCH_TABLE + 16 * (dispatch_slot(target) + u64::from(second))) as usize;
    mem[at..at + 8].copy_from_slice(&target.to_le_bytes());
    mem[at + 8..at + 16].copy_from_slice(&code.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::{MONITOR_VA, Options, Timing, VM_COMPILER, VMC_COMPILE_TICKS, dispatcher};
    use a64_encoder as a64;
    use std::collections::BTreeMap;

    /// The figures at a stop (Sprint 248): the frame still open is folded into the most, whichever holds more.
    #[test]
    fn the_open_frame_is_folded_into_the_most() {
        let slots = |open: u64| {
            move |slot: u64| match slot {
                super::VMC_COMPILE_TICKS => 900,
                super::VMC_COMPILE_LONGEST => 30,
                super::VMC_FRAME_TICKS => open,
                super::VMC_FRAME_MOST => 200,
                _ => panic!("slot {slot:x}"),
            }
        };
        let ticks = |longest, most_in_a_frame| super::CompileTicks {
            total: 900,
            longest,
            most_in_a_frame,
        };
        assert_eq!(super::CompileTicks::read(slots(150)), ticks(30, 200));
        assert_eq!(super::CompileTicks::read(slots(250)), ticks(30, 250));
    }

    /// The compile probe (Sprint 247; reviewer-3's Low 1 of its review of the rework): the default dispatcher reads no
    /// counter, so it is the words the gates' runs judge against the frozen host; the timed one adds the probe alone,
    /// and the miss stub moves by it. Sprint 248's longest and frame words are counted part by part.
    #[test]
    fn the_compile_probe_is_off_by_default_and_its_size_known() {
        let symbols: BTreeMap<String, u64> = [("w2m_vm_compile".to_string(), 0x7f00_0418_0000)].into();
        let (plain, plain_miss) = dispatcher(&symbols, Options::default());
        let (timed, timed_miss) = dispatcher(
            &symbols,
            Options {
                timing: Timing {
                    compile: true,
                    ..Timing::default()
                },
                ..Options::default()
            },
        );
        for read in [a64::mrs(26, a64::CNTVCT_EL0), a64::mrs(12, a64::CNTFRQ_EL0)] {
            assert!(!plain.contains(&read), "the default dispatcher reads {read:08x}");
            assert!(timed.contains(&read), "the timed dispatcher does not read {read:08x}");
        }
        let address = a64::mov_imm(16, MONITOR_VA + VM_COMPILER + VMC_COMPILE_TICKS).len();
        let frames = a64::mov_imm(13, super::FRAMES_A_SECOND).len();
        let before_call = 2; // isb, mrs
        let sum = 3 + address + 3; // isb, mrs, sub; the address; ldr, add, str
        let longest = 2 + 1 + 1; // ldr, cmp; b.ls; str
        let past_the_frame = 3 + frames + 2 + 1; // ldr, sub, mrs; 60; mul, cmp; b.lo
        let fold = 3 + 1 + 1 + 2; // ldr, ldr, cmp; b.ls; str; str, str
        let frame_sum = 3; // ldr, add, str
        let probe = before_call + sum + longest + past_the_frame + fold + frame_sum;
        assert_eq!(timed.len() - plain.len(), probe, "the probe's words");
        assert_eq!(
            timed_miss - plain_miss,
            4 * probe as u64,
            "the miss stub moves by the probe"
        );
    }

    /// The host range (Sprint 257): absent, the dispatcher is the Python's; present, the miss stub compares the target
    /// with both ends and branches to the host path before the in-VM compiler; the host path moves by the test.
    #[test]
    fn the_host_range_is_off_by_default_and_sent_before_the_compiler() {
        let symbols: BTreeMap<String, u64> = [("w2m_vm_compile".to_string(), 0x7f00_0418_0000)].into();
        let (plain, plain_miss) = dispatcher(&symbols, Options::default());
        let range = (0x1_8000_0000, 0x1_819d_b000);
        let (ranged, ranged_miss) = dispatcher(
            &symbols,
            Options {
                host_range: Some(range),
                ..Options::default()
            },
        );
        let test = a64::mov_imm(16, range.0).len() + 2 + a64::mov_imm(16, range.1).len() + 2; // cmp, b.lo; cmp, b.lo
        assert_eq!(ranged.len() - plain.len(), test, "the range test's words");
        assert_eq!(
            ranged_miss - plain_miss,
            4 * test as u64,
            "the host path moves by the test"
        );
        assert!(
            !plain.contains(&a64::cmp(17, 16)),
            "the default dispatcher compares x17 with x16"
        );
    }
}
