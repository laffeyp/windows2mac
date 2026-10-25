// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The monitor code the Python host writes at a VM's start, written from Rust (Sprint 136): the vectors, the
//! dispatcher, the empty dispatch table and the exception entry (`src/w2m/vm_backend.py` `_write_monitor`,
//! 4207-4267, and `_write_exception_entry`, 4343-4400), with the runtime lock and budget helpers they use (521-557).
//! Each function carries the lines it names, word for word in the Python's order.

use std::collections::BTreeMap;

use a64_encoder as a64;
use guest_runtime::layout::{
    DISPATCH_BITS, DISPATCH_ENTRIES, DISPATCH_TABLE, LOCK_OWNER, MONITOR_DATA, MONITOR_VA, PROCESS_VA, RUN_TABLE,
    RUNTIME_STACK, RUNTIME_STACK_SIZE, SAVE_AREA, STATE_OFF, VM_COMPILER,
};

use crate::assembler::{Assembler, Target};
use crate::{CPSR_EL1H, DISPATCHER, VECTORS};

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

/// The dispatcher (vm_backend 4217-4258): its words and the miss stub's address.
pub fn dispatcher(symbols: &BTreeMap<String, u64>) -> (Vec<u32>, u64) {
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
    d.emit(&a64::mov_imm(16, MONITOR_VA + VM_COMPILER));
    d.emit(&[a64::ldr(8, 16, 16, VMC_ENABLED as u32)]);
    d.branch(|off| a64::cbz(16, off), host);
    lock_runtime(&mut d);
    d.emit(&a64::mov_imm(16, MONITOR_VA + VM_COMPILER + VMC_SAVE));
    for r in (0..16).step_by(2) {
        d.emit(&[a64::stp(r, r + 1, 16, 8 * i64::from(r))]);
    }
    d.emit(&[a64::stp(17, 30, 16, 128)]);
    d.emit(&a64::mov_imm(16, MONITOR_VA + RUNTIME_STACK + RUNTIME_STACK_SIZE));
    d.emit(&[a64::addi(31, 16, 0), a64::mov(0, 17)]);
    d.emit(&a64::mov_imm(16, symbols["w2m_vm_compile"]));
    d.emit(&[a64::blr(16), a64::mov(25, 0)]);
    d.emit(&a64::mov_imm(16, MONITOR_VA + VM_COMPILER + VMC_SAVE));
    for r in (0..16).step_by(2) {
        d.emit(&[a64::ldp(r, r + 1, 16, 8 * i64::from(r))]);
    }
    d.emit(&[a64::ldp(17, 30, 16, 128)]);
    unlock_runtime(&mut d);
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
    (d.finish(), miss_va)
}

/// The exception entry (vm_backend 4343-4400): its words and the call stub's address (`exception_call_va`).
/// `resume_va` is where it goes on after the runtime dispatched the exception: the dispatcher.
pub fn exception_entry(symbols: &BTreeMap<String, u64>, resume_va: u64) -> (Vec<u32>, u64) {
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
    lock_runtime(&mut e);
    e.emit(&a64::mov_imm(16, MONITOR_VA + RUNTIME_STACK + RUNTIME_STACK_SIZE));
    e.emit(&[a64::addi(31, 16, 0)]);
    e.emit(&a64::mov_imm(16, symbols["w2m_exception"]));
    e.emit(&[a64::blr(16)]);
    e.emit(&budget_from_count());
    unlock_runtime(&mut e);
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
/// exception entry.
pub fn write(mem: &mut [u8], symbols: &BTreeMap<String, u64>) -> MonitorCode {
    let header: Vec<u8> = [0u64, 0, MONITOR_VA + DISPATCH_TABLE]
        .iter()
        .flat_map(|q| q.to_le_bytes())
        .collect();
    mem[MONITOR_DATA as usize..MONITOR_DATA as usize + 24].copy_from_slice(&header);
    write_words(mem, VECTORS, &vectors());
    let (d, miss_va) = dispatcher(symbols);
    write_words(mem, DISPATCHER, &d);
    let mut empty = [0u8; 16];
    empty[..8].copy_from_slice(&EMPTY_TARGET.to_le_bytes());
    empty[8..].copy_from_slice(&miss_va.to_le_bytes());
    for i in 0..DISPATCH_ENTRIES {
        let at = (DISPATCH_TABLE + 16 * i) as usize;
        mem[at..at + 16].copy_from_slice(&empty);
    }
    let dispatch_va = MONITOR_VA + DISPATCHER;
    let (e, exception_call_va) = exception_entry(symbols, dispatch_va);
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
