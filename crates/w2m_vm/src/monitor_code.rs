// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The monitor code the Python host writes at a VM's start, written from Rust (Sprint 136): the vectors, the
//! dispatcher, the empty dispatch table and the exception entry (`src/w2m/vm_backend.py` `_write_monitor`,
//! 4207-4267, and `_write_exception_entry`, 4343-4400), with the runtime lock and budget helpers they use (521-557).
//! Each function carries the lines it names, word for word in the Python's order.

use std::collections::BTreeMap;

use a64_encoder as a64;
use guest_runtime::layout::{
    CACHE_CODE_VA, CODE_CACHE, CODE_CACHE_SIZE, DISPATCH_BITS, DISPATCH_ENTRIES, DISPATCH_TABLE, LOCK_OWNER, MAX_VCPUS,
    MONITOR_DATA, MONITOR_SIZE, MONITOR_VA, PAGE, PROCESS_VA, RECON_COUNT, RECON_ENTRIES, RECON_TABLE, RUN_TABLE,
    RUNTIME_STACK, RUNTIME_STACK_SIZE, SAVE_AREA, STATE_OFF, VCPU_AREA, VCPU_STRIDE, VM_COMPILER, VMC_LOG,
    VMC_LOG_ENTRIES, VMC_LOG_ENTRY,
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
/// VM_COMPILER + these, the C compiler's own words (`block_compiler.c` 18): the code cursor, an offset from
/// `MONITOR_VA`, and the compile log's count. Here, not in `guest` and `run`, since the installer reads them and every
/// target builds the monitor code (Sprint 263).
pub const VMC_CURSOR: u64 = 0x08;
pub const VMC_LOG_COUNT: u64 = 0x10;
/// The block the C compiler leaves to the host (`block_compiler.c` 223), the call trace's first, which the host's
/// compile opens the trace at (`guest.rs`, `VMC_EXIT + 16`); the installer leaves it too (Sprint 263).
pub const VMC_HOST_ONLY: u64 = 0x28;
/// VM_COMPILER + these (Sprint 263; card 5's III and VI), past `VMC_SAVE`'s 0x90 bytes: the installer's stop word,
/// nonzero once a write to the cached image's code stopped the cache; the installer's address, 0 when the run has
/// none; and, written by the host when it places the cache file, the image's base, then the guest addresses of the
/// file's block table, its block count, and its words, marks and fix-ups.
pub const VMC_INSTALL_STOP: u64 = 0x300;
pub const VMC_INSTALLER: u64 = 0x308;
pub const VMC_CACHE_BASE: u64 = 0x310;
pub const VMC_CACHE_BLOCKS: u64 = 0x318;
pub const VMC_CACHE_NBLOCKS: u64 = 0x320;
pub const VMC_CACHE_WORDS: u64 = 0x328;
pub const VMC_CACHE_MARKS: u64 = 0x330;
pub const VMC_CACHE_FIXUPS: u64 = 0x338;
/// A cached block's entry in the file (`cache_file::to_bytes`): ten u32s, start, end, word, nwords, mark, nmarks, live,
/// insns, fixup, nfixups, at these byte offsets.
const BLOCK_ENTRY: u64 = crate::cache_file::BLOCK_BYTES;
// the installer finds a log entry by a shift
const _: () = assert!(VMC_LOG_ENTRY.is_power_of_two());

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

/// The owner the host writes in the runtime word while it holds it (Sprint 275): no vCPU's `LOCK_OWNER`.
pub const HOST_OWNER: u32 = u32::MAX;

/// The runtime word in the guest's memory `mem` (offset `RUN_TABLE + RUNTIME_LOCK`), as an atomic for one access: the
/// guest takes it by `CASA` and releases it by `STLR` on the same physical word, so a host atomic is the guest's word.
fn runtime_word(mem: &mut [u8]) -> &std::sync::atomic::AtomicU32 {
    let off = (RUN_TABLE + RUNTIME_LOCK) as usize;
    let p = mem[off..off + 4].as_mut_ptr().cast::<u32>();
    // SAFETY: the word is 4-byte aligned (`RUNTIME_LOCK` 0x80 in the page-aligned run table) and lives as long as `mem`;
    // every access to it, the guest's and the host's, is atomic
    unsafe { std::sync::atomic::AtomicU32::from_ptr(p) }
}

/// The host takes the runtime word (Sprint 275, census D6), waiting while a vCPU holds it, unless the vCPU whose exit
/// the host serves holds it already (`own`, its `LOCK_OWNER`: the frozen host holds the word across a host request):
/// whether the host took it, and so must release it.
pub fn host_take_runtime_word(mem: &mut [u8], own: u32) -> bool {
    use std::sync::atomic::Ordering;
    loop {
        match runtime_word(mem).compare_exchange(0, HOST_OWNER, Ordering::Acquire, Ordering::Relaxed) {
            Ok(_) => return true,
            Err(holder) if holder == own => return false,
            Err(_) => std::hint::spin_loop(),
        }
    }
}

/// The host releases the runtime word it took.
pub fn host_release_runtime_word(mem: &mut [u8]) {
    runtime_word(mem).store(0, std::sync::atomic::Ordering::Release);
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
    /// the instructions retired at each call of the in-VM compiler or the installer that logged a block, written into
    /// that log entry's `VMC_LOG_EXECUTED` word (Sprint 266): the guest's own clock, which the host's load does not move
    pub instructions_at_compile: bool,
}

/// A log entry's last word, past `struct log_entry`'s 120 bytes (`block_compiler.c` 31-42), which the C never writes:
/// the instructions retired at the call that logged it, when `Timing::instructions_at_compile` asks; 0 otherwise. The
/// C's text stays the frozen host's, whose image `python_image.rs` judges byte for byte, so the word has no C name.
pub const VMC_LOG_EXECUTED: u64 = 120;
const _: () = assert!(VMC_LOG_EXECUTED + 8 == VMC_LOG_ENTRY);

/// How the monitor code is written: what it times, and the targets its miss stub sends to the host before the in-VM
/// compiler (Sprint 257: the translation cache's image, `[base, base + size)`, which the host installs from the cache).
/// The default writes the Python's words.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    pub timing: Timing,
    pub host_range: Option<(u64, u64)>,
    /// with a host range, the range's misses go to the installer inside the VM (Sprint 263), the host's `hvc 1` only
    /// for what it declines
    pub install_in_vm: bool,
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

/// The miss stub's call of a compiler or the installer, the callee's address put in x16 by `load`: the runtime word
/// taken, x0 to x15, x17 and x30 saved at `VMC_SAVE`, the runtime stack, the target in x0, the call timed when
/// `timing.compile` asks, the registers restored, the word released, then the code in x0 run or the target sent to
/// `host`.
fn call_path(d: &mut Assembler, timing: Timing, load: &dyn Fn(&mut Assembler), host: Target) {
    let timed = timing.runtime_word;
    take_runtime(d, timed);
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
    load(d);
    d.emit(&[a64::blr(16), a64::mov(25, 0)]);
    if timing.compile {
        compile_probe(d);
    }
    if timing.instructions_at_compile {
        // a nonzero x0 logged one entry, the last (the C moves `VMC_LOG_COUNT` before its return, the installer before
        // its dispatch pair): into its last word the limit at x28 + 8 less the budget left in x23, the count
        // `count_from_budget` writes, both registers kept by the call (AAPCS64); x16, x18 and x26 free, as above
        let logged = d.label();
        d.branch(|off| a64::cbz(25, off), logged);
        d.emit(&a64::mov_imm(16, MONITOR_VA + VM_COMPILER));
        d.emit(&[a64::ldr(8, 18, 16, VMC_LOG_COUNT as u32)]);
        d.emit(&a64::mov_imm(
            16,
            MONITOR_VA + VM_COMPILER + VMC_LOG + VMC_LOG_EXECUTED - VMC_LOG_ENTRY,
        ));
        d.emit(&[
            a64::add(16, 16, 18, a64::LSL, VMC_LOG_ENTRY.trailing_zeros()),
            a64::ldr(8, 26, 28, 8),
            a64::sub(26, 26, REG_BUDGET),
            a64::str(8, 26, 16, 0),
        ]);
        d.bind(logged);
    }
    d.emit(&a64::mov_imm(16, MONITOR_VA + VM_COMPILER + VMC_SAVE));
    for r in (0..16).step_by(2) {
        d.emit(&[a64::ldp(r, r + 1, 16, 8 * i64::from(r))]);
    }
    d.emit(&[a64::ldp(17, 30, 16, 128)]);
    // x25 holds the compiled code; x16, x18 and x26 are free (x18 and x26 are temporaries, dead between blocks)
    release_runtime(d, timed, Site::Compile, (16, 18, 26));
    d.branch(|off| a64::cbz(25, off), host);
    d.emit(&[a64::br(25)]);
}

/// The installer (Sprint 263; card 5's III to VI), called by the miss stub's `call_path` with the target in x0: x0
/// the installed block's code, or 0 to send the target to `hvc 1`. It reads the cache file the host placed at
/// `CACHE_FILE_VA` through the `VMC_CACHE_*` words, and declines when a write stopped the cache, the target is not a
/// cached start, the code area, the reconstruction table or the log is full, or a branch's move would leave its reach.
/// It copies the block's words to the code cursor, patches its fix-ups by the move's distance, appends its marks rebased
/// to the cursor, makes the words visible by range as `block_compiler.c` 166-176 does, logs it with `pad` 1 for the
/// host's replay, moves the counts last as the C does, and writes the dispatch pair as `dispatch_insert` does
/// (`block_compiler.c` 146-162). It uses x0 to x16 and x30, which `call_path` saves, and no stack.
pub fn installer(a: &mut Assembler) {
    let decline = a.label();
    // x15 the VM compiler's words; x2 the image's base; x3 the target's RVA, under 4 GiB
    a.emit(&a64::mov_imm(15, MONITOR_VA + VM_COMPILER));
    a.emit(&[a64::ldr(8, 1, 15, VMC_INSTALL_STOP as u32)]);
    a.branch(|o| a64::cbnz(1, o), decline);
    // the block the host opens the call trace at, left to the host as the C leaves it
    a.emit(&[a64::ldr(8, 1, 15, VMC_HOST_ONLY as u32), a64::cmp(1, 0)]);
    a.branch(|o| a64::bcond("eq", o), decline);
    a.emit(&[a64::ldr(8, 2, 15, VMC_CACHE_BASE as u32), a64::subs(3, 0, 2)]);
    a.branch(|o| a64::bcond("lo", o), decline);
    a.emit(&[a64::lsri(1, 3, 32)]);
    a.branch(|o| a64::cbnz(1, o), decline);
    // the first block whose start is not below the RVA, by bisection: x4 the table, x5 its count, x6 lo, x7 hi
    let (search, below, found) = (a.label(), a.label(), a.label());
    a.emit(&[
        a64::ldr(8, 4, 15, VMC_CACHE_BLOCKS as u32),
        a64::ldr(8, 5, 15, VMC_CACHE_NBLOCKS as u32),
        a64::mov(7, 5),
    ]);
    a.emit(&a64::mov_imm(6, 0));
    a.emit(&a64::mov_imm(9, BLOCK_ENTRY));
    a.bind(search);
    a.emit(&[a64::cmp(6, 7)]);
    a.branch(|o| a64::bcond("hs", o), found);
    a.emit(&[
        a64::add(8, 6, 7, a64::LSL, 0),
        a64::lsri(8, 8, 1),
        a64::mul(10, 8, 9),
        a64::ldr_reg(4, 11, 4, 10, false),
        a64::cmp(11, 3),
    ]);
    a.branch(|o| a64::bcond("hs", o), below);
    a.emit(&[a64::addi(6, 8, 1)]);
    a.branch(a64::b, search);
    a.bind(below);
    a.emit(&[a64::mov(7, 8)]);
    a.branch(a64::b, search);
    a.bind(found);
    // x10 the block's entry, when it is in the table and starts at the RVA
    a.emit(&[a64::cmp(6, 5)]);
    a.branch(|o| a64::bcond("hs", o), decline);
    a.emit(&[
        a64::mul(10, 6, 9),
        a64::add(10, 4, 10, a64::LSL, 0),
        a64::ldr(4, 11, 10, 0),
        a64::cmp(11, 3),
    ]);
    a.branch(|o| a64::bcond("ne", o), decline);
    // room: x1 the cursor, x12 the block's first word, x13 its word count, x14 the code at the cursor
    a.emit(&[
        a64::ldr(8, 1, 15, VMC_CURSOR as u32),
        a64::ldr(4, 12, 10, 8),
        a64::ldr(4, 13, 10, 12),
        a64::add(16, 1, 13, a64::LSL, 2),
    ]);
    a.emit(&a64::mov_imm(9, CODE_CACHE + CODE_CACHE_SIZE));
    a.emit(&[a64::cmp(16, 9)]);
    a.branch(|o| a64::bcond("hi", o), decline);
    a.emit(&a64::mov_imm(14, MONITOR_VA));
    a.emit(&[a64::add(14, 14, 1, a64::LSL, 0)]);
    // x7 the run table, x8 the reconstruction table's count, x11 the block's marks; x6 the log's count
    a.emit(&a64::mov_imm(7, MONITOR_VA + RUN_TABLE));
    a.emit(&[
        a64::ldr(8, 8, 7, RECON_COUNT as u32),
        a64::ldr(4, 11, 10, 20),
        a64::add(16, 8, 11, a64::LSL, 0),
    ]);
    a.emit(&a64::mov_imm(9, RECON_ENTRIES));
    a.emit(&[a64::cmp(16, 9)]);
    a.branch(|o| a64::bcond("hi", o), decline);
    a.emit(&[a64::ldr(8, 6, 15, VMC_LOG_COUNT as u32)]);
    a.emit(&a64::mov_imm(9, VMC_LOG_ENTRIES));
    a.emit(&[a64::cmp(6, 9)]);
    a.branch(|o| a64::bcond("hs", o), decline);
    // the words copied: x4 the block's words in the file, x5 the index
    let (copy, copied) = (a.label(), a.label());
    a.emit(&[
        a64::ldr(8, 4, 15, VMC_CACHE_WORDS as u32),
        a64::add(4, 4, 12, a64::LSL, 2),
    ]);
    a.emit(&a64::mov_imm(5, 0));
    a.bind(copy);
    a.emit(&[a64::cmp(5, 13)]);
    a.branch(|o| a64::bcond("hs", o), copied);
    a.emit(&[
        a64::ldr_reg(4, 9, 4, 5, true),
        a64::str_reg(4, 9, 14, 5, true),
        a64::addi(5, 5, 1),
    ]);
    a.branch(a64::b, copy);
    a.bind(copied);
    // x12 the marks' rebase, code - 4 * word (the words were assembled at CACHE_CODE_VA + 4 * word); x9 the move in
    // words for a branch leaving the block, (CACHE_CODE_VA + 4 * word - code) / 4
    a.emit(&[a64::ubfm(12, 12, 62, 61), a64::sub(12, 14, 12)]);
    a.emit(&a64::mov_imm(9, CACHE_CODE_VA));
    a.emit(&[a64::sub(9, 9, 12), a64::sbfx(9, 9, 2, 62)]);
    // the fix-ups: x4 the block's list, x5 its count, x16 the index; x3 the word's index, x11 the word, x13 its
    // offset; a moved offset out of the 26 bits' reach declines
    let (fix, fixed) = (a.label(), a.label());
    a.emit(&[
        a64::ldr(8, 4, 15, VMC_CACHE_FIXUPS as u32),
        a64::ldr(4, 5, 10, 32),
        a64::add(4, 4, 5, a64::LSL, 2),
        a64::ldr(4, 5, 10, 36),
    ]);
    a.emit(&a64::mov_imm(16, 0));
    a.bind(fix);
    a.emit(&[a64::cmp(16, 5)]);
    a.branch(|o| a64::bcond("hs", o), fixed);
    a.emit(&[
        a64::ldr_reg(4, 3, 4, 16, true),
        a64::ldr_reg(4, 11, 14, 3, true),
        a64::sbfx(13, 11, 0, 26),
        a64::add(13, 13, 9, a64::LSL, 0),
        a64::sbfx(2, 13, 0, 26),
        a64::cmp(2, 13),
    ]);
    a.branch(|o| a64::bcond("ne", o), decline);
    a.emit(&[
        a64::bfi(11, 13, 0, 26),
        a64::str_reg(4, 11, 14, 3, true),
        a64::addi(16, 16, 1),
    ]);
    a.branch(a64::b, fix);
    a.bind(fixed);
    // the marks appended at the reconstruction table's count: x4 the block's marks in the file, x5 their count, x3 the
    // table's entry; each (host offset, RVA) written as (host offset + x12, RVA + base)
    let (mark, marked) = (a.label(), a.label());
    a.emit(&[
        a64::ldr(8, 2, 15, VMC_CACHE_BASE as u32),
        a64::ldr(8, 4, 15, VMC_CACHE_MARKS as u32),
        a64::ldr(4, 5, 10, 16),
        a64::add(4, 4, 5, a64::LSL, 3),
        a64::ldr(4, 5, 10, 20),
    ]);
    a.emit(&a64::mov_imm(3, MONITOR_VA + RECON_TABLE));
    a.emit(&[a64::add(3, 3, 8, a64::LSL, 4)]);
    a.bind(mark);
    a.branch(|o| a64::cbz(5, o), marked);
    a.emit(&[
        a64::ldr(4, 11, 4, 0),
        a64::ldr(4, 13, 4, 4),
        a64::add(11, 11, 12, a64::LSL, 0),
        a64::add(13, 13, 2, a64::LSL, 0),
        a64::str(8, 11, 3, 0),
        a64::str(8, 13, 3, 8),
        a64::addi(4, 4, 8),
        a64::addi(3, 3, 16),
        a64::subi(5, 5, 1),
    ]);
    a.branch(a64::b, mark);
    a.bind(marked);
    // the words made visible to instruction fetch by range: x9 the data line, x16 the instruction line, x5 the end
    let (clean, cleaned, invalidate, invalidated) = (a.label(), a.label(), a.label(), a.label());
    a.emit(&[
        a64::mrs(16, a64::CTR_EL0),
        a64::ubfx(9, 16, 16, 4),
        a64::ubfx(16, 16, 0, 4),
    ]);
    a.emit(&a64::mov_imm(11, 4));
    a.emit(&[
        a64::lslv(9, 11, 9),
        a64::lslv(16, 11, 16),
        a64::ldr(4, 13, 10, 12),
        a64::add(5, 14, 13, a64::LSL, 2),
        a64::subi(11, 9, 1),
        a64::bic(4, 14, 11),
    ]);
    a.bind(clean);
    a.emit(&[a64::cmp(4, 5)]);
    a.branch(|o| a64::bcond("hs", o), cleaned);
    a.emit(&[a64::dc_cvau(4), a64::add(4, 4, 9, a64::LSL, 0)]);
    a.branch(a64::b, clean);
    a.bind(cleaned);
    a.emit(&[a64::DSB_ISH, a64::subi(11, 16, 1), a64::bic(4, 14, 11)]);
    a.bind(invalidate);
    a.emit(&[a64::cmp(4, 5)]);
    a.branch(|o| a64::bcond("hs", o), invalidated);
    a.emit(&[a64::ic_ivau(4), a64::add(4, 4, 16, a64::LSL, 0)]);
    a.branch(a64::b, invalidate);
    a.bind(invalidated);
    a.emit(&[a64::DSB_ISH, a64::ISB]);
    // the log entry (`block_compiler.c` 31-42): start, end, code, the marks' first index and count, the instructions,
    // no cut, the block's live flags as both, no scanned pages, `pad` 1 for an install
    a.emit(&a64::mov_imm(3, MONITOR_VA + VM_COMPILER + VMC_LOG));
    a.emit(&[
        a64::add(3, 3, 6, a64::LSL, VMC_LOG_ENTRY.trailing_zeros()),
        a64::str(8, 0, 3, 0),
        a64::ldr(4, 11, 10, 4),
        a64::add(11, 11, 2, a64::LSL, 0),
        a64::str(8, 11, 3, 8),
        a64::str(8, 14, 3, 16),
        a64::str(4, 8, 3, 24),
        a64::ldr(4, 11, 10, 20),
        a64::str(4, 11, 3, 28),
        a64::ldr(4, 11, 10, 28),
        a64::str(4, 11, 3, 32),
        a64::str(4, 31, 3, 36),
        a64::ldr(4, 11, 10, 24),
        a64::str(4, 11, 3, 40),
        a64::str(4, 11, 3, 44),
        a64::str(4, 31, 3, 48),
    ]);
    a.emit(&a64::mov_imm(11, 1));
    a.emit(&[a64::str(4, 11, 3, 52)]);
    // the counts last: the reconstruction table's, the cursor, the log's
    a.emit(&[
        a64::ldr(4, 11, 10, 20),
        a64::add(11, 8, 11, a64::LSL, 0),
        a64::str(8, 11, 7, RECON_COUNT as u32),
        a64::ldr(4, 13, 10, 12),
        a64::add(13, 1, 13, a64::LSL, 2),
        a64::str(8, 13, 15, VMC_CURSOR as u32),
        a64::addi(6, 6, 1),
        a64::str(8, 6, 15, VMC_LOG_COUNT as u32),
    ]);
    // the dispatch pair: the first slot if empty or the target's, else the second, else the first moved to the second
    let bits = DISPATCH_BITS as u32;
    let (first, second, inserted) = (a.label(), a.label(), a.label());
    a.emit(&[a64::eor(9, 0, 0, a64::LSR, bits), a64::eor(9, 9, 0, a64::LSR, 2 * bits)]);
    a.emit(&a64::mov_imm(11, (1 << DISPATCH_BITS) - 2));
    a.emit(&[a64::and(9, 9, 11)]);
    a.emit(&a64::mov_imm(3, MONITOR_VA + DISPATCH_TABLE));
    a.emit(&[a64::add(3, 3, 9, a64::LSL, 4)]);
    a.emit(&a64::mov_imm(11, EMPTY_TARGET));
    a.emit(&[a64::ldr(8, 4, 3, 0), a64::cmp(4, 11)]);
    a.branch(|o| a64::bcond("eq", o), first);
    a.emit(&[a64::cmp(4, 0)]);
    a.branch(|o| a64::bcond("eq", o), first);
    a.emit(&[a64::ldr(8, 4, 3, 16), a64::cmp(4, 11)]);
    a.branch(|o| a64::bcond("eq", o), second);
    a.emit(&[a64::cmp(4, 0)]);
    a.branch(|o| a64::bcond("eq", o), second);
    a.emit(&[a64::ldp(4, 5, 3, 0), a64::str(8, 5, 3, 24), a64::str(8, 4, 3, 16)]);
    a.bind(first);
    a.emit(&[a64::str(8, 14, 3, 8), a64::str(8, 0, 3, 0)]);
    a.branch(a64::b, inserted);
    a.bind(second);
    a.emit(&[a64::str(8, 14, 3, 24), a64::str(8, 0, 3, 16)]);
    a.bind(inserted);
    a.emit(&[a64::mov(0, 14), a64::ret()]);
    a.bind(decline);
    a.emit(&a64::mov_imm(0, 0));
    a.emit(&[a64::ret()]);
}

/// The dispatcher (vm_backend 4217-4258): its words and the miss stub's address, written as `options` asks.
pub fn dispatcher(symbols: &BTreeMap<String, u64>, options: Options) -> (Vec<u32>, u64) {
    let timing = options.timing;
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
    // a target in the host's range goes to the host first, which installs it from the translation cache (Sprint 257),
    // or with `install_in_vm` to the installer inside the VM (Sprint 263); x16 is free here
    let install = (options.host_range.is_some() && options.install_in_vm).then(|| d.label());
    if let Some((lo, hi)) = options.host_range {
        let outside = d.label();
        d.emit(&a64::mov_imm(16, lo));
        d.emit(&[a64::cmp(17, 16)]);
        d.branch(|off| a64::bcond("lo", off), outside);
        d.emit(&a64::mov_imm(16, hi));
        d.emit(&[a64::cmp(17, 16)]);
        d.branch(|off| a64::bcond("lo", off), install.unwrap_or(host));
        d.bind(outside);
    }
    d.emit(&a64::mov_imm(16, MONITOR_VA + VM_COMPILER));
    d.emit(&[a64::ldr(8, 16, 16, VMC_ENABLED as u32)]);
    d.branch(|off| a64::cbz(16, off), host);
    let compiler = symbols["w2m_vm_compile"];
    call_path(&mut d, timing, &|d| d.emit(&a64::mov_imm(16, compiler)), host);
    if let Some(install) = install {
        // the installer inside the VM (Sprint 263; card 5's III), its address at `VMC_INSTALLER`
        d.bind(install);
        call_path(
            &mut d,
            timing,
            &|d| {
                d.emit(&a64::mov_imm(16, MONITOR_VA + VM_COMPILER));
                d.emit(&[a64::ldr(8, 16, 16, VMC_INSTALLER as u32)]);
            },
            host,
        );
    }
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
    use super::{
        MONITOR_VA, Options, REG_BUDGET, Timing, VM_COMPILER, VMC_COMPILE_TICKS, VMC_LOG, VMC_LOG_ENTRY,
        VMC_LOG_EXECUTED, dispatcher,
    };
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

    /// The instructions at a compile (Sprint 266): off by default, so the default dispatcher is the Python's; on, the
    /// call path adds the store of the limit less the budget into the last log entry's last word, and nothing else.
    #[test]
    fn the_instructions_at_a_compile_are_off_by_default_and_written_from_the_budget() {
        let symbols: BTreeMap<String, u64> = [("w2m_vm_compile".to_string(), 0x7f00_0418_0000)].into();
        let (plain, plain_miss) = dispatcher(&symbols, Options::default());
        let (counted, counted_miss) = dispatcher(
            &symbols,
            Options {
                timing: Timing {
                    instructions_at_compile: true,
                    ..Timing::default()
                },
                ..Options::default()
            },
        );
        let from_budget = [
            a64::ldr(8, 26, 28, 8),
            a64::sub(26, 26, REG_BUDGET),
            a64::str(8, 26, 16, 0),
        ];
        assert!(
            !plain.windows(3).any(|w| w == from_budget),
            "the default dispatcher writes the count"
        );
        assert!(
            counted.windows(3).any(|w| w == from_budget),
            "the counted dispatcher does not write it"
        );
        let compiler = a64::mov_imm(16, MONITOR_VA + VM_COMPILER).len();
        let last_word = a64::mov_imm(
            16,
            MONITOR_VA + VM_COMPILER + VMC_LOG + VMC_LOG_EXECUTED - VMC_LOG_ENTRY,
        )
        .len();
        let store = 1 + compiler + 1 + last_word + 4; // cbz; the area; ldr; the word's address; add, ldr, sub, str
        assert_eq!(counted.len() - plain.len(), store, "the store's words");
        assert_eq!(
            counted_miss - plain_miss,
            4 * store as u64,
            "the miss stub moves by the store"
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
