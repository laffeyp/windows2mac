// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! Guest threads in the Rust host (Sprint 156): a thread's run table entry (`_add_thread`, `src/w2m/vm_backend.py`
//! 3328-3350, and `GuestThread`, `src/w2m/scheduler.py` 60-160), CreateThread's host half (`_create_thread`,
//! 3094-3150, with `_thread_callbacks` 3017-3031, `_thread_static_tls` 3033-3060, `_write_vectors` 3352-3356 and
//! scheduler.py's `attach_stub`), the step budget (`_set_budget`, 3284-3289) and a run's slice end (`_slice_until`,
//! 2756-2780) for one vCPU in one process. The switch between threads is the VM's own (threads.c `w2m_switch`), but
//! for what it leaves to the host (Sprint 157): a thread's end, ExitThread's detach stub and exit code, and the
//! hand-over `hvc #12` makes (`_schedule`, `_switch`, `_save_thread`, `_load_thread`, 3463-3580).

use guest_runtime::layout::{
    CURRENT, CURRENT_ENTRY, E_MONITOR, E_MONITOR_AT, E_STATE, E_X, MONITOR_DATA, MONITOR_SIZE, MONITOR_VA, PAGE,
    PAUSE_COUNT, PEB_OFF, PROCESS_VA, RT_ENTRIES, RUN_ENTRY, RUN_TABLE, RUN_THREADS, SLICE_COUNT, SLICE_PAUSES,
    STATE_OFF, STATE_SIZE, TIME_SLICE, VECTOR_SLOT, VECTOR_SLOTS,
};
use pe_loader::TLS_SLOTS;
use virtual_memory::{GuestPages, MEM_COMMIT, MEM_RESERVE, PAGE_GUARD, PAGE_READWRITE, STACK_GUARD_PAGES};

use crate::guest::{Guest, GuestView};
use crate::modules::ModuleError;
use crate::process::{CLOCK_MODE, CLOCK_RATE, CLOCK_WARP};
use crate::run::{FCW_OFF, MXCSR_OFF, REG_UNDEFINED_FLAGS, RUN_END, fpcr_for};

/// The run table's header and an entry's words (scheduler.py 42-47).
pub const RT_COUNT: u64 = 0x00;
pub const RT_BLOCKED: u64 = 0x20;
pub const E_TID: u64 = 0x00;
pub const E_STATUS: u64 = 0x08;
pub const E_SUSPEND: u64 = 0x10;
pub const E_DEADLINE: u64 = 0x18;
pub const E_TEB: u64 = 0x20;
pub const E_VSLOT: u64 = 0x28;
pub const E_INSTRUCTIONS: u64 = 0x30;
pub const E_PAUSES: u64 = 0x38;
pub const E_VCPU: u64 = 0x138;
/// `STATUS_CODES` (scheduler.py 50).
pub const READY: u64 = 0;
pub const BLOCKED: u64 = 1;
pub const ENDED: u64 = 2;
/// state.schedule's bits (vm_backend 1134; runtime.h).
pub const SCHEDULE_RETRY: u32 = 2;
pub const SCHEDULE_END: u32 = 4;
pub const SCHEDULE_ANSWER: u32 = 8;
/// runtime.h W2M_THREAD_SCRATCH_OFF (scheduler.py 32): a thread's scratch in its TEB; its first word a wait's deadline.
const SCRATCH_OFF: u64 = 0x1900;
/// The monitor data ranges each thread keeps (scheduler.py 31): lazy AF and DF, MXCSR, the x87 control word, the YMM
/// upper halves.
pub const PER_THREAD_MONITOR: [(u64, u64); 4] = [(0x20, 0x40), (0x50, 0x58), (0x70, 0x72), (0x400, 0x500)];
/// scheduler.py 24-26; the first thread's id (vm_backend `_first_tid`).
pub const STILL_ACTIVE: u32 = 259;
const DLL_THREAD_ATTACH: u32 = 2;
const DLL_THREAD_DETACH: u32 = 3;
const START_LAST_ERROR: u32 = 187;
pub const FIRST_TID: u64 = 4;
/// CreateThread's flags (vm_backend 3099-3101, 3135).
const CREATE_SUSPENDED: u32 = 0x4;
const STACK_SIZE_PARAM_IS_A_RESERVATION: u32 = 0x10000;

/// A guest thread's host fields (`GuestThread`, scheduler.py 60-71); what the VM's switch reads and writes too is in
/// its run table entry, at `entry(index)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thread {
    pub tid: u64,
    pub index: u64,
    pub teb: u64,
    /// (deallocation base, StackLimit, StackBase)
    pub stack: (u64, u64, u64),
    pub exit_code: u32,
    /// the creating thread's id; 0 for the first
    pub parent: u64,
    /// the start routine, or the first thread's entry
    pub start: u64,
    pub created_suspended: bool,
}

/// The memory offset of run table entry `index`.
pub fn entry(index: u64) -> u64 {
    RUN_TABLE + RT_ENTRIES + RUN_ENTRY * index
}

fn read_u64(mem: &[u8], off: u64) -> u64 {
    u64::from_le_bytes(mem[off as usize..off as usize + 8].try_into().unwrap())
}

fn write_u64(mem: &mut [u8], off: u64, v: u64) {
    mem[off as usize..off as usize + 8].copy_from_slice(&v.to_le_bytes());
}

fn put(mem: &mut [u8], off: u64, bytes: &[u8]) {
    mem[off as usize..off as usize + bytes.len()].copy_from_slice(bytes);
}

/// Guest bytes at `va`, through the layout.
fn write_guest(guest: &mut Guest, va: u64, bytes: &[u8]) -> Result<(), String> {
    let at = guest
        .layout
        .offset(va)
        .ok_or_else(|| format!("guest address 0x{va:x} is not mapped"))?;
    put(guest.memory.as_mut_slice(), at, bytes);
    Ok(())
}

fn read_guest(guest: &Guest, va: u64, n: u64) -> Result<Vec<u8>, String> {
    let at = guest
        .layout
        .offset(va)
        .ok_or_else(|| format!("guest address 0x{va:x} is not mapped"))? as usize;
    Ok(guest.memory.as_slice()[at..at + n as usize].to_vec())
}

/// `VirtualAlloc` in the arena for the host: the base, or the error named.
fn alloc(guest: &mut Guest, address: u64, size: u64, kind: u32, what: &str) -> Result<u64, String> {
    let (va, err) = arena_alloc(guest, address, size, kind)?;
    if err != 0 {
        return Err(format!("{what}: VirtualAlloc error {err}"));
    }
    Ok(va)
}

/// The arena's answer for the host: the base and VirtualAlloc's error (0, or the error a guest would read), or Err
/// when the arena itself fails.
fn arena_alloc(guest: &mut Guest, address: u64, size: u64, kind: u32) -> Result<(u64, u32), String> {
    let mut view = GuestView {
        mem: guest.memory.as_mut_slice(),
        layout: &guest.layout,
    };
    guest
        .arena
        .alloc(&mut view, address, size, kind, PAGE_READWRITE)
        .map_err(|e| e.0)
}

/// `_add_thread` (3328-3344): a new thread's run table entry, after the others (the table's order is the ids', the
/// switch's rotation), ready, its vector slot given (`_vslot`); the first thread is current on vCPU 0.
pub fn add_thread(guest: &mut Guest, thread: Thread) -> Result<u64, String> {
    let mem = guest.memory.as_mut_slice();
    let n = read_u64(mem, RUN_TABLE + RT_COUNT);
    if n == RUN_THREADS {
        return Err(format!("more than {RUN_THREADS} threads in one process"));
    }
    if guest.threads.keys().next_back().is_some_and(|&last| thread.tid <= last) {
        return Err(format!(
            "thread id {} below an earlier thread's: the run table's order is the ids'",
            thread.tid
        ));
    }
    let at = entry(n);
    write_u64(mem, at + E_TID, thread.tid);
    write_u64(mem, at + E_STATUS, READY);
    write_u64(mem, at + E_TEB, thread.teb);
    write_u64(mem, at + E_VCPU, 0);
    write_u64(mem, RUN_TABLE + RT_COUNT, n + 1);
    write_u64(mem, at + E_VSLOT, VECTOR_SLOTS + VECTOR_SLOT * n);
    if n == 0 {
        // `set_current` (3323-3326) for vCPU 0
        write_u64(mem, MONITOR_DATA + CURRENT, 0);
        write_u64(mem, MONITOR_DATA + CURRENT_ENTRY, MONITOR_VA + at);
        write_u64(mem, at + E_VCPU, 1);
    }
    guest.threads.insert(thread.tid, Thread { index: n, ..thread });
    Ok(n)
}

/// `current_tid` (3316-3321): the id of the thread vCPU 0 runs, its current entry's (`w2m_switch` changes it in the
/// VM).
pub fn current_tid(guest: &Guest) -> u64 {
    let mem = guest.memory.as_slice();
    if read_u64(mem, RUN_TABLE + RT_COUNT) == 0 {
        // `_first_tid` (vm_backend 2497): the start's thread id, a child's from the Machine (Sprint 206: it was 4)
        return guest.start.tid;
    }
    let at = read_u64(mem, MONITOR_DATA + CURRENT_ENTRY) - MONITOR_VA;
    read_u64(mem, at + E_TID)
}

/// `_thread_callbacks` (3017-3031): (module base, function) a thread's start and end call, in order: each DLL whose
/// entry runs (load order, then the DLLs loaded after the start), its TLS callbacks and then its DllMain; the program's
/// TLS callbacks last.
pub fn thread_callbacks(guest: &Guest) -> Vec<(u64, u64)> {
    let images = &guest.images;
    let mut dlls: Vec<usize> = guest.init_order.clone();
    dlls.extend((1..images.len()).filter(|i| !guest.init_order.contains(i)));
    let mut out = Vec::new();
    for i in dlls {
        let image = &images[i];
        out.extend(image.pe.tls_callbacks.iter().map(|rva| (image.base, image.base + rva)));
        if let Some(entry) = image.entry() {
            out.push((image.base, entry));
        }
    }
    if let Some(program) = images.first() {
        out.extend(
            program
                .pe
                .tls_callbacks
                .iter()
                .map(|rva| (program.base, program.base + rva)),
        );
    }
    out
}

/// `_thread_static_tls` (3033-3060): a new thread's ThreadLocalStoragePointer: the pointer array and each module's
/// block, copied from the templates as the loader lays them out for the first thread; an array even when no module has
/// static TLS.
pub fn thread_static_tls(guest: &mut Guest) -> Result<u64, String> {
    if guest.images.is_empty() {
        return Ok(0);
    }
    let mut blocks: Vec<Option<Vec<u8>>> = Vec::new();
    let modules: Vec<usize> = std::iter::once(0)
        .chain((1..guest.images.len()).filter(|&i| guest.images[i].pe.tls.is_some()))
        .collect();
    for k in modules {
        let image = &guest.images[k];
        let Some(tls) = &image.pe.tls else {
            blocks.push(None); // the program's slot, when it has no TLS
            continue;
        };
        let relocate = |v: u64| v - image.pe.image_base + image.base;
        let (start, end) = (relocate(tls.start), relocate(tls.end));
        let mut block = read_guest(guest, start, end - start)?;
        block.resize(block.len() + tls.zero_fill as usize, 0);
        blocks.push(Some(block));
    }
    let round = |n: usize| (n as u64 + 15) & !15;
    let array = 8 * TLS_SLOTS as u64; // room for DLLs loaded later
    let size = array + blocks.iter().flatten().map(|b| round(b.len())).sum::<u64>();
    let base = alloc(guest, 0, size, MEM_COMMIT | MEM_RESERVE, "a thread's static TLS")?;
    let mut at = array;
    for (index, block) in blocks.iter().enumerate() {
        let Some(block) = block else { continue };
        write_guest(guest, base + 8 * index as u64, &(base + at).to_le_bytes())?;
        write_guest(guest, base + at, block)?;
        at += round(block.len());
    }
    Ok(base)
}

/// Whether the thread at run table entry `index` has ended: its state, as the switch keeps it (`t.status == "ended"`,
/// vm_backend 3963), not its exit code, which a thread can set to STILL_ACTIVE itself and which THREAD_END sets
/// while the thread still runs its detach callbacks (reviewer-3's Medium 1 of its review of Sprint 218).
pub fn ended(guest: &Guest, index: u64) -> bool {
    read_u64(guest.memory.as_slice(), entry(index) + E_STATUS) == ENDED
}

/// `_late_static_tls` (3953-3978; Sprint 218): static TLS for a DLL loaded after the start, `pe` placed at `base`, as
/// `contracts/windows.loader.late_tls` holds Windows doing it. The DLL takes the next index (1 + the DLLs before it
/// with static TLS), written at its AddressOfIndex. Every thread not ended, those started before the load and the
/// loading one, gets its own block, the template with its zero fill, in its array's slot; the array does not move
/// (each holds `TLS_SLOTS`). A thread started later gets the block from `thread_static_tls`, which reads the images.
/// An ended thread is one whose run table state is ENDED (`ended`).
///
/// The full array and the arena's refusals are refusals, as the frozen host raises them Unimplemented (3961, 3971,
/// 3976); an address not mapped is a failure.
pub fn late_static_tls(guest: &mut Guest, pe: &pe_image::PeImage, base: u64) -> Result<(), ModuleError> {
    use ModuleError::{Failed, NotServed};
    let Some(tls) = &pe.tls else { return Ok(()) };
    let index = 1 + guest.images[1..].iter().filter(|i| i.pe.tls.is_some()).count() as u64;
    if index >= TLS_SLOTS as u64 {
        return Err(NotServed(format!(
            "static TLS index {index}: each thread's array holds {TLS_SLOTS}"
        )));
    }
    let refused = |what: &str, (va, err): (u64, u32)| {
        if err == 0 {
            Ok(va)
        } else {
            Err(NotServed(format!("{what}: VirtualAlloc error {err}")))
        }
    };
    let relocate = |v: u64| v - pe.image_base + base;
    let (start, end) = (relocate(tls.start), relocate(tls.end));
    write_guest(guest, relocate(tls.index), &(index as u32).to_le_bytes()).map_err(Failed)?;
    let mut block = read_guest(guest, start, end - start).map_err(Failed)?;
    block.resize(block.len() + tls.zero_fill as usize, 0);
    let tebs: Vec<u64> = guest
        .threads
        .values()
        .filter(|t| !ended(guest, t.index))
        .map(|t| t.teb)
        .collect();
    for teb in tebs {
        let mut array = u64::from_le_bytes(read_guest(guest, teb + 0x58, 8).map_err(Failed)?.try_into().unwrap());
        if array == 0 {
            // no module had static TLS: the thread's first array
            let answer = arena_alloc(guest, 0, 8 * TLS_SLOTS as u64, MEM_COMMIT | MEM_RESERVE).map_err(Failed)?;
            array = refused("a thread's TLS array", answer)?;
            write_guest(guest, teb + 0x58, &array.to_le_bytes()).map_err(Failed)?;
        }
        let answer = arena_alloc(guest, 0, block.len().max(16) as u64, MEM_COMMIT | MEM_RESERVE).map_err(Failed)?;
        let va = refused("a thread's TLS block", answer)?;
        write_guest(guest, va, &block).map_err(Failed)?;
        write_guest(guest, array + 8 * index, &va.to_le_bytes()).map_err(Failed)?;
    }
    Ok(())
}

/// scheduler.py `_call`: sub rsp, 0x28; mov rcx, rcx; mov edx, edx; xor r8d, r8d; mov rax, fn; call rax; add rsp, 0x28.
fn call(code: &mut Vec<u8>, function: u64, rcx: u64, edx: u32) {
    code.extend([0x48, 0x83, 0xec, 0x28, 0x48, 0xb9]);
    code.extend(rcx.to_le_bytes());
    code.push(0xba);
    code.extend(edx.to_le_bytes());
    code.extend([0x45, 0x31, 0xc0, 0x48, 0xb8]);
    code.extend(function.to_le_bytes());
    code.extend([0xff, 0xd0, 0x48, 0x83, 0xc4, 0x28]);
}

/// scheduler.py `attach_stub`: a new thread's first code: each (module base, function) with DLL_THREAD_ATTACH, the last
/// error 187, the start routine with its parameter, then ExitThread with the routine's return. Entered with RSP = 8
/// mod 16, as a function is.
pub fn attach_stub(callbacks: &[(u64, u64)], start: u64, parameter: u64, exit_thread: u64) -> Vec<u8> {
    let mut code = Vec::new();
    for &(base, function) in callbacks {
        call(&mut code, function, base, DLL_THREAD_ATTACH);
    }
    code.extend([0x65, 0xc7, 0x04, 0x25, 0x68, 0x00, 0x00, 0x00]); // mov dword gs:[0x68], 187
    code.extend(START_LAST_ERROR.to_le_bytes());
    code.extend([0x48, 0x83, 0xec, 0x28, 0x48, 0xb9]);
    code.extend(parameter.to_le_bytes());
    code.extend([0x48, 0xb8]);
    code.extend(start.to_le_bytes());
    code.extend([0xff, 0xd0, 0x89, 0xc1, 0x48, 0xb8]); // call, then ExitThread(eax): never returns
    code.extend(exit_thread.to_le_bytes());
    code.extend([0xff, 0xd0, 0xcc]);
    code
}

/// scheduler.py `detach_stub`: a thread's last code, from ExitThread: each (module base, function) with
/// DLL_THREAD_DETACH, then W2mThreadEnd.
pub fn detach_stub(callbacks: &[(u64, u64)], thread_end: u64) -> Vec<u8> {
    let mut code = Vec::new();
    for &(base, function) in callbacks {
        call(&mut code, function, base, DLL_THREAD_DETACH);
    }
    code.extend([0x48, 0x83, 0xec, 0x28, 0x48, 0xb8]);
    code.extend(thread_end.to_le_bytes());
    code.extend([0xff, 0xd0, 0xcc]);
    code
}

/// `_thread_request`'s THREAD_EXIT_STUB (3201-3202): ExitThread's detach stub in a code page; its address.
pub fn exit_stub(guest: &mut Guest) -> Result<u64, String> {
    let callbacks = thread_callbacks(guest);
    let thread_end = guest
        .import_thunk("KERNEL32.dll!W2mThreadEnd")
        .map_err(|e| format!("{e:?}"))?;
    guest
        .code_page(&detach_stub(&callbacks, thread_end))
        .map_err(|e| format!("{e:?}"))
}

/// `_thread_request`'s THREAD_END (3203-3205): the current thread's final exit code.
pub fn end_thread(guest: &mut Guest, code: u32) -> Result<(), String> {
    let current = current_tid(guest);
    let t = guest
        .threads
        .get_mut(&current)
        .ok_or_else(|| format!("thread {current} ends, and the host holds no such thread"))?;
    t.exit_code = code;
    Ok(())
}

/// `_thread_request`'s THREAD_EXIT_CODE (3206-3211): thread `tid`'s exit code, STILL_ACTIVE while it runs.
pub fn exit_code(guest: &Guest, tid: u32) -> Result<u32, String> {
    guest
        .threads
        .get(&u64::from(tid))
        .map(|t| t.exit_code)
        .ok_or_else(|| format!("a thread request (7) for thread id {tid}, which w2m did not create"))
}

/// `_thread_request`'s THREAD_SUSPEND and THREAD_RESUME (vm_backend 3212-3223; Sprint 215): thread `tid`'s suspend
/// count, the run table entry's word the scheduler reads (scheduler.py `suspend`), raised or lowered. SuspendThread
/// answers the count before it rose, (DWORD)-1 for a thread that has ended; ResumeThread the count before it fell, 0
/// for a thread not suspended or ended (Microsoft Learn, SuspendThread and ResumeThread: "the thread's previous suspend
/// count"). A suspended thread is not chosen by the scheduler (`turn_state`, `_switch`), which on the Rust host's one
/// vCPU is all the frozen host's `_stop_elsewhere` comes to.
pub fn suspend(guest: &mut Guest, tid: u32, resume: bool) -> Result<u64, String> {
    let sub = if resume { 5 } else { 4 };
    let index = guest
        .threads
        .get(&u64::from(tid))
        .map(|t| t.index)
        .ok_or_else(|| format!("a thread request ({sub}) for thread id {tid}, which w2m did not create"))?;
    let mem = guest.memory.as_mut_slice();
    let at = entry(index);
    let count = read_u64(mem, at + E_SUSPEND);
    let ended = ended(guest, index);
    let mem = guest.memory.as_mut_slice();
    Ok(match (resume, ended, count) {
        (false, true, _) => u64::MAX,
        (false, false, n) => {
            write_u64(mem, at + E_SUSPEND, n + 1);
            n
        }
        (true, true, _) | (true, false, 0) => 0,
        (true, false, n) => {
            write_u64(mem, at + E_SUSPEND, n - 1);
            n
        }
    })
}

/// CONTEXT's parts (scheduler.py 35-36): ContextFlags' bits, and the x64 user-mode selectors (rows context: CS 0x33,
/// SS 0x2B).
const CONTEXT_CONTROL: u32 = 0x1;
const CONTEXT_INTEGER: u32 = 0x2;
const CONTEXT_SEGMENTS: u32 = 0x4;
const CONTEXT_FLOATING_POINT: u32 = 0x8;
const SELECTORS: [(usize, u16); 6] = [(56, 0x33), (58, 0x2B), (60, 0x2B), (62, 0x53), (64, 0x2B), (66, 0x2B)];

/// `_thread_request`'s THREAD_CONTEXT (vm_backend 3224-3253; Sprint 215): GetThreadContext of thread `tid` into the
/// caller's CONTEXT at `buffer` (1232 bytes, ContextFlags at 48), the parts the flags name written as
/// scheduler.py `write_context` writes them, from the thread's saved state in its run table entry: the registers,
/// EFLAGS rebuilt from the flag registers and the monitor's lazy PF and AF (`undefined` flags left clear), IF set and
/// DF from the monitor, MXCSR and the x87 control word, XMM0-15 from its vector slot. A thread blocked in a wait answers
/// RIP inside ntdll's page (contracts/windows.thread.running_context, row context.waiting: Windows' NtWait* stub), as
/// the frozen host does. The calling thread's own context is refused: its saved state is not current, and
/// GetThreadContext of the calling thread is not in the rows.
pub fn thread_context(guest: &mut Guest, tid: u32, buffer: u64) -> Result<u64, String> {
    use crate::cases::{DF_BYTE, FLAG_BIT, FLAG_REG};
    let t = guest
        .threads
        .get(&u64::from(tid))
        .ok_or_else(|| format!("a thread request (6) for thread id {tid}, which w2m did not create"))?;
    if u64::from(tid) == current_tid(guest) {
        return Err(format!(
            "GetThreadContext of the calling thread {tid}, which is not in the rows"
        ));
    }
    let mem = guest.memory.as_slice();
    let at = entry(t.index);
    let x: Vec<u64> = (0..31).map(|i| read_u64(mem, at + E_X + 8 * i)).collect();
    // the monitor ranges the thread keeps: lazy AF and DF (0x20-0x40), MXCSR (0x50), the x87 control word (0x70)
    let monitor = |range: usize, off: u64| at + E_MONITOR + E_MONITOR_AT[range] + off;
    let (af_a, af_b, af_r) = (
        read_u64(mem, monitor(0, 0)),
        read_u64(mem, monitor(0, 8)),
        read_u64(mem, monitor(0, 16)),
    );
    let df = u32::from(mem[monitor(0, DF_BYTE - 0x20) as usize] & 1);
    let mxcsr = read_u64(mem, monitor(1, 0)) as u32;
    let fcw = read_u64(mem, monitor(2, 0)) as u16;
    let undefined = x[REG_UNDEFINED_FLAGS as usize];
    let defined = |f: char| undefined >> FLAG_BIT.iter().find(|(k, _)| *k == f).unwrap().1 & 1 == 0;
    let mut eflags = 0x202 | df << 10;
    for (f, bit) in [('C', 0), ('Z', 6), ('S', 7), ('O', 11)] {
        let reg = FLAG_REG.iter().find(|(k, _)| *k == f).unwrap().1;
        if defined(f) && x[usize::from(reg)] != 0 {
            eflags |= 1 << bit;
        }
    }
    if defined('P') && (af_r & 0xFF).count_ones() % 2 == 0 {
        eflags |= 1 << 2;
    }
    if defined('A') && (af_a ^ af_b ^ af_r) & 0x10 != 0 {
        eflags |= 1 << 4;
    }
    let ntdll = guest.process.ntdll;
    let rip = if read_u64(mem, at + E_STATUS) == BLOCKED && ntdll != 0 {
        ntdll + 0x800
    } else {
        x[17]
    };
    let slot = read_u64(mem, at + E_VSLOT) as usize;
    let xmm = mem[slot..slot + 256].to_vec();
    let mut buf = read_guest(guest, buffer, 1232)?;
    let flags = u32::from_le_bytes(buf[48..52].try_into().unwrap());
    let put = |buf: &mut Vec<u8>, off: usize, bytes: &[u8]| buf[off..off + bytes.len()].copy_from_slice(bytes);
    if flags & CONTEXT_CONTROL != 0 {
        for (off, sel) in [SELECTORS[0], SELECTORS[5]] {
            put(&mut buf, off, &sel.to_le_bytes());
        }
        put(&mut buf, 68, &eflags.to_le_bytes());
        put(&mut buf, 120 + 8 * 4, &x[4].to_le_bytes());
        put(&mut buf, 248, &rip.to_le_bytes());
    }
    if flags & CONTEXT_INTEGER != 0 {
        for (i, r) in x.iter().take(16).enumerate().filter(|(i, _)| *i != 4) {
            put(&mut buf, 120 + 8 * i, &r.to_le_bytes());
        }
    }
    if flags & CONTEXT_SEGMENTS != 0 {
        for (off, sel) in &SELECTORS[1..5] {
            put(&mut buf, *off, &sel.to_le_bytes());
        }
    }
    if flags & CONTEXT_FLOATING_POINT != 0 {
        put(&mut buf, 52, &mxcsr.to_le_bytes());
        put(&mut buf, 256, &fcw.to_le_bytes());
        put(&mut buf, 256 + 24, &mxcsr.to_le_bytes());
        put(&mut buf, 256 + 28, &0xFFFFu32.to_le_bytes()); // MxCsr_Mask, the runner's
        put(&mut buf, 256 + 160, &xmm);
    }
    write_guest(guest, buffer, &buf)?;
    Ok(1)
}

/// `GuestThread.status`'s setter (scheduler.py 83-89): the entry's status, and the header's count of blocked threads,
/// which threads.c `w2m_signal` reads.
pub fn set_status(guest: &mut Guest, index: u64, status: u64) {
    let mem = guest.memory.as_mut_slice();
    let at = entry(index);
    let was = read_u64(mem, at + E_STATUS);
    if (was == BLOCKED) != (status == BLOCKED) {
        let blocked = read_u64(mem, RUN_TABLE + RT_BLOCKED);
        let blocked = if status == BLOCKED { blocked + 1 } else { blocked - 1 };
        write_u64(mem, RUN_TABLE + RT_BLOCKED, blocked);
    }
    write_u64(mem, at + E_STATUS, status);
}

/// What a hand-over leaves to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Switch {
    /// every thread has ended: the process ends with the last one's exit code
    Ended(u32),
    /// a thread goes on at the dispatcher with the registers `schedule` left in its `x`; the vector slots to swap,
    /// (outgoing, incoming), when it is another thread
    Go { vectors: Option<(u64, u64)> },
    /// no thread is ready in a process of a machine (`IDLE`, vm_backend 3565-3571): the turn goes to the machine, and
    /// `resume_idle` goes on from here
    Idle,
}

unsafe extern "C" {
    /// `mach/mach_time.h`
    fn mach_absolute_time() -> u64;
    fn mach_timebase_info(info: *mut [u32; 2]) -> i32;
}

/// `_counter_now` (vm_backend 4886-4895): the guest's 10 MHz counter now: on the instruction clock the count and the
/// warp; else the host's monotonic clock in 100 ns units (`monotonic_count`, kernel32.py 36-38, which is
/// `mach_absolute_time` on macOS, the counter the guest reads as CNTVCT_EL0).
pub fn counter_now(guest: &Guest) -> u64 {
    let mem = guest.memory.as_slice();
    if read_u64(mem, MONITOR_DATA + CLOCK_MODE) == 2 {
        let executed = u128::from(read_u64(mem, MONITOR_DATA));
        let rate = u128::from(read_u64(mem, MONITOR_DATA + CLOCK_RATE));
        let warp = u128::from(read_u64(mem, RUN_TABLE + CLOCK_WARP));
        return (executed / rate * 10_000_000 + executed % rate * 10_000_000 / rate + warp) as u64;
    }
    let mut base = [0u32; 2];
    let ticks = unsafe {
        mach_timebase_info(&mut base);
        mach_absolute_time()
    };
    (u128::from(ticks) * u128::from(base[0]) / u128::from(base[1]) / 100) as u64
}

/// `_catch_up` (vm_backend 3680-3688): on the instruction clock, in a machine of `processes`, a process behind the
/// machine's clock `machine` is moved to it by its warp, as time passed for it while it waited; at its start too
/// (`VM.steps`, 5055-5080; Sprint 199, reviewer-2's High 1 of its review of 196-198).
pub fn catch_up(guest: &mut Guest, machine: u64, processes: usize) {
    if read_u64(guest.memory.as_slice(), MONITOR_DATA + CLOCK_MODE) != 2 {
        return;
    }
    let lag = crate::machine::catch_up(counter_now(guest), machine, processes);
    let mem = guest.memory.as_mut_slice();
    let warp = read_u64(mem, RUN_TABLE + CLOCK_WARP);
    write_u64(mem, RUN_TABLE + CLOCK_WARP, warp + lag);
}

/// `_wake_expired` (4897-4905): every blocked thread whose deadline has passed is ready.
pub fn wake_expired(guest: &mut Guest) {
    let now = counter_now(guest);
    let mem = guest.memory.as_slice();
    let expired: Vec<u64> = guest
        .threads
        .values()
        .filter(|t| {
            let d = read_u64(mem, entry(t.index) + E_DEADLINE);
            read_u64(mem, entry(t.index) + E_STATUS) == BLOCKED && d != u64::MAX && d <= now
        })
        .map(|t| t.index)
        .collect();
    for index in expired {
        set_status(guest, index, READY);
    }
}

/// What a process reports of itself at the end of its turn (`VM.ready`, `waits_with_deadline`, vm_backend 3653-3690):
/// a thread ready and not suspended, and a thread blocked with a deadline.
pub fn turn_state(guest: &Guest) -> crate::turns::State {
    let mem = guest.memory.as_slice();
    let word = |t: &Thread, off: u64| read_u64(mem, entry(t.index) + off);
    let deadline = guest
        .threads
        .values()
        .filter(|t| word(t, E_STATUS) == BLOCKED && word(t, E_SUSPEND) == 0)
        .map(|t| word(t, E_DEADLINE))
        .min()
        .unwrap_or(u64::MAX);
    crate::turns::State {
        clock: counter_now(guest),
        ready: guest
            .threads
            .values()
            .any(|t| word(t, E_STATUS) == READY && word(t, E_SUSPEND) == 0),
        timed: deadline != u64::MAX,
        deadline,
        blocked: guest.threads.values().any(|t| word(t, E_STATUS) == BLOCKED),
    }
}

/// `warp_to_deadline` (vm_backend 3614-3628): on the instruction clock with no thread ready, the clock moved to the
/// earliest deadline of a blocked, unsuspended thread (the lowest tid among equals) and that thread made ready; false
/// when no thread waits with a deadline or the clock is another.
pub fn warp_to_deadline(guest: &mut Guest) -> bool {
    let mem = guest.memory.as_slice();
    if read_u64(mem, MONITOR_DATA + CLOCK_MODE) != 2 {
        return false;
    }
    let word = |t: &Thread, off: u64| read_u64(mem, entry(t.index) + off);
    let first = guest
        .threads
        .values()
        .filter(|t| word(t, E_STATUS) == BLOCKED && word(t, E_SUSPEND) == 0 && word(t, E_DEADLINE) != u64::MAX)
        .min_by_key(|t| (word(t, E_DEADLINE), t.tid))
        .map(|t| (t.index, word(t, E_DEADLINE)));
    let Some((index, deadline)) = first else {
        return false;
    };
    let now = counter_now(guest);
    if deadline > now {
        let mem = guest.memory.as_mut_slice();
        let warp = read_u64(mem, RUN_TABLE + CLOCK_WARP);
        write_u64(mem, RUN_TABLE + CLOCK_WARP, warp + deadline - now);
    }
    set_status(guest, index, READY);
    true
}

/// `_resume_idle` (3632-3652) after the machine gave the turn back: expired waits woken; with none ready, a blocked
/// thread with a deadline runs to check it (the instruction clock's warp is not carried, and refuses by name); then
/// `_switch` from the thread that went idle.
pub fn resume_idle(guest: &mut Guest, x: &mut [u64; 31], cur_tid: u64, bits: u32) -> Result<Switch, String> {
    wake_expired(guest);
    let mem = guest.memory.as_slice();
    let word = |t: &Thread, off: u64| read_u64(mem, entry(t.index) + off);
    if !guest
        .threads
        .values()
        .any(|t| word(t, E_STATUS) == READY && word(t, E_SUSPEND) == 0)
    {
        let timed = guest
            .threads
            .values()
            .find(|t| word(t, E_STATUS) == BLOCKED && word(t, E_SUSPEND) == 0 && word(t, E_DEADLINE) != u64::MAX)
            .map(|t| t.index);
        // the instruction clock warps to the earliest deadline (Sprint 199); the wall clock runs the first timed
        // waiter to check its wait
        if !warp_to_deadline(guest)
            && let Some(index) = timed
        {
            set_status(guest, index, READY);
        }
    }
    let cur = guest
        .threads
        .get(&cur_tid)
        .ok_or_else(|| format!("thread {cur_tid} went idle, and the host holds no such thread"))?
        .clone();
    switch(guest, x, &cur, bits)
}

/// `_schedule` (3500-3519) for `hvc #12`: the current thread (its registers in `x`, at an instruction boundary) hands
/// vCPU 0 over with state.schedule's `bits`. SCHEDULE_END: it has ended, and every blocked thread is made ready (its
/// handle signals; waiters check again). SCHEDULE_RETRY: a thread `w2m_block` blocked keeps its wait's deadline. Then
/// `_switch`, unless every thread has ended, which leaves in `x` the registers of the thread that goes on.
pub fn schedule(guest: &mut Guest, x: &mut [u64; 31], bits: u32) -> Result<Switch, String> {
    let current = current_tid(guest);
    let cur = guest
        .threads
        .get(&current)
        .ok_or_else(|| format!("thread {current} hands over, and the host holds no such thread"))?
        .clone();
    let word = |guest: &Guest, t: &Thread, off: u64| read_u64(guest.memory.as_slice(), entry(t.index) + off);
    if bits & SCHEDULE_END != 0 {
        set_status(guest, cur.index, ENDED);
        let blocked: Vec<u64> = guest
            .threads
            .values()
            .filter(|t| word(guest, t, E_STATUS) == BLOCKED)
            .map(|t| t.index)
            .collect();
        for index in blocked {
            set_status(guest, index, READY);
        }
    } else if bits & SCHEDULE_RETRY != 0 && word(guest, &cur, E_STATUS) == BLOCKED {
        let deadline = u64::from_le_bytes(read_guest(guest, cur.teb + SCRATCH_OFF, 8)?.try_into().unwrap());
        write_u64(guest.memory.as_mut_slice(), entry(cur.index) + E_DEADLINE, deadline);
    }
    if guest.threads.values().all(|t| word(guest, t, E_STATUS) == ENDED) {
        return Ok(Switch::Ended(cur.exit_code)); // the last thread ended: the process ends with it
    }
    switch(guest, x, &cur, bits)
}

/// `_switch` (3521-3580) for vCPU 0 of one process, the thread not yet put down: the next ready, unsuspended thread
/// after `cur` in id order, round robin, `cur` last. With none ready the host would warp the instruction clock to a
/// deadline or sleep 1 ms for a timed wait; neither is carried yet, and the switch refuses by name.
fn switch(guest: &mut Guest, x: &mut [u64; 31], cur: &Thread, bits: u32) -> Result<Switch, String> {
    let mem = guest.memory.as_slice();
    let word = |t: &Thread, off: u64| read_u64(mem, entry(t.index) + off);
    let order: Vec<&Thread> = guest.threads.values().collect();
    let i = order.iter().position(|t| t.tid == cur.tid).unwrap();
    let rotation = order[i + 1..].iter().chain(order[..=i].iter());
    let next = rotation
        .clone()
        .find(|t| word(t, E_STATUS) == READY && word(t, E_SUSPEND) == 0)
        .map(|t| (*t).clone());
    if next.is_none() && guest.link.is_some() {
        return Ok(Switch::Idle);
    }
    let Some(next) = next else {
        let timed = rotation
            .filter(|t| word(t, E_STATUS) == BLOCKED && word(t, E_SUSPEND) == 0 && word(t, E_DEADLINE) != u64::MAX)
            .count();
        return Err(if timed == 0 {
            "every thread waits with no timeout and nothing can wake one (a deadlock)".into()
        } else {
            format!(
                "no thread ready after thread {}'s hand-over and {timed} waiting with a deadline: the warp to a \
                 deadline is not served by the Rust host yet",
                cur.tid
            )
        });
    };
    if bits & SCHEDULE_ANSWER != 0 {
        x[0] = u64::from(next.tid != cur.tid); // SwitchToThread: nonzero when another thread runs
    }
    if next.tid == cur.tid {
        return Ok(Switch::Go { vectors: None });
    }
    // `_save_thread` (3467-3479)
    let mem = guest.memory.as_mut_slice();
    let (from, to) = (entry(cur.index), entry(next.index));
    let (executed, pauses) = (read_u64(mem, MONITOR_DATA), read_u64(mem, MONITOR_DATA + PAUSE_COUNT));
    let (begun, begun_pauses) = (
        read_u64(mem, MONITOR_DATA + SLICE_COUNT),
        read_u64(mem, MONITOR_DATA + SLICE_PAUSES),
    );
    write_u64(
        mem,
        from + E_INSTRUCTIONS,
        read_u64(mem, from + E_INSTRUCTIONS) + executed - begun,
    );
    write_u64(
        mem,
        from + E_PAUSES,
        read_u64(mem, from + E_PAUSES) + pauses - begun_pauses,
    );
    write_u64(mem, MONITOR_DATA + SLICE_COUNT, executed);
    write_u64(mem, MONITOR_DATA + SLICE_PAUSES, pauses);
    for (i, r) in x.iter().enumerate() {
        write_u64(mem, from + E_X + 8 * i as u64, *r);
    }
    let state = MONITOR_SIZE + STATE_OFF;
    for (p, (a, b)) in E_MONITOR_AT.iter().zip(PER_THREAD_MONITOR) {
        let data = mem[(MONITOR_DATA + a) as usize..(MONITOR_DATA + b) as usize].to_vec();
        put(mem, from + E_MONITOR + p, &data);
    }
    let data = mem[state as usize..(state + STATE_SIZE) as usize].to_vec();
    put(mem, from + E_STATE, &data);
    write_u64(mem, from + E_VCPU, 0);
    // `set_current` (3323-3326)
    write_u64(mem, MONITOR_DATA + CURRENT, next.index);
    write_u64(mem, MONITOR_DATA + CURRENT_ENTRY, MONITOR_VA + to);
    // `_load_thread` (3487-3498)
    if !matches!(read_u64(mem, to + E_VCPU), 0 | 1) {
        return Err(format!(
            "thread {} loaded on vCPU 0 while another vCPU has it",
            next.tid
        ));
    }
    for (p, (a, b)) in E_MONITOR_AT.iter().zip(PER_THREAD_MONITOR) {
        let data = mem[(to + E_MONITOR + p) as usize..(to + E_MONITOR + p + b - a) as usize].to_vec();
        put(mem, MONITOR_DATA + a, &data);
    }
    let data = mem[(to + E_STATE) as usize..(to + E_STATE + STATE_SIZE) as usize].to_vec();
    put(mem, state, &data);
    write_u64(mem, MONITOR_DATA + SLICE_COUNT, executed);
    write_u64(mem, MONITOR_DATA + SLICE_PAUSES, pauses);
    write_u64(mem, to + E_VCPU, 1);
    for (i, r) in x.iter_mut().enumerate() {
        *r = read_u64(mem, to + E_X + 8 * i as u64);
    }
    let vectors = (read_u64(mem, from + E_VSLOT), read_u64(mem, to + E_VSLOT));
    Ok(Switch::Go { vectors: Some(vectors) })
}

/// `_stack_grow` (vm_backend 3161-3188; Sprint 213): thread `tid`'s stack committed down to `address`'s page, as
/// Windows does on a touch of its guard (contracts/windows.memory.created_thread_stack, rows default.grow and
/// default.grown.*): the pages from it to StackLimit committed read-write and mapped, StackLimit (TEB+0x10) moved to
/// it, three PAGE_GUARD pages below. Only a page in the guard grows the stack; a `probe` (the runtime placing a frame
/// below StackLimit) grows it page by page as successive touches would. 0, an access violation, when the address is
/// outside the reservation, below the guard on a touch (row skip), or would bring the guard to the reservation's last
/// page; 1 when grown. The main thread's stack, committed whole in the process area, is not grown.
///
/// The frozen host checks the last-page bound inside its loop, so a probe that reaches it commits pages and returns 0
/// with StackLimit unmoved; here the bound is checked first and nothing changes (the user's rule of 2026-10-29).
pub fn stack_grow(guest: &mut Guest, tid: u64, address: u64, probe: bool) -> Result<u64, String> {
    let Some(thread) = guest.threads.get(&tid) else {
        return Ok(0);
    };
    let (dealloc, _, top) = thread.stack;
    let teb = thread.teb;
    if !guest.arena.reservations.contains_key(&dealloc) {
        return Ok(0);
    }
    let mut limit = u64::from_le_bytes(read_guest(guest, teb + 0x10, 8)?.try_into().unwrap());
    let page = address & !(PAGE - 1);
    let guard = STACK_GUARD_PAGES * PAGE;
    if !(dealloc <= page && page < limit)
        || (!probe && page < limit - guard)
        || page.saturating_sub(guard) < dealloc + PAGE
    {
        return Ok(0);
    }
    while limit > page {
        let new = if probe { limit - PAGE } else { page };
        let mut view = GuestView {
            mem: guest.memory.as_mut_slice(),
            layout: &guest.layout,
        };
        guest.arena.commit(
            &mut view as &mut dyn GuestPages,
            dealloc,
            new,
            limit - new,
            PAGE_READWRITE,
        );
        guest.map_pages(new, limit, true)?;
        let mut view = GuestView {
            mem: guest.memory.as_mut_slice(),
            layout: &guest.layout,
        };
        guest.arena.commit(
            &mut view as &mut dyn GuestPages,
            dealloc,
            new - guard,
            guard,
            PAGE_READWRITE | PAGE_GUARD,
        );
        limit = new;
    }
    write_guest(guest, teb + 0x10, &limit.to_le_bytes())?;
    if let Some(t) = guest.threads.get_mut(&tid) {
        t.stack = (dealloc, limit, top);
    }
    Ok(1)
}

/// `_create_thread` (3094-3150): CreateThread's host half: a thread id, a stack, a TEB, static TLS, the attach stub and
/// the first state. Returns the thread id.
pub fn create_thread(
    guest: &mut Guest,
    start: u64,
    parameter: u64,
    flags: u32,
    stack_size: u64,
) -> Result<u64, String> {
    let tid = guest.threads.keys().next_back().copied().unwrap_or(0) + 4;
    let program = guest.images.first().map(|i| (i.pe.stack_reserve, i.pe.stack_commit));
    // STACK_SIZE_PARAM_IS_A_RESERVATION: the size asked for, smaller than the image's or not; otherwise the image's
    // SizeOfStackReserve
    let reserve = if flags & STACK_SIZE_PARAM_IS_A_RESERVATION != 0 && stack_size != 0 {
        stack_size
    } else {
        program.map_or(0x10_0000, |p| p.0)
    };
    let reserve = (reserve.max(0x1_0000) + 0xFFFF) & !0xFFFF;
    let dealloc = alloc(
        guest,
        0,
        reserve,
        MEM_RESERVE,
        &format!("a thread's stack of 0x{reserve:x} bytes"),
    )?;
    // a created thread's stack as Windows lays it out (contracts/windows.memory.created_thread_stack): the initial
    // commit at the top, three PAGE_GUARD pages below StackLimit, the rest reserved; the guard and reserved pages
    // unmapped
    let top = dealloc + reserve;
    let commit = if stack_size != 0 && flags & STACK_SIZE_PARAM_IS_A_RESERVATION == 0 {
        stack_size
    } else {
        program.map_or(0x1000, |p| p.1)
    };
    let commit = ((commit + 0xFFF) & !0xFFF)
        .max(0x1000)
        .min(reserve - (2 + STACK_GUARD_PAGES) * PAGE);
    let limit = top - commit;
    alloc(guest, limit, commit, MEM_COMMIT, "a thread's stack commit")?;
    {
        let mut view = GuestView {
            mem: guest.memory.as_mut_slice(),
            layout: &guest.layout,
        };
        guest.arena.commit(
            &mut view as &mut dyn GuestPages,
            dealloc,
            limit - STACK_GUARD_PAGES * PAGE,
            STACK_GUARD_PAGES * PAGE,
            PAGE_READWRITE | PAGE_GUARD,
        );
    }
    guest.map_pages(dealloc, limit, false)?;
    let teb = alloc(guest, 0, 0x4000, MEM_COMMIT | MEM_RESERVE, "a thread's TEB")?; // TEB, then TlsExpansionSlots
    let static_tls = thread_static_tls(guest)?;
    let peb = PROCESS_VA + PEB_OFF;
    let process_id = guest.process.process_id;
    for (off, value) in [
        (0x08, top),
        (0x10, limit),
        (0x30, teb),
        (0x40, process_id),
        (0x48, tid),
        (0x58, static_tls),
        (0x60, peb),
        (0x1478, dealloc),
        (0x1780, teb + 0x2000),
    ] {
        write_guest(guest, teb + off, &u64::to_le_bytes(value))?;
    }
    let callbacks = thread_callbacks(guest);
    let exit_thread = guest
        .import_thunk("KERNEL32.dll!ExitThread")
        .map_err(|e| format!("{e:?}"))?;
    let stub = guest
        .code_page(&attach_stub(&callbacks, start, parameter, exit_thread))
        .map_err(|e| format!("{e:?}"))?;
    let mut x = [0u64; 31];
    x[4] = top - 0x48; // RSP = 8 mod 16, as at a function's entry
    x[17] = stub;
    x[REG_UNDEFINED_FLAGS as usize] = 0x3F;
    x[28] = MONITOR_VA + MONITOR_DATA;
    let parent = current_tid(guest);
    let index = add_thread(
        guest,
        Thread {
            tid,
            index: 0,
            teb,
            stack: (dealloc, limit, top),
            exit_code: STILL_ACTIVE,
            parent,
            start,
            created_suspended: flags & CREATE_SUSPENDED != 0,
        },
    )?;
    let at = entry(index);
    let mem = guest.memory.as_mut_slice();
    for (i, r) in x.iter().enumerate() {
        write_u64(mem, at + E_X + 8 * i as u64, *r);
    }
    // the per-thread monitor words: MXCSR 0x1F80 and the x87 control word 0x27F, the rest zero
    for (p, (a, b)) in E_MONITOR_AT.iter().zip(PER_THREAD_MONITOR) {
        let mut data = vec![0u8; (b - a) as usize];
        if a == MXCSR_OFF {
            data[0..4].copy_from_slice(&0x1F80u32.to_le_bytes());
        }
        if a == FCW_OFF {
            data[0..2].copy_from_slice(&0x27Fu16.to_le_bytes());
        }
        put(mem, at + E_MONITOR + p, &data);
    }
    let mut state = vec![0u8; STATE_SIZE as usize];
    state[152..156].copy_from_slice(&0x1F80u32.to_le_bytes());
    state[156..158].copy_from_slice(&0x27Fu16.to_le_bytes());
    state[176..184].copy_from_slice(&teb.to_le_bytes());
    state[184..192].copy_from_slice(&teb.to_le_bytes());
    put(mem, at + E_STATE, &state);
    write_u64(mem, at + E_SUSPEND, u64::from(flags & CREATE_SUSPENDED != 0));
    // `_write_vectors` (3352-3356): as a Windows thread starts, XMM clear, MXCSR 0x1F80
    let vslot = read_u64(mem, at + E_VSLOT);
    put(mem, vslot, &[0u8; 256]);
    write_u64(mem, vslot + 256, fpcr_for(0x1F80));
    write_u64(mem, vslot + 264, 0);
    guest.arena.stacks.push((dealloc, limit, top));
    set_budget(guest); // two threads: time slices from now on
    Ok(tid)
}

/// `_set_budget` (3284-3289): the step budget the block entries check: the run's limit, or with more than one thread a
/// time slice.
pub fn set_budget(guest: &mut Guest) {
    // a process of a machine shares the vCPU's time with the others too (Sprint 197)
    let shared = guest.threads.len() > 1 || guest.link.is_some();
    let mem = guest.memory.as_mut_slice();
    let (executed, max_steps) = (read_u64(mem, MONITOR_DATA), read_u64(mem, MONITOR_DATA + RUN_END));
    let limit = if shared {
        max_steps.min(executed + TIME_SLICE)
    } else {
        max_steps
    };
    write_u64(mem, MONITOR_DATA + 8, limit);
}

/// `_slice_until` (2756-2780) for one vCPU, one process, no lockstep, forced stop or pending wake: 0 when the current
/// thread is suspended or another is ready; the count at which the earliest deadline of a blocked thread falls, on the
/// instruction clock (0 on another clock); else ~0.
pub fn slice_until(guest: &Guest) -> u64 {
    // another process ready or waiting with a deadline needs the host at each slice's end (`_other_process_due`,
    // 2750-2754; Sprint 197)
    if guest.link.as_ref().is_some_and(|l| l.others_due(counter_now(guest))) {
        return 0;
    }
    let mem = guest.memory.as_slice();
    let current = current_tid(guest);
    let word = |t: &Thread, off: u64| read_u64(mem, entry(t.index) + off);
    if let Some(cur) = guest.threads.get(&current)
        && word(cur, E_SUSPEND) != 0
    {
        return 0;
    }
    let others: Vec<&Thread> = guest
        .threads
        .values()
        .filter(|t| t.tid != current && word(t, E_SUSPEND) == 0)
        .collect();
    if others.iter().any(|t| word(t, E_STATUS) == READY) {
        return 0;
    }
    let deadline = others
        .iter()
        .filter(|t| word(t, E_STATUS) == BLOCKED && word(t, E_DEADLINE) != u64::MAX)
        .map(|t| word(t, E_DEADLINE))
        .min();
    let Some(deadline) = deadline else {
        return u64::MAX;
    };
    if read_u64(mem, MONITOR_DATA + CLOCK_MODE) != 2 {
        return 0;
    }
    let (rate, warp) = (
        read_u64(mem, MONITOR_DATA + CLOCK_RATE),
        read_u64(mem, RUN_TABLE + CLOCK_WARP),
    );
    // the count whose clock reaches the deadline, rounded down
    let ticks = deadline as i128 - warp as i128;
    if ticks <= 0 {
        return 0;
    }
    let (ticks, rate) = (ticks as u128, u128::from(rate));
    (ticks / 10_000_000 * rate + ticks % 10_000_000 * rate / 10_000_000) as u64
}

#[cfg(test)]
mod tests {
    /// A guest of qpc.exe with one created thread; the runtime image clang builds, so the tests that use it run on the
    /// Mac.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn guest_with_a_thread() -> (crate::guest::Guest, u64) {
        use crate::guest::{Guest, vm_compiler_symbols};
        use crate::process::Start;
        use guest_runtime::layout::{RUNTIME_IMAGE_VA, runtime_defines};
        use std::path::Path;

        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let data = std::fs::read(root.join("tests/data/timed_waits/qpc.exe")).unwrap();
        let loaded = pe_loader::load(&data, None, "probe.exe").unwrap();
        let (_, image) = guest_runtime::build(&root, &runtime_defines(), RUNTIME_IMAGE_VA).unwrap();
        let services = guest_runtime::services::services(&guest_runtime::texts(&root).unwrap()).unwrap();
        let table = std::fs::read_to_string(root.join("tools/vm_compiler/stencil_table.h")).unwrap();
        let symbols = vm_compiler_symbols(&table).unwrap();
        let mut g = Guest::new(
            &loaded,
            &image,
            &services,
            &Start::console("probe.exe"),
            &std::collections::BTreeSet::new(),
            &symbols,
        )
        .unwrap();
        // two threads: the first takes the current thread's id (4) in a guest no run has started, so the second is the
        // one a test reads and changes as another thread
        super::create_thread(&mut g, loaded.entry, 0, 0, 0).unwrap();
        let tid = super::create_thread(&mut g, loaded.entry, 0, 0, 0).unwrap();
        (g, tid)
    }

    /// A thread has ended by its run table state, not its exit code (Sprint 218; reviewer-3's Medium 1): an ended thread
    /// whose code is 259, STILL_ACTIVE's value, has ended; a running one whose code THREAD_END has set has not.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn a_thread_has_ended_by_its_state_not_its_exit_code() {
        use super::{ENDED, READY, STILL_ACTIVE, ended, set_status};

        let (mut g, tid) = guest_with_a_thread();
        let index = g.threads[&tid].index;
        assert!(!ended(&g, index));
        g.threads.get_mut(&tid).unwrap().exit_code = 7;
        assert!(!ended(&g, index), "THREAD_END's code set, the thread still running");
        set_status(&mut g, index, ENDED);
        g.threads.get_mut(&tid).unwrap().exit_code = STILL_ACTIVE;
        assert!(
            ended(&g, index),
            "ExitThread(259): ended, its code reading STILL_ACTIVE"
        );
        set_status(&mut g, index, READY);
        assert!(!ended(&g, index));
    }

    /// HOST_UNIMPLEMENTED (request 4) names the service by its API id and form and keeps the call's return address, the
    /// word at the saved RSP, for the record; an RSP that is not mapped is a failure. HOST_CLOCK (request 3) answers the
    /// host's monotonic count in mode 0, `counter_now` (Sprint 219; reviewer-3's Medium 2 of its review of Sprint 219).
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn requests_three_and_four_answer_as_the_frozen_host() {
        use crate::host_request::{HostError, unimplemented};
        use guest_runtime::layout::{HEAP_VA, MONITOR_SIZE, STATE_OFF};

        let (mut g, _) = guest_with_a_thread();
        let names = guest_runtime::services::api_names(&g.services).unwrap();
        let at = g.layout.offset(HEAP_VA).unwrap() as usize;
        let slot = (MONITOR_SIZE + STATE_OFF + 8 * 4) as usize;
        let mem = g.memory.as_mut_slice();
        mem[slot..slot + 8].copy_from_slice(&HEAP_VA.to_le_bytes());
        mem[at..at + 8].copy_from_slice(&0x1_4000_1234u64.to_le_bytes());
        let stop = unimplemented(&mut g, 2, 7);
        let service = format!("{} argument form 0x7", names[1]);
        assert!(matches!(&stop, HostError::NotServed(m) if *m == service), "{stop:?}");
        assert_eq!(g.unimplemented, Some((service, 0x1_4000_1234)));
        g.memory.as_mut_slice()[slot..slot + 8].copy_from_slice(&0x10u64.to_le_bytes());
        g.unimplemented = None;
        assert!(matches!(unimplemented(&mut g, 2, 7), HostError::Failed(_)));
        assert_eq!(g.unimplemented, None);
        let (a, b) = (super::counter_now(&g), super::counter_now(&g));
        assert!(a > 0 && b >= a, "the host's monotonic count: {a}, then {b}");
    }

    /// A late DLL whose index would pass each thread's array (`TLS_SLOTS`) is refused, a request not served, as the
    /// frozen host raises it Unimplemented (vm_backend 3961; reviewer-3's Medium 3 of its review of Sprint 218).
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn a_late_dll_past_the_tls_array_is_refused() {
        use super::late_static_tls;
        use crate::modules::ModuleError;

        let (mut g, _) = guest_with_a_thread();
        let mut pe = g.images[0].pe.clone();
        pe.tls = Some(pe_image::Tls {
            start: pe.image_base,
            end: pe.image_base,
            index: pe.image_base,
            zero_fill: 0,
        });
        for n in 0..pe_loader::TLS_SLOTS - 1 {
            g.images.push(pe_loader::Image {
                name: format!("late{n}.dll"),
                pe: pe.clone(),
                base: 0,
            });
        }
        assert_eq!(
            late_static_tls(&mut g, &pe, 0),
            Err(ModuleError::NotServed(
                "static TLS index 64: each thread's array holds 64".into()
            ))
        );
    }

    /// SuspendThread and ResumeThread answer the previous count (Sprint 215; Microsoft Learn: "the thread's previous
    /// suspend count"), an ended thread (DWORD)-1 and 0; GetThreadContext of a suspended thread writes RIP, RSP, the
    /// registers and EFLAGS rebuilt from its flag registers, the lazy PF and AF and DF, as contracts/
    /// windows.thread.running_context's rows read them (IF set).
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn a_thread_is_suspended_read_and_resumed_as_windows_answers() {
        use super::{E_MONITOR, E_MONITOR_AT, E_STATUS, E_X, ENDED, entry, suspend, thread_context, write_u64};
        use crate::run::REG_UNDEFINED_FLAGS;
        use guest_runtime::layout::HEAP_VA;

        let (mut g, tid) = guest_with_a_thread();
        let t = tid as u32;
        assert_eq!(suspend(&mut g, t, false), Ok(0));
        assert_eq!(suspend(&mut g, t, false), Ok(1));
        assert_eq!(suspend(&mut g, t, true), Ok(2));
        assert_eq!(suspend(&mut g, t, true), Ok(1));
        assert_eq!(suspend(&mut g, t, true), Ok(0), "a thread not suspended");
        // the thread's saved state: RAX, RSP, RIP, CF set, every flag defined, lazy PF and AF, DF
        let at = entry(g.threads[&tid].index);
        let mem = g.memory.as_mut_slice();
        write_u64(mem, at + E_X, 0x1111);
        write_u64(mem, at + E_X + 8 * 4, 0x7_0000_1000);
        write_u64(mem, at + E_X + 8 * 17, 0x1_4000_2000);
        write_u64(mem, at + E_X + 8 * 19, 1);
        write_u64(mem, at + E_X + 8 * u64::from(REG_UNDEFINED_FLAGS), 0);
        let lazy = at + E_MONITOR + E_MONITOR_AT[0];
        write_u64(mem, lazy, 0x10);
        write_u64(mem, lazy + 8, 0);
        write_u64(mem, lazy + 16, 0x03);
        mem[(lazy + 0x18) as usize] = 1;
        // a CONTEXT asking for control, integer, segments and floating point (0x10000F)
        let buffer = HEAP_VA + 0x800;
        let off = g.layout.offset(buffer).unwrap() as usize;
        g.memory.as_mut_slice()[off..off + 1232].fill(0);
        g.memory.as_mut_slice()[off + 48..off + 52].copy_from_slice(&0x0010_000Fu32.to_le_bytes());
        assert_eq!(thread_context(&mut g, t, buffer), Ok(1));
        let ctx = &g.memory.as_slice()[off..off + 1232];
        let q = |o: usize| u64::from_le_bytes(ctx[o..o + 8].try_into().unwrap());
        let d = |o: usize| u32::from_le_bytes(ctx[o..o + 4].try_into().unwrap());
        assert_eq!(
            (q(120), q(152), q(248)),
            (0x1111, 0x7_0000_1000, 0x1_4000_2000),
            "RAX, RSP, RIP"
        );
        // IF and the reserved bit, CF, PF (0x03 has two ones), AF (0x10 ^ 0 ^ 0x03), DF
        assert_eq!(d(68), 0x202 | 0x1 | 0x4 | 0x10 | 0x400, "EFLAGS");
        assert_eq!(u16::from_le_bytes(ctx[56..58].try_into().unwrap()), 0x33, "CS");
        assert_eq!(d(48), 0x0010_000F, "ContextFlags stays as asked");
        // an ended thread
        write_u64(g.memory.as_mut_slice(), at + E_STATUS, ENDED);
        assert_eq!(suspend(&mut g, t, false), Ok(u64::MAX));
        assert_eq!(suspend(&mut g, t, true), Ok(0));
    }

    /// A created thread's stack grown as contracts/windows.memory.created_thread_stack's rows say (Sprint 213): a touch
    /// of the first guard page moves StackLimit one page and keeps three guard pages below it (default.grow,
    /// default.grown.*); a probe two pages below grows two pages; a touch below the guard is an access violation that
    /// does not move the stack (skip); a probe that would bring the guard to the reservation's last page answers 0 and
    /// changes nothing. The guest needs the runtime image clang builds, so the test runs on the Mac.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn a_created_threads_stack_grows_as_windows_rows_say() {
        use super::{PAGE, stack_grow};
        use crate::guest::{Guest, GuestView};
        use virtual_memory::{MEM_COMMIT, MEM_RESERVE, PAGE_GUARD, PAGE_READWRITE};

        let (mut g, tid) = guest_with_a_thread();
        let (dealloc, limit, _) = g.threads[&tid].stack;
        let teb = g.threads[&tid].teb;
        let stack_limit = |g: &Guest| {
            let at = g.layout.offset(teb + 0x10).unwrap() as usize;
            u64::from_le_bytes(g.memory.as_slice()[at..at + 8].try_into().unwrap())
        };
        let query = |g: &mut Guest, va: u64| {
            let view = GuestView {
                mem: g.memory.as_mut_slice(),
                layout: &g.layout,
            };
            let info = g.arena.query(&view, va).unwrap();
            (info.state, info.protect)
        };
        let guarded = (MEM_COMMIT, PAGE_READWRITE | PAGE_GUARD);
        assert_eq!(stack_limit(&g), limit);
        assert_eq!(query(&mut g, limit - PAGE), guarded, "default.query.limit-1");
        // a touch of the first guard page: StackLimit one page down, three guard pages below, reserved below them
        assert_eq!(stack_grow(&mut g, tid, limit - PAGE + 8, false), Ok(1));
        let one = limit - PAGE;
        assert_eq!(stack_limit(&g), one, "default.grow moved_1");
        assert_eq!(g.threads[&tid].stack.1, one);
        assert_eq!(query(&mut g, one), (MEM_COMMIT, PAGE_READWRITE), "default.grown.limit");
        for k in 1..=3 {
            assert_eq!(query(&mut g, one - k * PAGE), guarded, "default.grown.limit-{k}p");
        }
        assert_eq!(query(&mut g, one - 4 * PAGE).0, MEM_RESERVE, "default.grown.limit-4p");
        // a probe two pages below StackLimit: two pages, as two touches would
        assert_eq!(stack_grow(&mut g, tid, one - 2 * PAGE, true), Ok(1));
        let three = one - 2 * PAGE;
        assert_eq!(stack_limit(&g), three, "two pages more");
        // a probe five pages below, past the guard: a probe grows where a touch may not
        assert_eq!(stack_grow(&mut g, tid, three - 5 * PAGE, true), Ok(1));
        let eight = three - 5 * PAGE;
        assert_eq!(stack_limit(&g), eight, "five pages more");
        // a touch below the guard: an access violation, the stack unmoved (row skip)
        assert_eq!(stack_grow(&mut g, tid, eight - 4 * PAGE, false), Ok(0));
        assert_eq!(stack_limit(&g), eight, "skip: moved 0");
        // a probe that would bring the guard to the reservation's last page: 0 and nothing changed
        assert_eq!(stack_grow(&mut g, tid, dealloc + PAGE, true), Ok(0));
        assert_eq!(stack_limit(&g), eight);
        assert_eq!(query(&mut g, eight - 4 * PAGE).0, MEM_RESERVE, "no page committed");
    }
}
