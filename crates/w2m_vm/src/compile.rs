// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The host's block compile (Sprint 155): a block the VM's C compiler declines, translated by the Rust translator,
//! carried from `src/w2m/vm_backend.py` `_compile` (4588-4642, without the translation cache and the block census),
//! `_flags_live_out` and `_flags_live_in` (4778-4825), `_protect_code` (4535-4551; `_page_descriptor`, 4507-4533, is
//! the guest's since Sprint 156), `_page_set_add` and `_page_set_remove` (2527-2552) and `_code_written` (4554-4577). One vCPU: no private tables, so
//! a descriptor written is written once (`_write_descriptor`'s copies are a several-vCPU matter).

use std::collections::{BTreeMap, BTreeSet};

use guest_runtime::layout::{
    AP_READ_ONLY, CODE_CACHE, CODE_CACHE_SIZE, CODE_PAGE_BITS, CODE_PAGE_SET, CODE_PAGE_SLOTS, DISPATCH_TABLE,
    IPA_BASE, MONITOR_VA, PAGE, PROCESS_VA, RECON_COUNT, RECON_ENTRIES, RECON_TABLE, RUN_TABLE, STATE_OFF, VM_COMPILER,
};
use translator::lift::{LiftError, Lifted, Op, lift};
use translator::lower::{ALL_FLAGS, assemble_block, insn_flags, nzcv_setter};

use crate::guest::{Guest, VMC_CURSOR};
use crate::monitor_code::{EMPTY_TARGET, dispatch_slot};
use crate::run::Tails;

/// vm_backend 372: a block's instructions at most.
pub const MAX_BLOCK_INSNS: usize = 64;
/// vm_backend 861: the instructions a successor is scanned for, at most.
pub const LIVENESS_SCAN: usize = 8;

fn read_u64(mem: &[u8], off: u64) -> u64 {
    u64::from_le_bytes(mem[off as usize..off as usize + 8].try_into().unwrap())
}

fn write_u64(mem: &mut [u8], off: u64, v: u64) {
    mem[off as usize..off as usize + 8].copy_from_slice(&v.to_le_bytes());
}

/// The host's compiles and the state they keep (vm_backend 2425, 2444-2467): the blocks, the pages behind them, the
/// tails. The split tables' next free page is the guest's (`Guest::page_descriptor`, Sprint 156).
#[derive(Default)]
pub struct HostCompiler {
    /// x86 start -> code VA
    pub blocks: BTreeMap<u64, u64>,
    /// a page behind translated code -> the blocks over it
    pub code_pages: BTreeMap<u64, BTreeSet<u64>>,
    pub tails: Tails,
    /// (start, end) of each compile, in order (`translations`)
    pub translations: Vec<(u64, u64)>,
    pub blocks_dropped: u64,
    page_set_removed: u64,
}

/// Up to 15 bytes of guest code at `va`, as far as its mapping reaches (`Memory.code_at`): None when unmapped.
fn code_at(guest: &Guest, va: u64) -> Option<Vec<u8>> {
    let (v, ipa, size) = guest.layout.maps.iter().find(|(v, _, n)| (*v..v + n).contains(&va))?;
    let off = (ipa - IPA_BASE + (va - v)) as usize;
    let n = 15.min((v + size - va) as usize);
    Some(guest.memory.as_slice()[off..off + n].to_vec())
}

/// `_lift`: the instruction at `addr`.
pub(crate) fn lift_at(guest: &Guest, addr: u64) -> Result<Lifted, LiftError> {
    let code = code_at(guest, addr).ok_or_else(|| LiftError(format!("no guest memory at 0x{addr:x}")))?;
    lift(&code, addr)
}

/// `_page_set_slot` (2524-2525).
fn page_set_slot(page: u64) -> u64 {
    (page >> 12).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> (64 - CODE_PAGE_BITS)
}

impl HostCompiler {
    /// `_compile` (4588-4642), not in lockstep and with no translation cache: the block at `start` compiled at the
    /// code cursor, its marks, tails and pages noted. Err names why it cannot be.
    pub fn compile(&mut self, guest: &mut Guest, start: u64) -> Result<u64, String> {
        if start == guest.trace_from && guest.trace_from != 0 {
            // the trace opens (vm_backend 4589-4590; Sprint 191): the VM's compiler left this block to the host
            let at = (guest_runtime::layout::MONITOR_SIZE + guest_runtime::layout::TRACE_OFF) as usize;
            guest.memory.as_mut_slice()[at..at + 8].copy_from_slice(&1u64.to_le_bytes());
        }
        let mut insns: Vec<Lifted> = Vec::new();
        let mut addr = start;
        while insns.len() < MAX_BLOCK_INSNS {
            if !insns.is_empty() && guest.thunks.contains_key(&addr) {
                break;
            }
            let ins = match lift_at(guest, addr) {
                Ok(ins) => ins,
                Err(e) if insns.is_empty() => return Err(e.0),
                Err(_) => break,
            };
            addr += ins.length;
            let ends = ins.ops.iter().any(|op| matches!(op, Op::Br { .. }));
            insns.push(ins);
            if ends {
                break;
            }
        }
        let exit_va = guest.code.dispatch_va;
        self.protect_code(guest, start, addr, None)?; // before the successor scan, which reads the pages it covers
        let live = self.flags_live_out(guest, start, insns.last().unwrap(), addr)?;
        let mem = guest.memory.as_mut_slice();
        let cursor = read_u64(mem, VM_COMPILER + VMC_CURSOR);
        let base = MONITOR_VA + cursor;
        // the runtime's functions the words call (`_compile` passes the runtime's symbols, 4618-4619)
        if !guest.symbols.contains_key("w2m_cpuid") {
            return Err("the guest runtime defines no w2m_cpuid".into());
        }
        let runtime = translator::lower::RuntimeCalls::from_symbols(|s| guest.symbols.get(s).copied());
        let (words, marks) = assemble_block(
            &insns,
            base,
            exit_va,
            guest.slice_va,
            PROCESS_VA + STATE_OFF,
            &runtime,
            live,
        )
        .map_err(|e| e.to_string())?;
        if cursor + 4 * words.len() as u64 > CODE_CACHE + CODE_CACHE_SIZE {
            return Err("code cache full".into());
        }
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        mem[cursor as usize..cursor as usize + bytes.len()].copy_from_slice(&bytes);
        write_u64(mem, VM_COMPILER + VMC_CURSOR, cursor + bytes.len() as u64);
        // `_recon_append` (4850-4859)
        let n = read_u64(mem, RUN_TABLE + RECON_COUNT);
        if n + marks.len() as u64 > RECON_ENTRIES {
            return Err("reconstruction table full".into());
        }
        for (i, (host, x86)) in marks.iter().enumerate() {
            let at = RECON_TABLE + 16 * (n + i as u64);
            write_u64(mem, at, *host);
            write_u64(mem, at + 8, *x86);
        }
        write_u64(mem, RUN_TABLE + RECON_COUNT, n + marks.len() as u64);
        let hosts: Vec<u64> = marks[1..1 + insns.len()].iter().map(|(h, _)| *h).collect();
        self.tails.note(&hosts);
        self.blocks.insert(start, base);
        self.translations.push((start, addr));
        Ok(base)
    }

    /// `_flags_live_out` (4778-4788): the flags a block read after it ends, from its direct successors; a successor
    /// not known when compiling reads all.
    fn flags_live_out(&mut self, guest: &Guest, start: u64, last: &Lifted, end: u64) -> Result<u32, String> {
        let constant: BTreeMap<u32, u64> = last
            .ops
            .iter()
            .filter_map(|op| match op {
                Op::Imm { t, value } => Some((*t, *value)),
                _ => None,
            })
            .collect();
        let Some((cond, t)) = last.ops.iter().find_map(|op| match op {
            Op::Br { cond, t } => Some((*cond, *t)),
            _ => None,
        }) else {
            return self.flags_live_in(guest, start, end);
        };
        let Some(target) = constant.get(&t) else {
            return Ok(ALL_FLAGS);
        };
        let taken = self.flags_live_in(guest, start, *target)?;
        Ok(taken
            | if cond == "always" {
                0
            } else {
                self.flags_live_in(guest, start, end)?
            })
    }

    /// `_flags_live_in` (4790-4825): the flags code at `addr` reads before writing them, from at most LIVENESS_SCAN
    /// instructions on its straight path; only pages already behind translated code are read, and `owner` joins each
    /// such page's blocks.
    fn flags_live_in(&mut self, guest: &Guest, owner: u64, mut addr: u64) -> Result<u32, String> {
        let (mut live, mut undecided) = (0u32, ALL_FLAGS);
        let mut previous: Option<Lifted> = None;
        for _ in 0..LIVENESS_SCAN {
            if guest.thunks.contains_key(&addr)
                || addr >= PROCESS_VA
                || !self.code_pages.contains_key(&(addr & !(PAGE - 1)))
            {
                break;
            }
            let Ok(ins) = lift_at(guest, addr) else {
                break;
            };
            let pages = [addr & !(PAGE - 1), (addr + ins.length - 1) & !(PAGE - 1)];
            if !pages.iter().all(|p| self.code_pages.contains_key(p)) {
                break;
            }
            for p in pages {
                self.code_pages.get_mut(&p).unwrap().insert(owner);
            }
            // a condition the lowering does not name ends the compile by name, as the Python's KeyError leaves
            // `_flags_live_in` (4812), `_compile` and `serve_exit` (5242) uncaught and ends the run (Sprint 171)
            let (read, written) = insn_flags(&ins, previous.as_ref().and_then(nzcv_setter))
                .map_err(|e| format!("0x{addr:x}, scanning the flags block 0x{owner:x} leaves live: {e}"))?;
            live |= read & undecided;
            undecided &= !(read | written);
            if undecided == 0 {
                return Ok(live);
            }
            let br = ins.ops.iter().find_map(|op| match op {
                Op::Br { cond, t } => Some((*cond, *t)),
                _ => None,
            });
            let Some((cond, t)) = br else {
                addr += ins.length;
                previous = Some(ins);
                continue;
            };
            let target = ins.ops.iter().find_map(|op| match op {
                Op::Imm { t: it, value } if *it == t => Some(*value),
                _ => None,
            });
            match (cond, target) {
                ("always", Some(target)) => {
                    addr = target;
                    previous = None;
                }
                _ => break,
            }
        }
        Ok(live | undecided)
    }

    /// `_protect_code` (4535-4551): the pages under guest bytes [start, end) read-only, and the block over them noted
    /// (`owner`, when the bytes are not its own). The thunks and the process area stay as they are.
    fn protect_code(&mut self, guest: &mut Guest, start: u64, end: u64, owner: Option<u64>) -> Result<(), String> {
        let owner = owner.unwrap_or(start);
        if start >= PROCESS_VA {
            return Ok(());
        }
        let mut page = start & !(PAGE - 1);
        while page < end {
            if !self.code_pages.contains_key(&page) {
                let at = guest.page_descriptor(page)?;
                if at == 0 {
                    page += PAGE;
                    continue;
                }
                let mem = guest.memory.as_mut_slice();
                let desc = read_u64(mem, at);
                write_u64(mem, at, desc | AP_READ_ONLY);
                self.page_set_add(guest, page);
            }
            self.code_pages.entry(page).or_default().insert(owner);
            page += PAGE;
        }
        Ok(())
    }

    /// `_page_set_add` (2527-2539): the page in the VM's open-addressed code page set (1 marks a removed slot).
    fn page_set_add(&mut self, guest: &mut Guest, page: u64) {
        let mem = guest.memory.as_mut_slice();
        let (mut i, mut free) = (page_set_slot(page), None);
        loop {
            let v = read_u64(mem, CODE_PAGE_SET + 8 * i);
            if v == page {
                return;
            }
            if v == 1 && free.is_none() {
                free = Some(i);
            }
            if v == 0 {
                break;
            }
            i = (i + 1) & (CODE_PAGE_SLOTS - 1);
        }
        if free.is_some() {
            self.page_set_removed -= 1;
        }
        write_u64(mem, CODE_PAGE_SET + 8 * free.unwrap_or(i), page);
    }

    /// `_page_set_remove` (2541-2556).
    fn page_set_remove(&mut self, guest: &mut Guest, page: u64) {
        let mem = guest.memory.as_mut_slice();
        let mut i = page_set_slot(page);
        loop {
            let v = read_u64(mem, CODE_PAGE_SET + 8 * i);
            if v == 0 {
                return;
            }
            if v == page {
                write_u64(mem, CODE_PAGE_SET + 8 * i, 1);
                self.page_set_removed += 1;
                break;
            }
            i = (i + 1) & (CODE_PAGE_SLOTS - 1);
        }
        if self.page_set_removed > CODE_PAGE_SLOTS / 8 {
            // probes grow with removed slots: rebuild
            mem[CODE_PAGE_SET as usize..(CODE_PAGE_SET + 8 * CODE_PAGE_SLOTS) as usize].fill(0);
            self.page_set_removed = 0;
            let pages: Vec<u64> = self.code_pages.keys().copied().collect();
            for p in pages {
                self.page_set_add(guest, p);
            }
        }
    }

    /// `_code_written` (4554-4577): a write faulted at `address`; when its page is behind translated code, the blocks
    /// over the page are dropped with their dispatch slots and the page made writable again. True when it was.
    pub fn code_written(&mut self, guest: &mut Guest, address: u64) -> Result<bool, String> {
        let page = address & !(PAGE - 1);
        let Some(blocks) = self.code_pages.remove(&page) else {
            return Ok(false);
        };
        self.page_set_remove(guest, page);
        let miss_va = guest.code.miss_va;
        for start in blocks {
            if self.blocks.remove(&start).is_none() {
                continue;
            }
            let mem = guest.memory.as_mut_slice();
            let slot = DISPATCH_TABLE + 16 * dispatch_slot(start);
            for s in [slot, slot + 16] {
                if read_u64(mem, s) == start {
                    write_u64(mem, s, EMPTY_TARGET);
                    write_u64(mem, s + 8, miss_va);
                }
            }
            self.blocks_dropped += 1;
        }
        let at = guest.page_descriptor(page)?;
        let mem = guest.memory.as_mut_slice();
        let desc = read_u64(mem, at);
        write_u64(mem, at, desc & !AP_READ_ONLY);
        Ok(true)
    }
}
