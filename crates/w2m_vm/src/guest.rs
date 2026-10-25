// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! A program's guest memory and the code the host writes before it runs (Sprint 141): the layout `VM.__init__` gives
//! the monitor area, the process area, the loaded regions, the heap and the VirtualAlloc arena (`src/w2m/vm_backend.py`
//! 2382-2424), the process area (Sprint 140), the monitor code (Sprint 136), and in the code cache, in the Python's
//! order, the two switch stubs, one bridge per import thunk, the slice stub and the entry stub (2472-2507). Each
//! function cites the lines it carries.

use std::collections::{BTreeMap, BTreeSet};

use a64_encoder as a64;
use guest_runtime::Image;
use guest_runtime::layout::{
    BLOCK, CODE_CACHE, CURRENT_ENTRY, E_X, HEAP_SIZE, IPA_BASE, MONITOR_DATA, MONITOR_SIZE, MONITOR_VA, PAGE,
    PROCESS_VA, RECON_COUNT, RECON_ENTRIES, RECON_TABLE, RUN_TABLE, RUNTIME_STACK, RUNTIME_STACK_SIZE, SLICE_UNTIL,
    STATE_OFF, TIME_SLICE, VM_COMPILER,
};
use guest_runtime::services::Service;
use pe_loader::Loaded;

use crate::GIGA_BLOCK;
use crate::assembler::{Assembler, Target};
use crate::monitor_code::{
    self, MonitorCode, REG_BUDGET, STATE_REDIRECT, budget_from_count, count_from_budget, lock_runtime, unlock_runtime,
};
use crate::process::{self, Areas, PARAMETERS_ROOM, ProcessArea, ProcessError, Start, TRACE_SIZE};
use crate::tables::{TableError, write_translation_tables};

/// vm_backend 334-335: the VirtualAlloc arena, sparse.
pub const VIRTUAL_VA: u64 = 0x0000_0010_0000_0000;
pub const VIRTUAL_SIZE: u64 = 0x80_0000_0000;
/// vm_backend 261: the process area's size.
pub const PROCESS_SIZE: u64 = guest_runtime::layout::TRACE_OFF + TRACE_SIZE;
/// VM_COMPILER + this: the code cache's next free offset from MONITOR_VA (vm_backend 197, 2514-2520).
pub const VMC_CURSOR: u64 = 0x08;
/// VM_COMPILER + this: the dispatcher's, the slice stub's and the trace's addresses (vm_backend 197, 2508).
pub const VMC_EXIT: u64 = 0x18;
/// The state block's schedule word (vm_backend 1133).
pub const STATE_SCHEDULE: u32 = 0xF0;
/// hvc numbers (vm_backend 1126; `_bridge` 3746).
pub const HVC_SCHEDULE: u32 = 12;
pub const HVC_NO_SERVICE: u32 = 9;
/// The run table's header: the VM may switch threads (scheduler.py 42-43).
pub const RT_IN_VM: u32 = 0x18;
/// vm_backend 291-293: imports by ordinal, as Wine's specs number them.
const ORDINAL_NAMES: [(&str, &str, &str); 8] = [
    ("OLEAUT32.DLL", "#2", "SysAllocString"),
    ("OLEAUT32.DLL", "#6", "SysFreeString"),
    ("OLEAUT32.DLL", "#7", "SysStringLen"),
    ("OLEAUT32.DLL", "#8", "VariantInit"),
    ("OLEAUT32.DLL", "#9", "VariantClear"),
    ("WS2_32.DLL", "#111", "WSAGetLastError"),
    ("WS2_32.DLL", "#115", "WSAStartup"),
    ("WS2_32.DLL", "#116", "WSACleanup"),
];

/// Why a guest could not be laid out.
#[derive(Debug)]
pub enum GuestError {
    Memory(std::io::Error),
    Tables(TableError),
    Process(ProcessError),
    /// the Python's `CompileError`, in its words
    Layout(String),
}

/// `service_for` (vm_backend 2302-2315): the runtime service an import binds to (its C symbol, whether it sets RAX), or
/// `None`: `hvc #9` at the call. `services` is `guest_runtime::services::services`'s list.
pub fn service_for<'a>(services: &'a [Service], dll: &str, function: &str) -> Option<&'a Service> {
    let upper = dll.to_uppercase();
    let function = ORDINAL_NAMES
        .iter()
        .find(|(d, ordinal, _)| *d == upper && *ordinal == function)
        .map_or(function, |(_, _, name)| name);
    let find = |table: &str| services.iter().find(|s| s.dll == table && s.name == function);
    let kernel = upper == "KERNEL32.DLL" || upper == "KERNELBASE.DLL";
    let service = if kernel { find("KERNEL32") } else { find(&upper) };
    // kernel.appcore's AppPolicy* functions are threads.c's k32_ services (Sprint 063)
    let service = service.or_else(|| (upper == "KERNEL.APPCORE.DLL").then(|| find("KERNEL32")).flatten());
    // kernel32 exports RtlUnwindEx, RtlCaptureContext and the other Rtl* exception functions as its own, and they do
    // ntdll's work: one service serves both names
    service.or_else(|| kernel.then(|| find("NTDLL.DLL")).flatten())
}

fn round_up(n: u64, to: u64) -> u64 {
    (n + to - 1) & !(to - 1)
}

/// Where each part of guest memory sits in the memory the VM maps at `IPA_BASE` (vm_backend 2382-2392, 2421-2423).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// the loaded regions but the thunks: (region index in `Loaded::memory`, offset)
    pub placed: Vec<(usize, u64)>,
    pub heap_off: u64,
    pub virtual_off: u64,
    pub size: u64,
    /// (VA, IPA, size)
    pub maps: Vec<(u64, u64, u64)>,
}

impl Layout {
    pub fn new(loaded: &Loaded) -> Layout {
        let mut size = MONITOR_SIZE + PROCESS_SIZE;
        let mut placed = Vec::new();
        for (i, r) in loaded.memory.regions.iter().enumerate() {
            if r.name == "thunks" {
                continue;
            }
            placed.push((i, size));
            size += round_up(r.data.len() as u64, PAGE);
        }
        let heap_off = round_up(size, BLOCK); // the process heap: 2 MB aligned, mapped in blocks
        size = heap_off + HEAP_SIZE;
        // the VirtualAlloc arena: its IPA 1 GB aligned for 1 GB blocks; sparse
        let virtual_off = round_up(IPA_BASE + size, GIGA_BLOCK) - IPA_BASE;
        size = virtual_off + VIRTUAL_SIZE;
        let mut maps = vec![
            (MONITOR_VA, IPA_BASE, MONITOR_SIZE),
            (PROCESS_VA, IPA_BASE + MONITOR_SIZE, PROCESS_SIZE),
        ];
        maps.extend(placed.iter().map(|&(i, off)| {
            let r = &loaded.memory.regions[i];
            (r.base, IPA_BASE + off, round_up(r.data.len() as u64, PAGE))
        }));
        maps.push((guest_runtime::layout::HEAP_VA, IPA_BASE + heap_off, HEAP_SIZE));
        maps.push((VIRTUAL_VA, IPA_BASE + virtual_off, VIRTUAL_SIZE));
        Layout {
            placed,
            heap_off,
            virtual_off,
            size: round_up(size, 0x4000),
            maps,
        }
    }

    /// The memory offset of guest address `va`, if a map holds it.
    pub fn offset(&self, va: u64) -> Option<u64> {
        self.maps
            .iter()
            .find(|(v, _, n)| (*v..v + n).contains(&va))
            .map(|(v, ipa, _)| ipa - IPA_BASE + (va - v))
    }
}

fn read_u64(mem: &[u8], off: u64) -> u64 {
    u64::from_le_bytes(mem[off as usize..off as usize + 8].try_into().unwrap())
}

fn write_u64(mem: &mut [u8], off: u64, v: u64) {
    mem[off as usize..off as usize + 8].copy_from_slice(&v.to_le_bytes());
}

/// The code cache's cursor and its writes (vm_backend 2514-2520, 4401-4402, 4850-4859).
struct CodeCache<'a> {
    mem: &'a mut [u8],
}

impl CodeCache<'_> {
    fn cursor(&self) -> u64 {
        read_u64(self.mem, VM_COMPILER + VMC_CURSOR)
    }

    /// Puts `build`'s words at the cursor, advances it, and marks the code as not the guest's in the reconstruction
    /// table: a fault there is the runtime's (`_recon_append([(va, 0)])`).
    fn put(&mut self, build: impl FnOnce(&mut Assembler)) -> Result<u64, GuestError> {
        let off = self.cursor();
        let va = MONITOR_VA + off;
        let mut a = Assembler::new(va);
        build(&mut a);
        let words = a.finish();
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        self.mem[off as usize..off as usize + bytes.len()].copy_from_slice(&bytes);
        write_u64(self.mem, VM_COMPILER + VMC_CURSOR, off + bytes.len() as u64);
        self.recon_append(&[(va, 0)])?;
        Ok(va)
    }

    /// `_recon_append` (vm_backend 4850-4859): (host code VA, x86 address) pairs after the table's entries.
    fn recon_append(&mut self, marks: &[(u64, u64)]) -> Result<(), GuestError> {
        let n = read_u64(self.mem, RUN_TABLE + RECON_COUNT);
        if n + marks.len() as u64 > RECON_ENTRIES {
            return Err(GuestError::Layout("reconstruction table full".into()));
        }
        for (i, (host, guest)) in marks.iter().enumerate() {
            let at = RECON_TABLE + 16 * (n + i as u64);
            write_u64(self.mem, at, *host);
            write_u64(self.mem, at + 8, *guest);
        }
        write_u64(self.mem, RUN_TABLE + RECON_COUNT, n + marks.len() as u64);
        Ok(())
    }
}

/// `_switch_stub` (vm_backend 2650-2701): the thread switch inside the VM. Save the guest registers in the running
/// thread's run-table entry and its vectors in its slot, write the count, and call threads.c `w2m_switch(schedule,
/// stop)` on the runtime's stack; on 1 load the thread it chose and dispatch; on 0 restore and take the exit the stub
/// stands for (a slice's end returns to the block's `hvc #4` in x30, a bridge's hand-over makes `hvc #12`).
fn switch_stub(a: &mut Assembler, from_slice: bool, switch: u64, dispatch_va: u64) {
    let entry_in_x16 = |a: &mut Assembler| {
        a.emit(&a64::mov_imm(16, MONITOR_VA + MONITOR_DATA));
        a.emit(&[a64::ldr(8, 16, 16, CURRENT_ENTRY as u32)]);
    };
    let registers = |a: &mut Assembler, op: fn(u32, u32, u32, i64) -> u32| {
        for r in [0, 2, 4, 6, 8, 10, 12, 14, 17, 19, 21, 23, 25, 27, 29] {
            a.emit(&[op(r, r + 1, 16, (E_X + 8 * u64::from(r)) as i64)]);
        }
    };
    let vectors_at_x17 = |a: &mut Assembler| {
        a.emit(&[a64::ldr(8, 17, 16, 0x28)]); // E_VSLOT
        a.emit(&a64::mov_imm(18, MONITOR_VA));
        a.emit(&[a64::add(17, 17, 18, a64::LSL, 0)]);
    };
    lock_runtime(a); // before the first store to the shared run table
    entry_in_x16(a);
    registers(a, a64::stp);
    vectors_at_x17(a);
    for q in (0..16).step_by(2) {
        a.emit(&[a64::stp_q(q, q + 1, 17, 16 * i64::from(q))]);
    }
    a.emit(&[
        a64::mrs(18, a64::FPCR),
        a64::mrs(25, a64::FPSR),
        a64::stp(18, 25, 17, 256),
    ]);
    a.emit(&count_from_budget());
    a.emit(&a64::mov_imm(16, MONITOR_VA + RUNTIME_STACK + RUNTIME_STACK_SIZE));
    a.emit(&[a64::addi(31, 16, 0)]);
    if from_slice {
        a.emit(&[a64::movz(0, 0, 0), a64::mov(1, 30)]);
    } else {
        a.emit(&a64::mov_imm(16, PROCESS_VA + STATE_OFF));
        a.emit(&[a64::ldr(4, 0, 16, STATE_SCHEDULE), a64::movz(1, 0, 0)]);
    }
    a.emit(&a64::mov_imm(16, switch));
    a.emit(&[a64::blr(16)]);
    let decline = a.label();
    entry_in_x16(a);
    a.branch(|off| a64::cbz(0, off), decline);
    vectors_at_x17(a);
    for q in (0..16).step_by(2) {
        a.emit(&[a64::ldp_q(q, q + 1, 17, 16 * i64::from(q))]);
    }
    a.emit(&[
        a64::ldp(18, 25, 17, 256),
        a64::msr(a64::FPCR, 18),
        a64::msr(a64::FPSR, 25),
    ]);
    registers(a, a64::ldp);
    unlock_runtime(a); // x16 is not among the registers loaded
    a.branch(a64::b, Target::Va(dispatch_va));
    a.bind(decline);
    registers(a, a64::ldp);
    unlock_runtime(a);
    a.emit(&[if from_slice { a64::ret() } else { a64::hvc(HVC_SCHEDULE) }]);
}

/// `_slice_stub` (vm_backend 2622-2648): when the count is below SLICE_UNTIL and a further TIME_SLICE stays within
/// RUN_END, raise the limit and x23 and return past the block's `hvc #4`; otherwise, when the run table lets the VM
/// switch, on to the switch stub, else back to the `hvc`.
fn slice_stub(a: &mut Assembler, switch_slice_va: u64) {
    let out = a.label();
    a.emit(&[
        a64::ldp(16, 17, 28, SLICE_UNTIL as i64),
        a64::ldr(8, 18, 28, 8),
        a64::sub(25, 18, REG_BUDGET),
        a64::cmp(25, 16),
    ]);
    a.branch(|off| a64::bcond("hs", off), out);
    a.emit(&a64::mov_imm(25, TIME_SLICE));
    a.emit(&[a64::add(18, 18, 25, a64::LSL, 0), a64::cmp(18, 17)]);
    a.branch(|off| a64::bcond("hi", off), out);
    a.emit(&[
        a64::str(8, 18, 28, 8),
        a64::add(REG_BUDGET, REG_BUDGET, 25, a64::LSL, 0),
        a64::addi(30, 30, 4),
        a64::ret(),
    ]);
    a.bind(out);
    let hvc = a.label();
    a.emit(&a64::mov_imm(16, MONITOR_VA + RUN_TABLE));
    a.emit(&[a64::ldr(8, 16, 16, RT_IN_VM)]);
    a.branch(|off| a64::cbz(16, off), hvc);
    a.branch(a64::b, Target::Va(switch_slice_va));
    a.bind(hvc);
    a.emit(&[a64::ret()]);
}

/// `_entry_stub` (vm_backend 2609-2620): SP at the runtime stack's top, which the miss stub checks, then the dispatcher.
fn entry_stub(a: &mut Assembler, dispatch_va: u64) {
    a.emit(&a64::mov_imm(16, MONITOR_VA + RUNTIME_STACK + RUNTIME_STACK_SIZE));
    a.emit(&[a64::addi(31, 16, 0)]);
    a.branch(a64::b, Target::Va(dispatch_va));
}

/// `_bridge` (vm_backend 3733-3794) for an import with service `symbol` at `service_va`: save the guest GPRs in the
/// state block, pass RCX, RDX, R8, R9 and RSP in x0-x4, call the C service on the runtime stack under the runtime lock,
/// put its result in RAX unless it is void, restore the GPRs, and do RET's pop (the Microsoft x64 convention; AAPCS64
/// keeps x19-x28, so the flag registers survive). A service may set `state.redirect_rip` (the next x86 address instead
/// of the return; RSP keeps the return address) and `state.schedule`: RETRY leaves the call unreturned with RIP at
/// `thunk`; any schedule bit goes to the switch stub. With no service: `hvc #9`. Call tracing (`call_trace`) is not
/// carried yet.
fn bridge(a: &mut Assembler, service: Option<(u64, bool)>, thunk: u64, resume_va: u64, switch_va: u64) {
    let Some((service_va, sets_rax)) = service else {
        a.emit(&[a64::hvc(HVC_NO_SERVICE)]);
        return;
    };
    let state = PROCESS_VA + STATE_OFF;
    a.emit(&a64::mov_imm(16, state));
    for i in (0..16).step_by(2) {
        a.emit(&[a64::stp(i, i + 1, 16, 8 * i64::from(i))]);
    }
    a.emit(&[a64::mov(0, 1), a64::mov(1, 2), a64::mov(2, 8), a64::mov(3, 9)]); // x4 = RSP already
    a.emit(&a64::mov_imm(16, MONITOR_VA + RUNTIME_STACK + RUNTIME_STACK_SIZE));
    a.emit(&[a64::addi(31, 16, 0)]);
    a.emit(&count_from_budget());
    lock_runtime(a);
    a.emit(&a64::mov_imm(16, service_va));
    a.emit(&[a64::blr(16)]);
    a.emit(&budget_from_count());
    unlock_runtime(a);
    a.emit(&a64::mov_imm(16, state));
    if sets_rax {
        a.emit(&[a64::str(8, 0, 16, 0)]);
    }
    for i in (0..16).step_by(2) {
        a.emit(&[a64::ldp(i, i + 1, 16, 8 * i64::from(i))]);
    }
    let (not_retry, pop, after, to_host) = (a.label(), a.label(), a.label(), a.label());
    a.emit(&[a64::ldr(4, 17, 16, STATE_SCHEDULE)]);
    a.branch(|off| a64::tbz(17, 1, off), not_retry);
    a.emit(&a64::mov_imm(17, thunk)); // RETRY: RIP back at the call, RSP as it is
    a.branch(a64::b, to_host);
    a.bind(not_retry);
    a.emit(&[a64::ldr(8, 17, 16, STATE_REDIRECT)]);
    a.branch(|off| a64::cbz(17, off), pop);
    a.emit(&[a64::str(8, a64::ZR, 16, STATE_REDIRECT)]);
    a.branch(a64::b, after);
    a.bind(pop);
    a.emit(&[a64::ldr(8, 17, 4, 0), a64::addi(4, 4, 8)]);
    a.bind(after);
    a.emit(&[a64::ldr(4, 16, 16, STATE_SCHEDULE)]);
    a.branch(|off| a64::cbnz(16, off), to_host);
    a.branch(a64::b, Target::Va(resume_va));
    a.bind(to_host);
    a.branch(a64::b, Target::Va(switch_va)); // threads.c w2m_switch, or hvc #12
}

/// A program's guest memory, laid out and written up to its first instruction.
pub struct Guest {
    pub memory: hypervisor::GuestMemory,
    pub layout: Layout,
    pub ttbr: u64,
    pub code: MonitorCode,
    pub process: ProcessArea,
    pub switch_va: u64,
    pub switch_slice_va: u64,
    /// thunk address -> its bridge's address
    pub bridges: BTreeMap<u64, u64>,
    pub slice_va: u64,
    pub entry_va: u64,
}

impl Guest {
    /// `VM.__init__`'s memory and code (vm_backend 2382-2424, 2461, 2472-2475, 2506-2508) for `loaded`, with the
    /// runtime `image` and its `services`. The COM vtables (2488) and the VM compiler's symbol table (2509-2510) come
    /// with the run loop.
    pub fn new(
        loaded: &Loaded,
        image: &Image,
        services: &[Service],
        start: &Start,
        system_modules: &BTreeSet<String>,
    ) -> Result<Guest, GuestError> {
        let layout = Layout::new(loaded);
        let mut memory = hypervisor::GuestMemory::new(layout.size as usize).map_err(GuestError::Memory)?;
        let mem = memory.as_mut_slice();
        for &(i, off) in &layout.placed {
            let r = &loaded.memory.regions[i];
            mem[off as usize..off as usize + r.data.len()].copy_from_slice(&r.data);
        }
        for (va, data) in &image.segments {
            let off = (va - MONITOR_VA) as usize;
            mem[off..off + data.len()].copy_from_slice(data);
        }
        let process_area = {
            let (monitor, rest) = mem.split_at_mut(MONITOR_SIZE as usize);
            let (process, rest) = rest.split_at_mut(PROCESS_SIZE as usize);
            let heap_start = (layout.heap_off - MONITOR_SIZE - PROCESS_SIZE) as usize;
            let heap = &mut rest[heap_start..heap_start + PARAMETERS_ROOM as usize];
            process::write(&mut Areas { monitor, process, heap }, loaded, start, system_modules)
                .map_err(GuestError::Process)?
        };
        let (ttbr, _) = write_translation_tables(mem, &layout.maps).map_err(GuestError::Tables)?;
        write_u64(mem, VM_COMPILER + VMC_CURSOR, CODE_CACHE);
        let code = monitor_code::write(mem, &image.symbols);
        let switch = image.symbols["w2m_switch"];
        let mut cache = CodeCache { mem };
        let switch_va = cache.put(|a| switch_stub(a, false, switch, code.dispatch_va))?;
        let switch_slice_va = cache.put(|a| switch_stub(a, true, switch, code.dispatch_va))?;
        let mut bridges = BTreeMap::new();
        for (&thunk, name) in &loaded.thunks {
            let (dll, function) = name.split_once('!').unwrap_or((name.as_str(), ""));
            let service = service_for(services, dll, function).map(|s| (image.symbols[&s.symbol], s.sets_rax));
            let va = cache.put(|a| bridge(a, service, thunk, code.dispatch_va, switch_va))?;
            bridges.insert(thunk, va);
        }
        let slice_va = cache.put(|a| slice_stub(a, switch_slice_va))?;
        let entry_va = cache.put(|a| entry_stub(a, code.dispatch_va))?;
        write_u64(cache.mem, VM_COMPILER + VMC_EXIT, code.dispatch_va);
        write_u64(cache.mem, VM_COMPILER + VMC_EXIT + 8, slice_va);
        write_u64(cache.mem, VM_COMPILER + VMC_EXIT + 16, 0); // the trace opens from the start
        Ok(Guest {
            memory,
            layout,
            ttbr,
            code,
            process: process_area,
            switch_va,
            switch_slice_va,
            bridges,
            slice_va,
            entry_va,
        })
    }
}
