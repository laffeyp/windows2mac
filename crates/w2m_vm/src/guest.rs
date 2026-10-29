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
    SPLIT_TABLES, SPLIT_TABLES_SIZE, STATE_OFF, TIME_SLICE, TRANSLATION_TABLES, VM_COMPILER,
};
use guest_runtime::services::Service;
use pe_loader::{Loaded, THUNK_BASE};
use virtual_memory::{GuestPages, MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE, VirtualMemory};

use crate::GIGA_BLOCK;
use crate::assembler::{Assembler, Target};
use crate::monitor_code::{
    self, MonitorCode, REG_BUDGET, STATE_REDIRECT, Site, budget_from_count, count_from_budget, record_taking,
    release_runtime, take_runtime, unlock_runtime,
};
use crate::process::{self, Areas, PARAMETERS_ROOM, ProcessArea, ProcessError, Start, TRACE_SIZE};
use crate::tables::{TableError, write_translation_tables};

pub use crate::{VIRTUAL_SIZE, VIRTUAL_VA};
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
/// VM_COMPILER + this: the stencil table's symbols' addresses, in `vmc_symbol_name`'s order (vm_backend 198).
pub const VMC_SYMBOLS: u64 = 0x40;
/// VM_COMPILER + this: whether a miss compiles in the VM first (vm_backend 197, 4968-4969).
pub const VMC_ENABLED: u64 = 0x00;
/// VM_COMPILER + these: the C compiler's counts by cause, and their names (vm_backend 199-202).
pub const VMC_COUNTS: u64 = 0x100;
pub const VMC_COUNT_NAMES: [&str; 23] = [
    "compiled",
    "not a code page",
    "host only",
    "decode",
    "no stencil",
    "no choice",
    "encoder",
    "full",
    "key size",
    "guarded",
    "scan pages",
    "log full",
    "code cache full",
    "recon full",
    "cut by decode",
    "cut by no stencil",
    "cut by no choice",
    "cut by encoder",
    "cut by key size",
    "cut by guarded",
    "page needs a split",
    "pages protected",
    "pause",
];

const DXGI_OBJECT: [&str; 7] = [
    "QueryInterface",
    "AddRef",
    "Release",
    "SetPrivateData",
    "SetPrivateDataInterface",
    "GetPrivateData",
    "GetParent",
];

/// vm_backend 293-307: each COM class's vtable in the runtime image: its symbol, its DLL, its methods' name prefix, and
/// the methods after the DXGI object's seven (when `dxgi`) in vtable order. A slot whose method has no service stops
/// the run by its name when called (`hvc #9`).
const COM_VTABLES: [(&str, &str, &str, bool, &[&str]); 5] = [
    (
        "w2m_enumerator_vtable",
        "MMDEVAPI.dll",
        "W2mEnumerator",
        false,
        &[
            "QueryInterface",
            "AddRef",
            "Release",
            "EnumAudioEndpoints",
            "GetDefaultAudioEndpoint",
            "GetDevice",
            "RegisterEndpointNotificationCallback",
            "UnregisterEndpointNotificationCallback",
        ],
    ),
    (
        "w2m_collection_vtable",
        "MMDEVAPI.dll",
        "W2mCollection",
        false,
        &["QueryInterface", "AddRef", "Release", "GetCount", "Item"],
    ),
    (
        "w2m_dxgi_factory_vtable",
        "dxgi.dll",
        "W2mFactory",
        true,
        &[
            "EnumAdapters",
            "MakeWindowAssociation",
            "GetWindowAssociation",
            "CreateSwapChain",
            "CreateSoftwareAdapter",
        ],
    ),
    (
        "w2m_dxgi_adapter_vtable",
        "dxgi.dll",
        "W2mAdapter",
        true,
        &["EnumOutputs", "GetDesc", "CheckInterfaceSupport"],
    ),
    (
        "w2m_dxgi_output_vtable",
        "dxgi.dll",
        "W2mOutput",
        true,
        &[
            "GetDesc",
            "GetDisplayModeList",
            "FindClosestMatchingMode",
            "WaitForVBlank",
            "TakeOwnership",
            "ReleaseOwnership",
            "GetGammaControlCapabilities",
            "SetGammaControl",
            "GetGammaControl",
            "SetDisplaySurface",
            "GetDisplaySurfaceData",
            "GetFrameStatistics",
        ],
    ),
];

/// `_vm_compiler_symbols` (vm_backend 1068-1072): the quoted names in `stencil_table.h`'s `vmc_symbol_name[]`, in order.
pub fn vm_compiler_symbols(stencil_table_h: &str) -> Result<Vec<String>, String> {
    let start = stencil_table_h
        .find("vmc_symbol_name[")
        .ok_or("no vmc_symbol_name in the table")?;
    let body = &stencil_table_h[start..];
    let body = &body[..body.find("};").ok_or("vmc_symbol_name is not closed")?];
    Ok(body.split('"').skip(1).step_by(2).map(str::to_string).collect())
}
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
/// `timed`: the runtime word's takings timed (Sprint 164), the taking recorded before the guest registers are reloaded
/// (x18, x25 and x26 are reloaded there; x16 holds the entry they come from).
fn switch_stub(a: &mut Assembler, from_slice: bool, switch: u64, dispatch_va: u64, timed: bool) {
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
    take_runtime(a, timed); // before the first store to the shared run table
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
    if timed {
        record_taking(a, Site::Switch, 18, 25, 26);
    }
    registers(a, a64::ldp);
    unlock_runtime(a); // x16 is not among the registers loaded
    a.branch(a64::b, Target::Va(dispatch_va));
    a.bind(decline);
    if timed {
        record_taking(a, Site::Switch, 18, 25, 26);
    }
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
/// `timed`: the runtime word's takings timed (Sprint 164); x1 and x2 are free at the release (x0-x15 reloaded after).
/// call_trace.h's UNTRACED (vm_backend 256-259): functions that read or unwind their caller's context or never return
/// keep a plain bridge.
const UNTRACED: [&str; 16] = [
    "RaiseException",
    "RtlUnwindEx",
    "RtlUnwind",
    "RtlCaptureContext",
    "RtlVirtualUnwind",
    "RtlLookupFunctionEntry",
    "RtlRestoreContext",
    "RtlCaptureStackBackTrace",
    "CaptureStackBackTrace",
    "ExitProcess",
    "ExitThread",
    "FreeLibraryAndExitThread",
    "TerminateProcess",
    "GetProcAddress",
    "SetUnhandledExceptionFilter",
    "UnhandledExceptionFilter",
];

/// `TRACE_WIDE_ARGS` (vm_backend 253): functions whose argument i is a UTF-16 string the trace keeps, bit i.
const TRACE_WIDE_ARGS: [(&str, u8); 1] = [("SetThreadDescription", 1 << 1)];

/// vm_backend 3754-3757 (Sprint 191): with the trace on, a bridge calls `w2m_traced`, but for UNTRACED functions and
/// w2m's own COM methods (`W2m*`, reached through a vtable, not an import); a function whose arguments the trace keeps
/// as strings gets its mask byte. Some(`w2m_traced`'s address) for the traced form.
fn traced_form(
    call_trace: &[String],
    function: &str,
    symbols: &BTreeMap<String, u64>,
    mem: &mut [u8],
    thunk: u64,
) -> Option<u64> {
    use guest_runtime::layout::{TRACE_OFF, TRACE_THUNKS, TRACE_WIDE_OFF};
    if call_trace.is_empty() || UNTRACED.contains(&function) || function.starts_with("W2m") {
        return None;
    }
    let index = (thunk - THUNK_BASE) / 16;
    if let Some((_, mask)) = TRACE_WIDE_ARGS.iter().find(|(f, _)| *f == function)
        && index < TRACE_THUNKS
    {
        mem[(MONITOR_SIZE + TRACE_OFF + TRACE_WIDE_OFF + index) as usize] = *mask;
    }
    symbols.get("w2m_traced").copied()
}

fn bridge(
    a: &mut Assembler,
    service: Option<(u64, bool)>,
    thunk: u64,
    resume_va: u64,
    switch_va: u64,
    timed: bool,
    traced: Option<u64>,
) {
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
    // vm_backend 3754-3760 (Sprint 191): a traced call goes through `w2m_traced` (call_trace.c), the service in x5 and
    // the thunk's index in x6
    let target = match traced {
        Some(traced_va) => {
            a.emit(&a64::mov_imm(5, service_va));
            a.emit(&a64::mov_imm(6, (thunk - pe_loader::THUNK_BASE) / 16));
            traced_va
        }
        None => service_va,
    };
    a.emit(&count_from_budget());
    take_runtime(a, timed);
    a.emit(&a64::mov_imm(16, target));
    a.emit(&[a64::blr(16)]);
    a.emit(&budget_from_count());
    release_runtime(a, timed, Site::Bridge, (16, 1, 2));
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
    /// thunk address -> "DLL!Name": the loader's, then those the COM vtables made
    pub thunks: BTreeMap<u64, String>,
    /// thunk address -> its bridge's address
    pub bridges: BTreeMap<u64, u64>,
    pub slice_va: u64,
    pub entry_va: u64,
    /// the services and the runtime's symbols, for a bridge made after construction (`make_thunk`, 2920-2924)
    pub services: Vec<Service>,
    pub symbols: BTreeMap<String, u64>,
    /// the VirtualAlloc arena (vm_backend 2498-2505)
    pub arena: VirtualMemory,
    /// guest_stubs.rs's page, once written (vm_backend 2492)
    pub stub_page: u64,
    /// the guest's standard output, as HOST_WRITE gives it (vm_backend `self.out`, Sprint 155)
    pub stdout: Vec<u8>,
    /// the program's image as `Modules` holds it (vm_backend 2958-2959): (base, SizeOfImage)
    pub image_range: Option<(u64, u64)>,
    /// the program's DLLs as `Modules.dlls` holds them (2958): (base, SizeOfImage)
    pub dll_ranges: Vec<(u64, u64)>,
    /// the split tables' next free page (`_next_table`, 2425): `page_descriptor`'s, which thread creation's
    /// `_map_pages` and the host's compiles share (Sprint 156)
    pub next_table: u64,
    /// the threads by id (`self.threads`, Sprint 156): the host's own fields; their run table entries are in memory
    pub threads: BTreeMap<u64, crate::threads::Thread>,
    /// the runtime word's takings timed in the monitor code and bridges (`Start::time_runtime_word`, Sprint 164)
    pub time_runtime_word: bool,
    /// the loader's images, the program first, and the DLLs whose entry runs, in order (`self.ld.images`,
    /// `self.ld.init_order`): what a created thread's stack, static TLS and attach stub read (Sprint 156)
    pub images: Vec<pe_loader::Image>,
    pub init_order: Vec<usize>,
    /// the images whose calls call_trace.c records (`Start::call_trace`; Sprint 191)
    pub call_trace: Vec<String>,
    /// the host directory that is C:\w2m\app (`Start::app_root`; Sprint 196)
    pub app_root: Option<std::path::PathBuf>,
    /// the block whose first compile opens the trace (`_trace_from`, vm_backend 2483-2487), 0 for from the start
    pub trace_from: u64,
    /// GetProcAddress's thunks: untraced ones given while the trace was closed, traced ones its hook gave after
    /// (`_trace_quiet`, `_trace_dynamic`), by "DLL!Name"
    pub trace_quiet: BTreeMap<String, u64>,
    pub trace_dynamic: BTreeMap<String, u64>,
    /// the traced images whose import slots are already on their own thunks (`_trace_bound`)
    pub trace_bound: BTreeSet<String>,
    /// the objects this process shares through KERNEL 33 (`kernel.py`; Sprint 193)
    pub kernel: crate::kernel::Objects,
    /// the process's place in the run's machine (Sprint 197): None alone and not yet a parent
    pub link: Option<crate::turns::Link>,
    /// how the process started: a child's start copies its environment, group and clock (`spawner`; Sprint 197)
    pub start: Start,
    /// the host's random bytes on the instruction clock, `random.Random(0x5EED)` (vm_backend 2357; Sprint 203); None:
    /// the host's CSPRNG
    pub random: Option<crate::mt19937::Random>,
}

/// Guest memory by guest address, for the arena (`_zero`, and the `read` the arena is given, vm_backend 2498-2500).
pub struct GuestView<'a> {
    pub mem: &'a mut [u8],
    pub layout: &'a Layout,
}

impl GuestView<'_> {
    fn at(&self, va: u64, n: u64) -> std::ops::Range<usize> {
        let off = self
            .layout
            .offset(va)
            .unwrap_or_else(|| panic!("guest address 0x{va:x} is not mapped")) as usize;
        off..off + n as usize
    }
}

impl GuestPages for GuestView<'_> {
    fn zero(&mut self, va: u64, n: u64) {
        let r = self.at(va, n);
        self.mem[r].fill(0);
    }

    fn read(&self, va: u64, n: u64) -> Vec<u8> {
        self.mem[self.at(va, n)].to_vec()
    }
}

/// The last unhandled exception the guest runtime recorded: the EXCEPTION_RECORD and the CONTEXT fields it built
/// (vm_backend `exception`, 4173-4195; runtime.h `w2m_exception_area`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exception {
    pub code: u32,
    pub flags: u32,
    pub address: u64,
    pub parameters: u32,
    pub information: [u64; 2],
    pub context_flags: u32,
    pub mxcsr: u32,
    /// CS, DS, ES, FS, GS, SS
    pub segments: [u16; 6],
    pub eflags: u32,
    pub gpr: [u64; 16],
    pub rip: u64,
    pub fltsave_control_word: u16,
    pub fltsave_mxcsr: u32,
    pub xmm: [u128; 16],
}

/// runtime.h 222: "ETIONw2x".
pub const EXCEPTION_MAGIC: u64 = 0x7832_774E_4F49_5445;

impl Guest {
    /// `VM.__init__`'s memory and code (vm_backend 2382-2424, 2461, 2472-2475, 2506-2508) for `loaded`, with the
    /// runtime `image`, its `services`, and `vm_compiler_symbols`, the stencil table's symbols in its order
    /// (`vm_compiler_symbols`); with the COM vtables (2488, 3078-3093) and the symbol table (2509-2510).
    pub fn new(
        loaded: &Loaded,
        image: &Image,
        services: &[Service],
        start: &Start,
        system_modules: &BTreeSet<String>,
        vm_compiler_symbols: &[String],
    ) -> Result<Guest, GuestError> {
        let layout = Layout::new(loaded);
        let mut memory = match start.memory_fd {
            // a child's: the supervisor's unlinked file, sized here and mapped shared (Sprint 197)
            Some(fd) => {
                use std::os::fd::FromRawFd;
                let file = std::mem::ManuallyDrop::new(unsafe { std::fs::File::from_raw_fd(fd) });
                file.set_len(layout.size).map_err(GuestError::Memory)?;
                hypervisor::GuestMemory::shared(fd, layout.size as usize).map_err(GuestError::Memory)?
            }
            None => hypervisor::GuestMemory::new(layout.size as usize).map_err(GuestError::Memory)?,
        };
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
        let timed = start.time_runtime_word;
        let code = monitor_code::write(mem, &image.symbols, timed);
        let switch = image.symbols["w2m_switch"];
        let mut cache = CodeCache { mem };
        let switch_va = cache.put(|a| switch_stub(a, false, switch, code.dispatch_va, timed))?;
        let switch_slice_va = cache.put(|a| switch_stub(a, true, switch, code.dispatch_va, timed))?;
        let mut thunks = loaded.thunks.clone();
        let mut bridges = BTreeMap::new();
        let mut bind = |cache: &mut CodeCache<'_>, thunk: u64, name: &str| -> Result<(), GuestError> {
            let (dll, function) = name.split_once('!').unwrap_or((name, ""));
            let service = service_for(services, dll, function).map(|s| (image.symbols[&s.symbol], s.sets_rax));
            let traced = traced_form(&start.call_trace, function, &image.symbols, cache.mem, thunk);
            let va = cache.put(|a| bridge(a, service, thunk, code.dispatch_va, switch_va, timed, traced))?;
            bridges.insert(thunk, va);
            Ok(())
        };
        for (&thunk, name) in &loaded.thunks {
            bind(&mut cache, thunk, name)?;
        }
        // `_com_vtables` (3078-3085): each COM class's vtable in the runtime image filled with its methods' thunks, each
        // an import thunk (`_import_thunk`, 3087-3092) made after the loader's, with its bridge
        let mut imports: BTreeMap<String, u64> = thunks.iter().map(|(t, n)| (n.to_lowercase(), *t)).collect();
        for (symbol, dll, prefix, dxgi, methods) in COM_VTABLES {
            let at = image.symbols[symbol] - MONITOR_VA;
            let all = if dxgi { &DXGI_OBJECT[..] } else { &[][..] }.iter().chain(methods);
            for (i, method) in all.enumerate() {
                let name = format!("{dll}!{prefix}{method}");
                let thunk = match imports.get(&name.to_lowercase()) {
                    Some(t) => *t,
                    None => {
                        let t = pe_loader::THUNK_BASE + 16 * thunks.len() as u64;
                        thunks.insert(t, name.clone());
                        bind(&mut cache, t, &name)?;
                        imports.insert(name.to_lowercase(), t);
                        t
                    }
                };
                write_u64(cache.mem, at + 8 * i as u64, thunk);
            }
        }
        let slice_va = cache.put(|a| slice_stub(a, switch_slice_va))?;
        let entry_va = cache.put(|a| entry_stub(a, code.dispatch_va))?;
        write_u64(cache.mem, VM_COMPILER + VMC_EXIT, code.dispatch_va);
        write_u64(cache.mem, VM_COMPILER + VMC_EXIT + 8, slice_va);
        write_u64(cache.mem, VM_COMPILER + VMC_EXIT + 16, 0); // the trace opens from the start
        for (i, name) in vm_compiler_symbols.iter().enumerate() {
            let va = *image.symbols.get(name).ok_or_else(|| {
                GuestError::Layout(format!("the stencil table names {name}, which the runtime lacks"))
            })?;
            write_u64(cache.mem, VM_COMPILER + VMC_SYMBOLS + 8 * i as u64, va);
        }
        // the arena (2498-2505): the program's images, the main stack, the heap, every mapped range and the thunks'
        // 4 GB, and each writable image page as loaded
        let images = if loaded.images.is_empty() {
            loaded.image.iter().map(|p| (p.image_base, p.clone())).collect()
        } else {
            loaded.images.iter().map(|i| (i.base, i.pe.clone())).collect()
        };
        let mut arena = VirtualMemory::new(VIRTUAL_VA, VIRTUAL_SIZE, images);
        arena.stack = Some(process_area.stack);
        arena.heap = Some((guest_runtime::layout::HEAP_VA, HEAP_SIZE));
        arena.mapped = layout.maps.iter().map(|(va, _, size)| (*va, *size)).collect();
        arena.mapped.push((THUNK_BASE, 1 << 32));
        arena.snapshot(&GuestView {
            mem: memory.as_mut_slice(),
            layout: &layout,
        });
        Guest {
            memory,
            layout,
            ttbr,
            code,
            process: process_area,
            switch_va,
            switch_slice_va,
            thunks,
            bridges,
            slice_va,
            entry_va,
            services: services.to_vec(),
            symbols: image.symbols.clone(),
            arena,
            stub_page: 0,
            stdout: Vec::new(),
            image_range: loaded.image.as_ref().map(|p| (p.image_base, p.size_of_image as u64)),
            dll_ranges: loaded
                .images
                .iter()
                .skip(1)
                .map(|d| (d.base, d.pe.size_of_image as u64))
                .collect(),
            next_table: SPLIT_TABLES,
            threads: BTreeMap::new(),
            time_runtime_word: timed,
            images: loaded.images.clone(),
            init_order: loaded.init_order.clone(),
            call_trace: start.call_trace.clone(),
            app_root: start.app_root.clone(),
            trace_from: 0,
            trace_quiet: BTreeMap::new(),
            trace_dynamic: BTreeMap::new(),
            trace_bound: BTreeSet::new(),
            kernel: crate::kernel::Objects::new(),
            link: None,
            start: start.clone(),
            random: start.instruction_clock.map(|_| crate::mt19937::Random::new(0x5EED)),
        }
        .with_trace(start)
    }

    /// The trace's start (vm_backend 2480-2487, 2508; Sprint 191): the traced images' slots on their own thunks, and,
    /// with `call_trace_from`, the trace closed until that export's block is first compiled, its address given to the
    /// VM's compiler (`VMC_EXIT + 16`), which leaves that block to the host.
    fn with_trace(mut self, start: &Start) -> Result<Guest, GuestError> {
        use guest_runtime::layout::TRACE_OFF;
        self.trace_ranges()?;
        if let Some(from) = &start.call_trace_from {
            let (dll, export) = from.split_once('!').unwrap_or((from, ""));
            let image = self
                .images
                .iter()
                .find(|im| im.name.eq_ignore_ascii_case(dll))
                .ok_or_else(|| GuestError::Layout(format!("the trace opens at {from}, whose image is not loaded")))?;
            let at = image
                .export(export)
                .ok()
                .flatten()
                .ok_or_else(|| GuestError::Layout(format!("the trace opens at {from}, which is not exported")))?;
            self.trace_from = at;
            let mem = self.memory.as_mut_slice();
            mem[(MONITOR_SIZE + TRACE_OFF) as usize..(MONITOR_SIZE + TRACE_OFF + 8) as usize].fill(0);
            write_u64(mem, VM_COMPILER + VMC_EXIT + 16, at);
        }
        Ok(self)
    }

    /// `_page_descriptor` (4507-4533): the memory offset of the level-3 descriptor that maps page `va`, a 1 GB or 2 MB
    /// block over it split into the next level's 512 descriptors with the block's attributes; 0 when nothing maps it.
    /// One vCPU: no private regions.
    pub fn page_descriptor(&mut self, va: u64) -> Result<u64, String> {
        let mem = self.memory.as_mut_slice();
        let mut off = TRANSLATION_TABLES;
        for (level, shift) in [(0u32, 39u32), (1, 30), (2, 21)] {
            let at = off + 8 * ((va >> shift) & 0x1FF);
            let mut desc = read_u64(mem, at);
            if desc & 1 == 0 {
                return Ok(0);
            }
            if desc & 2 == 0 {
                // a block: split it
                let size = 1u64 << shift;
                let base = desc & 0x0000_FFFF_FFFF_F000 & !(size - 1);
                let attributes = desc & !0x0000_FFFF_FFFF_F000 & !0x3;
                let new = self.next_table;
                if new + PAGE > SPLIT_TABLES + SPLIT_TABLES_SIZE {
                    return Err("stage-1 split table area full".into());
                }
                self.next_table += PAGE;
                let low = if level == 2 { 0x3 } else { 0x1 }; // pages at level 3, blocks at level 2
                for i in 0..512u64 {
                    write_u64(mem, new + 8 * i, (base + i * (size >> 9)) | attributes | low);
                }
                desc = (IPA_BASE + new) | 0x3;
                write_u64(mem, at, desc);
            }
            off = (desc & 0x0000_FFFF_FFFF_F000) - IPA_BASE;
        }
        Ok(off + 8 * ((va >> 12) & 0x1FF))
    }

    /// `_map_pages` (3152-3159): the pages [start, end) mapped or unmapped in the stage-1 tables (the descriptor's
    /// valid bit). The guest runtime flushes the TLB after the request that changed them.
    pub fn map_pages(&mut self, start: u64, end: u64, valid: bool) -> Result<(), String> {
        let mut page = start;
        while page < end {
            let at = self.page_descriptor(page)?;
            if at != 0 {
                let mem = self.memory.as_mut_slice();
                let desc = read_u64(mem, at);
                write_u64(mem, at, if valid { desc | 1 } else { desc & !1 });
            }
            page += PAGE;
        }
        Ok(())
    }

    /// The memory offset of the process area's `off` (`_process`).
    fn process_off(off: u64) -> usize {
        (MONITOR_SIZE + off) as usize
    }

    /// `_import_thunk` (3087-3092) with `make_thunk` (2920-2924): the thunk `name` ("KERNEL32.dll!Name") has, or a new
    /// one after the others with its bridge.
    pub fn import_thunk(&mut self, name: &str) -> Result<u64, GuestError> {
        let key = name.to_lowercase();
        if let Some((t, _)) = self.thunks.iter().find(|(_, n)| n.to_lowercase() == key) {
            return Ok(*t);
        }
        self.make_thunk(name)
    }

    /// `make_thunk` (2920-2924): a new thunk after the others, with its bridge, whatever thunks the name has already
    /// (a traced image's own slots and GetProcAddress's hook each need one of their own; Sprint 191).
    pub fn make_thunk(&mut self, name: &str) -> Result<u64, GuestError> {
        let thunk = THUNK_BASE + 16 * self.thunks.len() as u64;
        self.thunks.insert(thunk, name.to_string());
        let (dll, function) = name.split_once('!').unwrap_or((name, ""));
        let service = service_for(&self.services, dll, function).map(|s| (self.symbols[&s.symbol], s.sets_rax));
        let (dispatch_va, switch_va, timed) = (self.code.dispatch_va, self.switch_va, self.time_runtime_word);
        let traced = traced_form(
            &self.call_trace,
            function,
            &self.symbols,
            self.memory.as_mut_slice(),
            thunk,
        );
        let mut cache = CodeCache {
            mem: self.memory.as_mut_slice(),
        };
        let va = cache.put(|a| bridge(a, service, thunk, dispatch_va, switch_va, timed, traced))?;
        self.bridges.insert(thunk, va);
        Ok(thunk)
    }

    /// The owner byte of `thunk` in the trace header (call_trace.c `owner`): 1 + a traced image, 1 + TRACE_DYNAMIC for
    /// GetProcAddress's hook, 0xFF for never; thunks past the first TRACE_THUNKS have none.
    fn trace_owner(&mut self, thunk: u64, owner: u8) {
        use guest_runtime::layout::{TRACE_OFF, TRACE_OWNER_OFF, TRACE_THUNKS};
        let index = (thunk - THUNK_BASE) / 16;
        if index < TRACE_THUNKS {
            self.memory.as_mut_slice()[(MONITOR_SIZE + TRACE_OFF + TRACE_OWNER_OFF + index) as usize] = owner;
        }
    }

    /// `_trace_ranges` (4097-4120; Sprint 191): each traced image's range in the trace header as it is mapped, and its
    /// import slots on thunks of their own, owner 1 + the image, as tools/jit_capture points each hooked image's slots at
    /// its own thunks; a slot bound to a game DLL's export (no Windows hook there) stays.
    pub fn trace_ranges(&mut self) -> Result<(), GuestError> {
        use guest_runtime::layout::TRACE_OFF;
        for (i, name) in self.call_trace.clone().iter().enumerate() {
            let Some(image) = self
                .images
                .iter()
                .find(|im| im.name.eq_ignore_ascii_case(name))
                .cloned()
            else {
                continue;
            };
            let (base, end) = (image.base, image.base + u64::from(image.pe.size_of_image));
            let at = (MONITOR_SIZE + TRACE_OFF + 40 + 16 * i as u64) as usize;
            let mem = self.memory.as_mut_slice();
            mem[at..at + 8].copy_from_slice(&base.to_le_bytes());
            mem[at + 8..at + 16].copy_from_slice(&end.to_le_bytes());
            if self.trace_bound.contains(&name.to_lowercase()) {
                continue;
            }
            self.trace_bound.insert(name.to_lowercase());
            for &slot in image.pe.iat.keys() {
                let va = base + u64::from(slot);
                let off = self
                    .layout
                    .offset(va)
                    .ok_or_else(|| GuestError::Layout(format!("{name}'s import slot 0x{va:x} is not mapped")))?
                    as usize;
                let held = u64::from_le_bytes(self.memory.as_slice()[off..off + 8].try_into().unwrap());
                let Some(key) = self.thunks.get(&held).cloned() else {
                    continue;
                };
                let own = self.make_thunk(&key)?;
                self.memory.as_mut_slice()[off..off + 8].copy_from_slice(&own.to_le_bytes());
                self.trace_owner(own, i as u8 + 1);
            }
        }
        Ok(())
    }

    /// `_traced_proc` (4062-4095; Sprint 191): GetProcAddress's answer as the capture's process sees it. With the trace
    /// closed, a thunk of its own marked never traced; with it open and the caller (the return address at the saved
    /// RSP) inside a traced image, a thunk of its own with the GetProcAddress label; another caller's, the shared one.
    pub fn traced_proc(&mut self, va: u64) -> Result<u64, GuestError> {
        use guest_runtime::layout::{TRACE_DYNAMIC, TRACE_OFF};
        if self.call_trace.is_empty() {
            return Ok(va);
        }
        let Some(key) = self.thunks.get(&va).cloned() else {
            return Ok(va);
        };
        let t = (MONITOR_SIZE + TRACE_OFF) as usize;
        let word = |mem: &[u8], at: usize| u64::from_le_bytes(mem[at..at + 8].try_into().unwrap());
        if word(self.memory.as_slice(), t) != 0 {
            let mem = self.memory.as_slice();
            let rsp = word(mem, (MONITOR_SIZE + STATE_OFF + 8 * 4) as usize);
            // the Python's `self.mem.read(rsp, 8)` raises on an unmapped RSP (Sprint 194; reviewer-2's Low 1)
            let caller = match self.layout.offset(rsp) {
                Some(off) => word(mem, off as usize),
                None => {
                    return Err(GuestError::Layout(format!(
                        "GetProcAddress's saved RSP {rsp:#x} is not mapped"
                    )));
                }
            };
            let traced_caller = (0..2).any(|i| {
                let (lo, hi) = (word(mem, t + 40 + 16 * i), word(mem, t + 48 + 16 * i));
                lo <= caller && caller < hi
            });
            if !traced_caller {
                return Ok(va);
            }
            if let Some(hooked) = self.trace_dynamic.get(&key) {
                return Ok(*hooked);
            }
            let hooked = self.make_thunk(&key)?;
            self.trace_dynamic.insert(key, hooked);
            self.trace_owner(hooked, 1 + TRACE_DYNAMIC);
            return Ok(hooked);
        }
        if let Some(quiet) = self.trace_quiet.get(&key) {
            return Ok(*quiet);
        }
        let quiet = self.make_thunk(&key)?;
        self.trace_quiet.insert(key, quiet);
        self.trace_owner(quiet, 0xFF);
        Ok(quiet)
    }

    /// `traced_calls` (4133-4147; Sprint 191): the calls call_trace.c recorded, in `scripts/call_compare.py`
    /// `write_tsv`'s form (seq, tid, label, DLL, function, eight arguments, return value and last error, hex but for
    /// seq, tid and the error), and the count the full area left out.
    ///
    /// From Sprint 194 (reviewer-2's Medium 2 and Low 1 of its review of 190-192) it also returns the records not yet
    /// ready, the calls in flight, as (seq, tid, image, dll, function, a0..a7), and a thunk with no name is an error,
    /// as the Python's `self.ld.thunks[...]` raises.
    pub fn traced_calls(&self) -> Result<(String, String, u64), String> {
        use guest_runtime::layout::{TRACE_OFF, TRACE_RECORD, TRACE_RECORDS_OFF};
        let mem = self.memory.as_slice();
        let t = (MONITOR_SIZE + TRACE_OFF) as usize;
        let word = |at: usize| u64::from_le_bytes(mem[at..at + 8].try_into().unwrap());
        let (count, dropped) = (word(t + 16), word(t + 32));
        let (mut tsv, mut in_flight) = (String::new(), String::new());
        for k in 0..count as usize {
            let r = t + (TRACE_RECORDS_OFF + TRACE_RECORD * k as u64) as usize;
            let f: Vec<u64> = (0..15).map(|i| word(r + 8 * i)).collect();
            let thunk = THUNK_BASE + 16 * f[4];
            let Some((dll, function)) = self.thunks.get(&thunk).and_then(|n| n.split_once('!')) else {
                return Err(format!(
                    "trace record {k} (seq {}) names thunk {thunk:#x}, which has no name",
                    f[1]
                ));
            };
            let label = self
                .call_trace
                .get(f[3] as usize)
                .map_or("GetProcAddress", String::as_str);
            let args: Vec<String> = f[5..13].iter().map(|a| format!("{a:x}")).collect();
            let call = format!("{}\t{}\t{label}\t{dll}\t{function}\t{}", f[1], f[2], args.join("\t"));
            if f[0] == 0 {
                in_flight += &format!("{call}\n");
            } else {
                tsv += &format!("{call}\t{:x}\t{}\n", f[13], f[14]);
            }
        }
        Ok((tsv, in_flight, dropped))
    }

    /// `_code_page` (vm_backend): x86 code the host writes, on fresh pages of the arena, an address never translated
    /// before.
    pub fn code_page(&mut self, code: &[u8]) -> Result<u64, GuestError> {
        let mut view = GuestView {
            mem: self.memory.as_mut_slice(),
            layout: &self.layout,
        };
        let (va, err) = self
            .arena
            .alloc(
                &mut view,
                0,
                code.len() as u64,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_EXECUTE_READWRITE,
            )
            .map_err(|e| GuestError::Layout(e.0))?;
        if err != 0 {
            return Err(GuestError::Layout(format!(
                "a host code page: VirtualAlloc error {err}"
            )));
        }
        let r = view.at(va, code.len() as u64);
        view.mem[r].copy_from_slice(code);
        Ok(va)
    }

    /// `_guest_stubs`: the x86 stubs services call guest code and resume contexts through (guest_stubs.rs), on a page
    /// of the arena written at the first use, so a process that needs none keeps its address space.
    pub fn guest_stubs(&mut self) -> Result<u64, GuestError> {
        if self.stub_page == 0 {
            let thunk = self.import_thunk("KERNEL32.dll!W2mGuestReturn")?;
            self.stub_page = self.code_page(&crate::guest_stubs::page(thunk))?;
        }
        Ok(self.stub_page)
    }

    /// `exception` (4173-4195): the last unhandled exception the guest runtime recorded, or None.
    pub fn exception(&self) -> Option<Exception> {
        let mem = self.memory.as_slice();
        let e = Self::process_off(guest_runtime::layout::EXCEPTION_OFF);
        let q = |at: usize| u64::from_le_bytes(mem[at..at + 8].try_into().unwrap());
        let d = |at: usize| u32::from_le_bytes(mem[at..at + 4].try_into().unwrap());
        let h = |at: usize| u16::from_le_bytes(mem[at..at + 2].try_into().unwrap());
        if q(e) != EXCEPTION_MAGIC {
            return None;
        }
        let (r, c) = (e + 16, e + 0x100);
        Some(Exception {
            code: d(r),
            flags: d(r + 4),
            address: q(r + 16),
            parameters: d(r + 24),
            information: [q(r + 32), q(r + 40)],
            context_flags: d(c + 48),
            mxcsr: d(c + 52),
            segments: std::array::from_fn(|i| h(c + 56 + 2 * i)),
            eflags: d(c + 68),
            gpr: std::array::from_fn(|i| q(c + 120 + 8 * i)),
            rip: q(c + 248),
            fltsave_control_word: h(c + 256),
            fltsave_mxcsr: d(c + 256 + 24),
            xmm: std::array::from_fn(|i| {
                let at = c + 256 + 160 + 16 * i;
                u128::from_le_bytes(mem[at..at + 16].try_into().unwrap())
            }),
        })
    }

    /// `clear_exception` (4197-4200): forget the last unhandled exception.
    pub fn clear_exception(&mut self) {
        let at = Self::process_off(guest_runtime::layout::EXCEPTION_OFF);
        self.memory.as_mut_slice()[at..at + 8].fill(0);
    }
}
