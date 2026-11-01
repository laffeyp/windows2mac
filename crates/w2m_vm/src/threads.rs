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
    CURRENT, CURRENT_ENTRY, E_MONITOR, E_MONITOR_AT, E_STATE, E_X, LOCK_OWNER, MONITOR_DATA, MONITOR_VA, PAGE,
    PAUSE_COUNT, PEB_OFF, PROCESS_VA, RT_ENTRIES, RUN_ENTRY, RUN_TABLE, RUN_THREADS, SLICE_COUNT, SLICE_PAUSES,
    STATE_SIZE, TIME_SLICE, VECTOR_SLOT, VECTOR_SLOTS,
};
use pe_loader::TLS_SLOTS;
use virtual_memory::{GuestPages, MEM_COMMIT, MEM_RESERVE, PAGE_GUARD, PAGE_READWRITE, STACK_GUARD_PAGES};

use crate::concurrent_memory::ConcurrentGuestMemory;
use crate::guest::{Guest, GuestView};
use crate::modules::ModuleError;
use crate::process::{CLOCK_MODE, CLOCK_RATE, CLOCK_WARP};
use crate::run::{FCW_OFF, MXCSR_OFF, REG_UNDEFINED_FLAGS, RUN_END, fpcr_for};

/// The run table's header and an entry's words (scheduler.py 42-47).
pub const RT_COUNT: u64 = 0x00;
pub const RT_BLOCKED: u64 = 0x20;
/// the vCPUs waiting in the host with no thread to run (`threads.c` 51-52, `struct w2m_run_table`'s `waiting`; Sprint
/// 319), which the guest's `readied()` reads to ask `THREAD_WAKE`
pub const RT_WAITING: u64 = 0x28;
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
pub use guest_runtime::layout::PER_THREAD_MONITOR;
/// scheduler.py 24-26; the first thread's id (vm_backend `_first_tid`).
pub const STILL_ACTIVE: u32 = crate::kernel::STILL_ACTIVE as u32;
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

// Guest memory through the view (Sprint 278, census D7): the scheduler makes no reference into it.
fn read_u64(mem: ConcurrentGuestMemory<'_>, off: u64) -> u64 {
    mem.read_u64(off)
}

fn write_u64(mem: ConcurrentGuestMemory<'_>, off: u64, v: u64) {
    mem.write_u64(off, v);
}

fn put(mem: ConcurrentGuestMemory<'_>, off: u64, bytes: &[u8]) {
    mem.write(off, bytes);
}

/// `n` bytes from `off`, copied out.
fn bytes(mem: ConcurrentGuestMemory<'_>, off: u64, n: u64) -> Vec<u8> {
    let mut out = vec![0; n as usize];
    mem.read(off, &mut out);
    out
}

/// Guest bytes at `va`, through the layout.
fn write_guest(guest: &mut Guest, va: u64, bytes: &[u8]) -> Result<(), String> {
    let at = guest
        .layout
        .offset(va)
        .ok_or_else(|| format!("guest address 0x{va:x} is not mapped"))?;
    guest.memory_view().write(at, bytes);
    Ok(())
}

fn read_guest(guest: &Guest, va: u64, n: u64) -> Result<Vec<u8>, String> {
    let at = guest
        .layout
        .offset(va)
        .ok_or_else(|| format!("guest address 0x{va:x} is not mapped"))?;
    let mut out = vec![0; n as usize];
    guest.memory_view().read(at, &mut out);
    Ok(out)
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
        mem: crate::guest::memory_view(&guest.memory),
        layout: &guest.layout,
    };
    guest
        .served
        .arena
        .alloc(&mut view, address, size, kind, PAGE_READWRITE)
        .map_err(|e| e.0)
}

/// `_add_thread` (3328-3344): a new thread's run table entry, after the others (the table's order is the ids', the
/// switch's rotation), ready, its vector slot given (`_vslot`); the first thread is current on vCPU 0.
pub fn add_thread(guest: &mut Guest, thread: Thread) -> Result<u64, String> {
    let mem = crate::guest::memory_view(&guest.memory);
    let n = read_u64(mem, RUN_TABLE + RT_COUNT);
    if n == RUN_THREADS {
        return Err(format!("more than {RUN_THREADS} threads in one process"));
    }
    if guest
        .served
        .threads
        .keys()
        .next_back()
        .is_some_and(|&last| thread.tid <= last)
    {
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
        write_u64(mem, guest.served.vcpu.monitor + CURRENT, 0);
        write_u64(mem, guest.served.vcpu.monitor + CURRENT_ENTRY, MONITOR_VA + at);
        write_u64(mem, at + E_VCPU, 1);
    }
    guest.served.threads.insert(thread.tid, Thread { index: n, ..thread });
    Ok(n)
}

/// `current_tid` (3316-3321): the id of the thread vCPU 0 runs, its current entry's (`w2m_switch` changes it in the
/// VM).
pub fn current_tid(guest: &Guest) -> u64 {
    let mem = crate::guest::memory_view(&guest.memory);
    if read_u64(mem, RUN_TABLE + RT_COUNT) == 0 {
        // `_first_tid` (vm_backend 2497): the start's thread id, a child's from the Machine (Sprint 206: it was 4)
        return guest.start.tid;
    }
    let at = read_u64(mem, guest.served.vcpu.monitor + CURRENT_ENTRY) - MONITOR_VA;
    read_u64(mem, at + E_TID)
}

/// `_thread_callbacks` (3017-3031): (module base, function) a thread's start and end call, in order: each DLL whose
/// entry runs (load order, then the DLLs loaded after the start), its TLS callbacks and then its DllMain; the program's
/// TLS callbacks last.
pub fn thread_callbacks(guest: &Guest) -> Vec<(u64, u64)> {
    let images = &guest.served.images;
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
    if guest.served.images.is_empty() {
        return Ok(0);
    }
    let mut blocks: Vec<Option<Vec<u8>>> = Vec::new();
    let modules: Vec<usize> = std::iter::once(0)
        .chain((1..guest.served.images.len()).filter(|&i| guest.served.images[i].pe.tls.is_some()))
        .collect();
    for k in modules {
        let image = &guest.served.images[k];
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
    read_u64(crate::guest::memory_view(&guest.memory), entry(index) + E_STATUS) == ENDED
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
    let index = 1 + guest.served.images[1..].iter().filter(|i| i.pe.tls.is_some()).count() as u64;
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
        .served
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
        .served
        .threads
        .get_mut(&current)
        .ok_or_else(|| format!("thread {current} ends, and the host holds no such thread"))?;
    t.exit_code = code;
    Ok(())
}

/// `_thread_request`'s THREAD_EXIT_CODE (3206-3211): thread `tid`'s exit code, STILL_ACTIVE while it runs.
pub fn exit_code(guest: &Guest, tid: u32) -> Result<u32, String> {
    guest
        .served
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
        .served
        .threads
        .get(&u64::from(tid))
        .map(|t| t.index)
        .ok_or_else(|| format!("a thread request ({sub}) for thread id {tid}, which w2m did not create"))?;
    let mem = crate::guest::memory_view(&guest.memory);
    let at = entry(index);
    let count = read_u64(mem, at + E_SUSPEND);
    let ended = ended(guest, index);
    let mem = crate::guest::memory_view(&guest.memory);
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
        .served
        .threads
        .get(&u64::from(tid))
        .ok_or_else(|| format!("a thread request (6) for thread id {tid}, which w2m did not create"))?;
    if u64::from(tid) == current_tid(guest) {
        return Err(format!(
            "GetThreadContext of the calling thread {tid}, which is not in the rows"
        ));
    }
    let mem = crate::guest::memory_view(&guest.memory);
    let at = entry(t.index);
    let x: Vec<u64> = (0..31).map(|i| read_u64(mem, at + E_X + 8 * i)).collect();
    // the monitor ranges the thread keeps: lazy AF and DF (0x20-0x40), MXCSR (0x50), the x87 control word (0x70)
    let monitor = |range: usize, off: u64| at + E_MONITOR + E_MONITOR_AT[range] + off;
    let (af_a, af_b, af_r) = (
        read_u64(mem, monitor(0, 0)),
        read_u64(mem, monitor(0, 8)),
        read_u64(mem, monitor(0, 16)),
    );
    let df = u32::from(bytes(mem, monitor(0, DF_BYTE - 0x20), 1)[0] & 1);
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
    let ntdll = guest.served.process.ntdll;
    let rip = if read_u64(mem, at + E_STATUS) == BLOCKED && ntdll != 0 {
        ntdll + 0x800
    } else {
        x[17]
    };
    let slot = read_u64(mem, at + E_VSLOT);
    let xmm = bytes(mem, slot, 256);
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
    let mem = crate::guest::memory_view(&guest.memory);
    let at = entry(index);
    let was = read_u64(mem, at + E_STATUS);
    if (was == BLOCKED) != (status == BLOCKED) {
        let blocked = read_u64(mem, RUN_TABLE + RT_BLOCKED);
        let blocked = if status == BLOCKED { blocked + 1 } else { blocked - 1 };
        write_u64(mem, RUN_TABLE + RT_BLOCKED, blocked);
    }
    write_u64(mem, at + E_STATUS, status);
    // a thread readied in the host: a parked vCPU may take it (the run-loop design's "A vCPU with no thread", the host's
    // one writer of a thread's status; Sprint 319)
    if status == READY && was != READY {
        guest.sched.notify();
    }
}

/// What a parked vCPU waits on (the run-loop design's "A vCPU with no thread"; Sprint 319): a count of changes that may
/// let a vCPU take a thread, which every readying notifies, and of the vCPUs parked (Sprint 320), outside the run's lock
/// so a parked vCPU waits holding none.
#[derive(Default, Debug)]
pub struct Sched {
    counts: std::sync::Mutex<Counts>,
    changed: std::sync::Condvar,
}

#[derive(Default, Debug)]
struct Counts {
    changes: u64,
    parked: u64,
    /// the parked vCPUs waiting with a deadline: while any is, a wait with none takes no deadlock verdict, the deadline
    /// a wake to come (Sprint 320's audit: each vCPU's own deadline, read at its park, does not see one set after)
    timed: u64,
    /// a deadlock declared by the vCPU whose quiet second ran out, which every parked vCPU then ends on
    deadlock: bool,
}

/// How a parked vCPU's wait ended (Sprint 320; the Python host's Sprint 120, `docs/design-several-vcpus-2026-10-17.md`
/// 176-192).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Woken {
    /// a change: a thread readied, a wake the guest asked for, or the run's stop
    Changed,
    /// the earliest deadline of a blocked thread came: expired waits are made ready before the vCPU takes
    Deadline,
    /// every vCPU of the run parked, none with a deadline, and a second with no change
    Deadlock,
}

impl Sched {
    /// A change: a thread readied, a wake the guest asked for, or the run's stop.
    pub fn notify(&self) {
        let mut counts = self.counts.lock().unwrap_or_else(|e| e.into_inner());
        counts.changes += 1;
        self.changed.notify_all();
    }

    /// The count of changes so far, read before a park so a change after it is not missed.
    pub fn changes(&self) -> u64 {
        self.counts.lock().unwrap_or_else(|e| e.into_inner()).changes
    }

    /// Parks one of the run's `vcpus` vCPUs until a change past `seen`, until `deadline`, or, when every vCPU is
    /// parked and none has a deadline, for one second with no change. A wait with neither has no timeout of its own:
    /// the run's stop notifies (the design's card 4c3b2), as the Python host's waiter waited for its run's timeout.
    pub fn park(&self, seen: u64, deadline: Option<std::time::Instant>, vcpus: u64) -> Woken {
        const QUIET: std::time::Duration = std::time::Duration::from_secs(1);
        let mut counts = self.counts.lock().unwrap_or_else(|e| e.into_inner());
        counts.parked += 1;
        counts.timed += u64::from(deadline.is_some());
        // the second with no change starts when every vCPU is parked and none with a deadline; a change ends the park
        let mut quiet_since: Option<std::time::Instant> = None;
        let woken = loop {
            if counts.changes != seen {
                break Woken::Changed;
            }
            if counts.deadlock {
                break Woken::Deadlock;
            }
            let now = std::time::Instant::now();
            if deadline.is_some_and(|d| now >= d) {
                break Woken::Deadline;
            }
            let all = counts.timed == 0 && counts.parked == vcpus;
            if !all {
                quiet_since = None;
            }
            let since = all.then(|| *quiet_since.get_or_insert(now));
            if since.is_some_and(|s| now >= s + QUIET) {
                // the whole run's deadlock: every parked vCPU, some waiting with no timeout, ends on it
                counts.deadlock = true;
                self.changed.notify_all();
                break Woken::Deadlock;
            }
            counts = match deadline.or(since.map(|s| s + QUIET)) {
                Some(until) => {
                    let wait = until.saturating_duration_since(now);
                    self.changed
                        .wait_timeout(counts, wait)
                        .unwrap_or_else(|e| e.into_inner())
                        .0
                }
                None => self.changed.wait(counts).unwrap_or_else(|e| e.into_inner()),
            };
        };
        counts.parked -= 1;
        counts.timed -= u64::from(deadline.is_some());
        woken
    }
}

/// The earliest deadline of a blocked, unsuspended thread, as an `Instant` on the host's clock (Sprint 320), the
/// guest's counter in 100 ns units (`counter_now`); none when no blocked thread has one.
pub fn park_deadline(guest: &Guest) -> Option<std::time::Instant> {
    let mem = crate::guest::memory_view(&guest.memory);
    let word = |t: &Thread, off: u64| read_u64(mem, entry(t.index) + off);
    let earliest = guest
        .served
        .threads
        .values()
        .filter(|t| word(t, E_STATUS) == BLOCKED && word(t, E_SUSPEND) == 0 && word(t, E_DEADLINE) != u64::MAX)
        .map(|t| word(t, E_DEADLINE))
        .min()?;
    let ahead = earliest.saturating_sub(counter_now(guest));
    Some(std::time::Instant::now() + std::time::Duration::from_nanos(ahead.saturating_mul(100)))
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
    /// no thread this vCPU may take is ready while another vCPU holds one (Sprint 319; the run-loop design's "A vCPU
    /// with no thread"): this vCPU's thread put down and the vCPU counted in the run table's `waiting`; it waits on
    /// `Sched` holding neither the word nor the lock, then `take`s a thread
    Parked,
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
    let mem = crate::guest::memory_view(&guest.memory);
    if read_u64(mem, guest.served.vcpu.monitor + CLOCK_MODE) == 2 {
        let executed = u128::from(read_u64(mem, guest.served.vcpu.monitor));
        let rate = u128::from(read_u64(mem, guest.served.vcpu.monitor + CLOCK_RATE));
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
    if read_u64(
        crate::guest::memory_view(&guest.memory),
        guest.served.vcpu.monitor + CLOCK_MODE,
    ) != 2
    {
        return;
    }
    let lag = crate::machine::catch_up(counter_now(guest), machine, processes);
    let mem = crate::guest::memory_view(&guest.memory);
    let warp = read_u64(mem, RUN_TABLE + CLOCK_WARP);
    write_u64(mem, RUN_TABLE + CLOCK_WARP, warp + lag);
}

/// `_wake_expired` (4897-4905): every blocked thread whose deadline has passed is ready.
pub fn wake_expired(guest: &mut Guest) {
    let now = counter_now(guest);
    let mem = crate::guest::memory_view(&guest.memory);
    let expired: Vec<u64> = guest
        .served
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
    let mem = crate::guest::memory_view(&guest.memory);
    let word = |t: &Thread, off: u64| read_u64(mem, entry(t.index) + off);
    let deadline = guest
        .served
        .threads
        .values()
        .filter(|t| word(t, E_STATUS) == BLOCKED && word(t, E_SUSPEND) == 0)
        .map(|t| word(t, E_DEADLINE))
        .min()
        .unwrap_or(u64::MAX);
    crate::turns::State {
        clock: counter_now(guest),
        ready: guest
            .served
            .threads
            .values()
            .any(|t| word(t, E_STATUS) == READY && word(t, E_SUSPEND) == 0),
        timed: deadline != u64::MAX,
        deadline,
        blocked: guest.served.threads.values().any(|t| word(t, E_STATUS) == BLOCKED),
    }
}

/// `warp_to_deadline` (vm_backend 3614-3628): on the instruction clock with no thread ready, the clock moved to the
/// earliest deadline of a blocked, unsuspended thread (the lowest tid among equals) and that thread made ready; false
/// when no thread waits with a deadline or the clock is another.
pub fn warp_to_deadline(guest: &mut Guest) -> bool {
    let mem = crate::guest::memory_view(&guest.memory);
    if read_u64(mem, guest.served.vcpu.monitor + CLOCK_MODE) != 2 {
        return false;
    }
    let word = |t: &Thread, off: u64| read_u64(mem, entry(t.index) + off);
    let first = guest
        .served
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
        let mem = crate::guest::memory_view(&guest.memory);
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
    let mem = crate::guest::memory_view(&guest.memory);
    let word = |t: &Thread, off: u64| read_u64(mem, entry(t.index) + off);
    if !guest
        .served
        .threads
        .values()
        .any(|t| word(t, E_STATUS) == READY && word(t, E_SUSPEND) == 0)
    {
        let timed = guest
            .served
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
        .served
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
    // the threads and the turn are every vCPU's services' to change, under the runtime word (Sprint 294): a caller
    // that skipped the take fails here, in the gates' release runs as in the tests (Sprint 311: a debug assertion was
    // compiled out of the `w2m-vm` the gates run)
    assert!(
        crate::monitor_code::runtime_word_held(
            guest.memory_view(),
            crate::monitor_code::vcpu_owner(guest.memory_view(), guest.served.vcpu.monitor)
        ),
        "the scheduler entered without the runtime word"
    );
    let current = current_tid(guest);
    let cur = guest
        .served
        .threads
        .get(&current)
        .ok_or_else(|| format!("thread {current} hands over, and the host holds no such thread"))?
        .clone();
    let word =
        |guest: &Guest, t: &Thread, off: u64| read_u64(crate::guest::memory_view(&guest.memory), entry(t.index) + off);
    if bits & SCHEDULE_END != 0 {
        set_status(guest, cur.index, ENDED);
        let blocked: Vec<u64> = guest
            .served
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
        write_u64(
            crate::guest::memory_view(&guest.memory),
            entry(cur.index) + E_DEADLINE,
            deadline,
        );
    }
    if guest.served.threads.values().all(|t| word(guest, t, E_STATUS) == ENDED) {
        return Ok(Switch::Ended(cur.exit_code)); // the last thread ended: the process ends with it
    }
    switch(guest, x, &cur, bits)
}

/// `_switch` with no thread ready and `timed`, in rotation order, waiting with a deadline (3565-3580 at the tag),
/// readied as the frozen host readies one (Sprint 234). On the instruction clock, `warp_to_deadline` (3614-3628): the
/// earliest deadline, then the lowest thread id, and the clock's warp moved so the clock reads that deadline if it is
/// still ahead. Else the host sleeps `pause` (1 ms there) and readies the first timed waiter, which checks its own
/// wait again in guest code and blocks again if its deadline has not come.
fn timed_waiter(guest: &mut Guest, timed: &[Thread], pause: std::time::Duration) -> Thread {
    let mem = crate::guest::memory_view(&guest.memory);
    let deadline = |t: &Thread| read_u64(mem, entry(t.index) + E_DEADLINE);
    let chosen = if read_u64(mem, guest.served.vcpu.monitor + CLOCK_MODE) == 2 {
        let first = timed.iter().min_by_key(|t| (deadline(t), t.tid)).unwrap().clone();
        let (at, now) = (deadline(&first), counter_now(guest));
        if at > now {
            let warp = read_u64(mem, RUN_TABLE + CLOCK_WARP);
            write_u64(
                crate::guest::memory_view(&guest.memory),
                RUN_TABLE + CLOCK_WARP,
                warp + at - now,
            );
        }
        first
    } else {
        std::thread::sleep(pause);
        timed[0].clone()
    };
    set_status(guest, chosen.index, READY);
    chosen
}

/// `_switch` (3521-3580) for vCPU 0 of one process, the thread not yet put down: the next ready, unsuspended thread
/// after `cur` in id order, round robin, `cur` last. With none ready and one waiting with a deadline, `timed_waiter`
/// readies one (Sprint 234); with none waiting with a deadline the threads are deadlocked.
fn switch(guest: &mut Guest, x: &mut [u64; 31], cur: &Thread, bits: u32) -> Result<Switch, String> {
    let mem = crate::guest::memory_view(&guest.memory);
    let word = |t: &Thread, off: u64| read_u64(mem, entry(t.index) + off);
    // this vCPU's index + 1, its `LOCK_OWNER` word, as the C runtime's `this_vcpu` reads it; a thread whose `vcpu` word
    // is another vCPU's is loaded there, and the pick passes it by, as `held_elsewhere` does (`threads.c` 64-68;
    // Sprint 316)
    let own = read_u64(mem, guest.served.vcpu.monitor + LOCK_OWNER);
    let here = |t: &Thread| free_for(mem, t, own);
    let order: Vec<&Thread> = guest.served.threads.values().collect();
    let i = order.iter().position(|t| t.tid == cur.tid).unwrap();
    let rotation = order[i + 1..].iter().chain(order[..=i].iter());
    let next = rotation
        .clone()
        .find(|t| word(t, E_STATUS) == READY && word(t, E_SUSPEND) == 0 && here(t))
        .map(|t| (*t).clone());
    // none here ready while another vCPU holds a thread, which can ready one: this vCPU parks, its thread put down and
    // itself counted in `waiting` (the run-loop design's "A vCPU with no thread"; Sprint 319); one vCPU never parks
    if next.is_none() && order.iter().any(|t| !here(t)) {
        save(guest, x, cur);
        write_u64(mem, RUN_TABLE + RT_WAITING, read_u64(mem, RUN_TABLE + RT_WAITING) + 1);
        return Ok(Switch::Parked);
    }
    if next.is_none() && guest.served.link.is_some() {
        return Ok(Switch::Idle);
    }
    let next = match next {
        Some(next) => next,
        None => {
            let timed: Vec<Thread> = rotation
                .filter(|t| {
                    word(t, E_STATUS) == BLOCKED
                        && word(t, E_SUSPEND) == 0
                        && word(t, E_DEADLINE) != u64::MAX
                        && here(t)
                })
                .map(|t| (*t).clone())
                .collect();
            if timed.is_empty() {
                return Err("every thread waits with no timeout and nothing can wake one (a deadlock)".into());
            }
            timed_waiter(guest, &timed, std::time::Duration::from_millis(1))
        }
    };
    // the thread given vCPU 0 is ready, whichever path chose it: the frozen `_switch` only ever takes a ready one, and a
    // blocked one run anyway leaves the run table's state and blocked count wrong for the rest of the run, unseen by
    // any run's output (Sprint 234; reviewer-3's Low 1 of its review at 234)
    let status = read_u64(crate::guest::memory_view(&guest.memory), entry(next.index) + E_STATUS);
    if status != READY {
        return Err(format!(
            "thread {} given vCPU 0 while not ready (status {status})",
            next.tid
        ));
    }
    if bits & SCHEDULE_ANSWER != 0 {
        x[0] = u64::from(next.tid != cur.tid); // SwitchToThread: nonzero when another thread runs
    }
    if next.tid == cur.tid {
        return Ok(Switch::Go { vectors: None });
    }
    let (executed, pauses) = save(guest, x, cur);
    load(guest, x, &next, own, executed, pauses)?;
    let mem = crate::guest::memory_view(&guest.memory);
    let vectors = (
        read_u64(mem, entry(cur.index) + E_VSLOT),
        read_u64(mem, entry(next.index) + E_VSLOT),
    );
    Ok(Switch::Go { vectors: Some(vectors) })
}

/// `_save_thread` (3467-3479): `cur`, the thread on the served vCPU, put down: its registers `x`, its instructions and
/// pauses since its slice began, its per-thread monitor words and the state block into its run entry, its `vcpu` word
/// cleared; the vCPU's count and pauses, which the next load's slice begins from.
fn save(guest: &Guest, x: &[u64; 31], cur: &Thread) -> (u64, u64) {
    let mem = crate::guest::memory_view(&guest.memory);
    let from = entry(cur.index);
    let (executed, pauses) = (
        read_u64(mem, guest.served.vcpu.monitor),
        read_u64(mem, guest.served.vcpu.monitor + PAUSE_COUNT),
    );
    let (begun, begun_pauses) = (
        read_u64(mem, guest.served.vcpu.monitor + SLICE_COUNT),
        read_u64(mem, guest.served.vcpu.monitor + SLICE_PAUSES),
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
    write_u64(mem, guest.served.vcpu.monitor + SLICE_COUNT, executed);
    write_u64(mem, guest.served.vcpu.monitor + SLICE_PAUSES, pauses);
    for (i, r) in x.iter().enumerate() {
        write_u64(mem, from + E_X + 8 * i as u64, *r);
    }
    let state = guest.served.vcpu.state;
    for (p, (a, b)) in E_MONITOR_AT.iter().zip(PER_THREAD_MONITOR) {
        let data = bytes(mem, guest.served.vcpu.monitor + a, b - a);
        put(mem, from + E_MONITOR + p, &data);
    }
    let data = bytes(mem, state, STATE_SIZE);
    put(mem, from + E_STATE, &data);
    write_u64(mem, from + E_VCPU, 0);
    (executed, pauses)
}

/// `set_current` and `_load_thread` (3323-3326, 3487-3498): `next` loaded onto the served vCPU, whose owner is `own`:
/// the vCPU's current thread, its per-thread monitor words and state block from the run entry, its slice begun at
/// `executed` and `pauses`, its `vcpu` word `own`, and its registers into `x`. A thread another vCPU holds is refused.
fn load(guest: &Guest, x: &mut [u64; 31], next: &Thread, own: u64, executed: u64, pauses: u64) -> Result<(), String> {
    let mem = crate::guest::memory_view(&guest.memory);
    let to = entry(next.index);
    let held = read_u64(mem, to + E_VCPU);
    if held != 0 && held != own {
        return Err(format!(
            "thread {} loaded on vCPU owner {own} while vCPU owner {held} has it",
            next.tid
        ));
    }
    write_u64(mem, guest.served.vcpu.monitor + CURRENT, next.index);
    write_u64(mem, guest.served.vcpu.monitor + CURRENT_ENTRY, MONITOR_VA + to);
    for (p, (a, b)) in E_MONITOR_AT.iter().zip(PER_THREAD_MONITOR) {
        let data = bytes(mem, to + E_MONITOR + p, b - a);
        put(mem, guest.served.vcpu.monitor + a, &data);
    }
    let data = bytes(mem, to + E_STATE, STATE_SIZE);
    put(mem, guest.served.vcpu.state, &data);
    write_u64(mem, guest.served.vcpu.monitor + SLICE_COUNT, executed);
    write_u64(mem, guest.served.vcpu.monitor + SLICE_PAUSES, pauses);
    write_u64(mem, to + E_VCPU, own);
    for (i, r) in x.iter_mut().enumerate() {
        *r = read_u64(mem, to + E_X + 8 * i as u64);
    }
    Ok(())
}

/// Whether `t` may be loaded on the vCPU whose owner is `own`: no vCPU holds it, or that one does, the C runtime's
/// `held_elsewhere` negated (`threads.c` 68).
fn free_for(mem: ConcurrentGuestMemory<'_>, t: &Thread, own: u64) -> bool {
    matches!(read_u64(mem, entry(t.index) + E_VCPU), v if v == 0 || v == own)
}

/// A thread for a vCPU with none, after its park (Sprint 319; the run-loop design's "A vCPU with no thread"): the
/// first ready, unsuspended thread in id order that no other vCPU holds, loaded with no thread put down, and the vCPU
/// taken off the run table's `waiting`; its id and its vector slot, for the vectors' load. None when none is ready:
/// the vCPU parks again, still counted.
pub fn take(guest: &mut Guest, x: &mut [u64; 31]) -> Result<Option<(u64, u64)>, String> {
    let mem = crate::guest::memory_view(&guest.memory);
    let own = read_u64(mem, guest.served.vcpu.monitor + LOCK_OWNER);
    let word = |t: &Thread, off: u64| read_u64(mem, entry(t.index) + off);
    let Some(next) = guest
        .served
        .threads
        .values()
        .find(|t| word(t, E_STATUS) == READY && word(t, E_SUSPEND) == 0 && free_for(mem, t, own))
        .cloned()
    else {
        return Ok(None);
    };
    let (executed, pauses) = (
        read_u64(mem, guest.served.vcpu.monitor),
        read_u64(mem, guest.served.vcpu.monitor + PAUSE_COUNT),
    );
    load(guest, x, &next, own, executed, pauses)?;
    write_u64(
        mem,
        RUN_TABLE + RT_WAITING,
        read_u64(mem, RUN_TABLE + RT_WAITING).saturating_sub(1),
    );
    Ok(Some((next.tid, word(&next, E_VSLOT))))
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
pub fn stack_grow(
    guest: &mut Guest,
    tid: u64,
    address: u64,
    probe: bool,
    flush: crate::guest::Flush<'_>,
) -> Result<u64, String> {
    let Some(thread) = guest.served.threads.get(&tid) else {
        return Ok(0);
    };
    let (dealloc, _, top) = thread.stack;
    let teb = thread.teb;
    if !guest.served.arena.reservations.contains_key(&dealloc) {
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
            mem: crate::guest::memory_view(&guest.memory),
            layout: &guest.layout,
        };
        guest.served.arena.commit(
            &mut view as &mut dyn GuestPages,
            dealloc,
            new,
            limit - new,
            PAGE_READWRITE,
        );
        guest.map_pages(new, limit, true, flush)?;
        let mut view = GuestView {
            mem: crate::guest::memory_view(&guest.memory),
            layout: &guest.layout,
        };
        guest.served.arena.commit(
            &mut view as &mut dyn GuestPages,
            dealloc,
            new - guard,
            guard,
            PAGE_READWRITE | PAGE_GUARD,
        );
        limit = new;
    }
    write_guest(guest, teb + 0x10, &limit.to_le_bytes())?;
    if let Some(t) = guest.served.threads.get_mut(&tid) {
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
    flush: crate::guest::Flush<'_>,
) -> Result<u64, String> {
    let tid = guest.served.threads.keys().next_back().copied().unwrap_or(0) + 4;
    let program = guest
        .served
        .images
        .first()
        .map(|i| (i.pe.stack_reserve, i.pe.stack_commit));
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
            mem: crate::guest::memory_view(&guest.memory),
            layout: &guest.layout,
        };
        guest.served.arena.commit(
            &mut view as &mut dyn GuestPages,
            dealloc,
            limit - STACK_GUARD_PAGES * PAGE,
            STACK_GUARD_PAGES * PAGE,
            PAGE_READWRITE | PAGE_GUARD,
        );
    }
    guest.map_pages(dealloc, limit, false, flush)?;
    let teb = alloc(guest, 0, 0x4000, MEM_COMMIT | MEM_RESERVE, "a thread's TEB")?; // TEB, then TlsExpansionSlots
    let static_tls = thread_static_tls(guest)?;
    let peb = PROCESS_VA + PEB_OFF;
    let process_id = guest.served.process.process_id;
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
    let mem = crate::guest::memory_view(&guest.memory);
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
    guest.served.arena.stacks.push((dealloc, limit, top));
    set_budget(guest); // two threads: time slices from now on
    Ok(tid)
}

/// `_set_budget` (3284-3289): the step budget the block entries check: the run's limit, or with more than one thread a
/// time slice.
pub fn set_budget(guest: &mut Guest) {
    // a process of a machine shares the vCPU's time with the others too (Sprint 197)
    let shared = guest.served.threads.len() > 1 || guest.served.link.is_some();
    let mem = crate::guest::memory_view(&guest.memory);
    let (executed, max_steps) = (
        read_u64(mem, guest.served.vcpu.monitor),
        read_u64(mem, guest.served.vcpu.monitor + RUN_END),
    );
    let limit = if shared {
        max_steps.min(executed + TIME_SLICE)
    } else {
        max_steps
    };
    write_u64(mem, guest.served.vcpu.monitor + 8, limit);
}

/// `_slice_until` (2756-2780) for one vCPU, one process, no lockstep, forced stop or pending wake: 0 when the current
/// thread is suspended or another is ready; the count at which the earliest deadline of a blocked thread falls, on the
/// instruction clock (0 on another clock); else ~0.
pub fn slice_until(guest: &Guest) -> u64 {
    // another process ready or waiting with a deadline needs the host at each slice's end (`_other_process_due`,
    // 2750-2754; Sprint 197)
    if guest
        .served
        .link
        .as_ref()
        .is_some_and(|l| l.others_due(counter_now(guest)))
    {
        return 0;
    }
    let mem = crate::guest::memory_view(&guest.memory);
    let current = current_tid(guest);
    let word = |t: &Thread, off: u64| read_u64(mem, entry(t.index) + off);
    if let Some(cur) = guest.served.threads.get(&current)
        && word(cur, E_SUSPEND) != 0
    {
        return 0;
    }
    let others: Vec<&Thread> = guest
        .served
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
    if read_u64(mem, guest.served.vcpu.monitor + CLOCK_MODE) != 2 {
        return 0;
    }
    let (rate, warp) = (
        read_u64(mem, guest.served.vcpu.monitor + CLOCK_RATE),
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
pub(crate) mod tests {
    /// A guest of qpc.exe with one created thread; the runtime image clang builds, so the tests that use it run on the
    /// Mac.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    pub(crate) fn guest_with_a_thread() -> (crate::guest::Guest, u64) {
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
        super::create_thread(&mut g, loaded.entry, 0, 0, 0, &mut || Ok(())).unwrap();
        let tid = super::create_thread(&mut g, loaded.entry, 0, 0, 0, &mut || Ok(())).unwrap();
        (g, tid)
    }

    /// A thread has ended by its run table state, not its exit code (Sprint 218; reviewer-3's Medium 1): an ended thread
    /// whose code is 259, STILL_ACTIVE's value, has ended; a running one whose code THREAD_END has set has not.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn a_thread_has_ended_by_its_state_not_its_exit_code() {
        use super::{ENDED, READY, STILL_ACTIVE, ended, set_status};

        let (mut g, tid) = guest_with_a_thread();
        let index = g.served.threads[&tid].index;
        assert!(!ended(&g, index));
        g.served.threads.get_mut(&tid).unwrap().exit_code = 7;
        assert!(!ended(&g, index), "THREAD_END's code set, the thread still running");
        set_status(&mut g, index, ENDED);
        g.served.threads.get_mut(&tid).unwrap().exit_code = STILL_ACTIVE;
        assert!(
            ended(&g, index),
            "ExitThread(259): ended, its code reading STILL_ACTIVE"
        );
        set_status(&mut g, index, READY);
        assert!(!ended(&g, index));
    }

    /// The host's switch on any vCPU (Sprint 316), as the C runtime's: a ready thread whose `vcpu` word is another
    /// vCPU's owner is passed by, and the thread loaded takes this vCPU's owner, its `LOCK_OWNER` word. Three threads
    /// in id order, the first current on vCPU owner 1.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn the_switch_passes_a_thread_another_vcpu_holds_and_claims_with_its_own_owner() {
        use super::{E_VCPU, Switch, current_tid, entry, read_u64, switch, write_u64};
        use guest_runtime::layout::LOCK_OWNER;

        let three = || {
            let (mut g, _) = guest_with_a_thread();
            let start = g.served.threads.values().next().unwrap().start;
            super::create_thread(&mut g, start, 0, 0, 0, &mut || Ok(())).unwrap();
            let ts: Vec<super::Thread> = g.served.threads.values().cloned().collect();
            (g, ts)
        };
        let vcpu = |g: &crate::guest::Guest, t: &super::Thread| read_u64(g.memory_view(), entry(t.index) + E_VCPU);

        // owner 1: the second thread held by owner 2 is passed by, the third loaded and claimed with 1
        let (mut g, ts) = three();
        write_u64(g.memory_view(), g.served.vcpu.monitor + LOCK_OWNER, 1);
        write_u64(g.memory_view(), entry(ts[1].index) + E_VCPU, 2);
        let mut x = [0u64; 31];
        let went = switch(&mut g, &mut x, &ts[0], 0).unwrap();
        assert!(matches!(went, Switch::Go { .. }));
        assert_eq!(current_tid(&g), ts[2].tid, "the thread owner 2 holds passed by");
        assert_eq!(
            (vcpu(&g, &ts[1]), vcpu(&g, &ts[2])),
            (2, 1),
            "the claims after the switch"
        );

        // owner 2: the second thread, free, loaded and claimed with 2
        let (mut g, ts) = three();
        write_u64(g.memory_view(), g.served.vcpu.monitor + LOCK_OWNER, 2);
        write_u64(g.memory_view(), entry(ts[0].index) + E_VCPU, 2);
        switch(&mut g, &mut x, &ts[0], 0).unwrap();
        assert_eq!(current_tid(&g), ts[1].tid);
        assert_eq!(vcpu(&g, &ts[1]), 2, "the loaded thread claimed with this vCPU's owner");
    }

    /// A vCPU parks and is woken (Sprint 319; the run-loop design's "A vCPU with no thread"): vCPU 1, on its own pages
    /// with owner 2, finds nothing it may take while vCPU 0 holds a thread, parks with its thread put down and counted
    /// in `waiting`; vCPU 0 readies a blocked thread through `set_status`, and vCPU 1, waiting on `Sched` on another
    /// host thread, wakes before its bound and takes it.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn a_parked_vcpu_is_woken_by_a_readied_thread_and_takes_it() {
        use super::{BLOCKED, E_DEADLINE, E_VCPU, E_VSLOT, READY, RT_WAITING, Switch, entry, read_u64, set_status};
        use super::{switch, take, write_u64};
        use guest_runtime::layout::{CURRENT, CURRENT_ENTRY, LOCK_OWNER, MONITOR_VA, RUN_TABLE};
        use std::time::Duration;

        let (mut g, _) = guest_with_a_thread();
        let start = g.served.threads.values().next().unwrap().start;
        super::create_thread(&mut g, start, 0, 0, 0, &mut || Ok(())).unwrap();
        let ts: Vec<super::Thread> = g.served.threads.values().cloned().collect();
        let first = g.served.vcpu;
        let (_, second) = g.private_chain(1).unwrap();
        // vCPU 0, owner 1, holds the first thread; vCPU 1, owner 2, the third; the second and third blocked, no deadline
        let mem = g.memory_view();
        write_u64(mem, first.monitor + LOCK_OWNER, 1);
        write_u64(mem, second.monitor + LOCK_OWNER, 2);
        write_u64(mem, entry(ts[2].index) + E_VCPU, 2);
        write_u64(mem, second.monitor + CURRENT, ts[2].index);
        write_u64(mem, second.monitor + CURRENT_ENTRY, MONITOR_VA + entry(ts[2].index));
        for t in [&ts[1], &ts[2]] {
            set_status(&mut g, t.index, BLOCKED);
            write_u64(g.memory_view(), entry(t.index) + E_DEADLINE, u64::MAX);
        }
        g.served.vcpu = second;
        assert_eq!(switch(&mut g, &mut [0u64; 31], &ts[2], 0), Ok(Switch::Parked));
        let mem = g.memory_view();
        fn waiting(mem: crate::concurrent_memory::ConcurrentGuestMemory<'_>) -> u64 {
            read_u64(mem, RUN_TABLE + RT_WAITING)
        }
        assert_eq!(
            (read_u64(mem, entry(ts[2].index) + E_VCPU), waiting(mem)),
            (0, 1),
            "the parked vCPU's thread put down and the vCPU counted waiting"
        );

        let (sched, seen) = (g.sched.clone(), g.sched.changes());
        let guest = std::sync::Mutex::new(g);
        let (woken, taken) = std::thread::scope(|s| {
            let parked = s.spawn(|| {
                let woken = sched.park(seen, Some(std::time::Instant::now() + Duration::from_secs(5)), 2)
                    == super::Woken::Changed;
                let mut g = guest.lock().unwrap();
                g.served.vcpu = second;
                (woken, take(&mut g, &mut [0u64; 31]).unwrap())
            });
            let mut g = guest.lock().unwrap();
            g.served.vcpu = first;
            set_status(&mut g, ts[1].index, READY);
            drop(g);
            parked.join().unwrap()
        });
        let g = guest.into_inner().unwrap();
        let mem = g.memory_view();
        assert!(
            woken,
            "the parked vCPU's wait ran its whole bound, the readying not seen"
        );
        assert_eq!(
            taken,
            Some((ts[1].tid, read_u64(mem, entry(ts[1].index) + E_VSLOT))),
            "the thread the parked vCPU took"
        );
        assert_eq!(
            (read_u64(mem, entry(ts[1].index) + E_VCPU), waiting(mem)),
            (2, 0),
            "the taken thread claimed by owner 2, the vCPU no longer waiting"
        );
    }

    /// A parked vCPU's wait ends three ways (Sprint 320; the Python host's Sprint 120): at its deadline; at a change,
    /// one of two vCPUs parked with no deadline waiting past the deadlock's second; and as a deadlock only when every
    /// vCPU of the run is parked with no deadline, after one second with no change.
    #[test]
    fn a_parked_wait_ends_at_its_deadline_at_a_change_or_in_a_deadlock_of_every_vcpu() {
        use super::{Sched, Woken};
        use std::time::{Duration, Instant};

        let sched = Sched::default();
        let started = Instant::now();
        let woken = sched.park(sched.changes(), Some(started + Duration::from_millis(50)), 2);
        let took = started.elapsed();
        assert_eq!(woken, Woken::Deadline);
        assert!(
            took >= Duration::from_millis(50) && took < Duration::from_secs(2),
            "the deadline's wait took {took:?}"
        );

        // one of two parked, no deadline: no deadlock, though its second passes; a change ends it
        let (alone, seen) = (Sched::default(), 0);
        let woken = std::thread::scope(|s| {
            let parked = s.spawn(|| alone.park(seen, None, 2));
            std::thread::sleep(Duration::from_millis(1300));
            alone.notify();
            parked.join().unwrap()
        });
        assert_eq!(
            woken,
            Woken::Changed,
            "one of two vCPUs parked with no deadline ended its wait"
        );

        // both of two parked, no deadline, no change: a deadlock after one second
        let both = Sched::default();
        let started = Instant::now();
        let ends = std::thread::scope(|s| {
            let a = s.spawn(|| both.park(0, None, 2));
            let b = s.spawn(|| both.park(0, None, 2));
            (a.join().unwrap(), b.join().unwrap())
        });
        let took = started.elapsed();
        assert_eq!(ends, (Woken::Deadlock, Woken::Deadlock));

        // two parked, the second with a deadline set after the first parked: the first, its own deadline none, takes
        // no deadlock verdict while the second's deadline stands (Sprint 320's audit), even woken with no change, as
        // `Condvar` allows (a spurious wakeup, made here by a bare `notify_all`); a later change ends it
        let late = Sched::default();
        let ends = std::thread::scope(|s| {
            let first = s.spawn(|| late.park(0, None, 2));
            std::thread::sleep(Duration::from_millis(50));
            let second = s.spawn(|| late.park(0, Some(Instant::now() + Duration::from_millis(1500)), 2));
            std::thread::sleep(Duration::from_millis(50));
            late.changed.notify_all();
            let second = second.join().unwrap();
            late.notify();
            (first.join().unwrap(), second)
        });
        assert_eq!(
            ends,
            (Woken::Changed, Woken::Deadline),
            "a deadlock declared while another parked vCPU's deadline stood"
        );
        assert!(
            took >= Duration::from_secs(1) && took < Duration::from_secs(3),
            "the deadlock took {took:?}"
        );
    }

    /// The park's deadline is the earliest of the blocked threads' (Sprint 320): of two, one 50 ms ahead on the host's
    /// clock and one 10 s ahead, the earlier, as an `Instant` within its 50 ms.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn the_parks_deadline_is_the_earliest_blocked_threads() {
        use super::{BLOCKED, E_DEADLINE, counter_now, entry, park_deadline, set_status, write_u64};
        use std::time::{Duration, Instant};

        let (mut g, _) = guest_with_a_thread();
        let start = g.served.threads.values().next().unwrap().start;
        super::create_thread(&mut g, start, 0, 0, 0, &mut || Ok(())).unwrap();
        let ts: Vec<super::Thread> = g.served.threads.values().cloned().collect();
        assert_eq!(park_deadline(&g), None, "no blocked thread, no deadline");
        let now = counter_now(&g);
        for (t, ahead) in [(&ts[1], 500_000u64), (&ts[2], 100_000_000)] {
            set_status(&mut g, t.index, BLOCKED);
            write_u64(g.memory_view(), entry(t.index) + E_DEADLINE, now + ahead);
        }
        let before = Instant::now();
        let at = park_deadline(&g).expect("a deadline");
        assert!(
            at > before && at <= before + Duration::from_millis(60),
            "the park's deadline {:?} ahead, not the earliest's 50 ms",
            at - before
        );
    }

    /// With no thread ready, a timed waiter is readied as `_switch` and `warp_to_deadline` ready one (Sprint 234): on the
    /// instruction clock the earliest deadline, then the lowest id, its warp moving the clock to that deadline, and no
    /// warp for a deadline already passed; on the host's clock the first waiter in rotation.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn a_timed_waiter_is_readied_as_the_frozen_host_readies_one() {
        use super::{BLOCKED, CLOCK_MODE, CLOCK_RATE, CLOCK_WARP, E_DEADLINE, E_STATUS, READY, entry, timed_waiter};
        use super::{RUN_TABLE, read_u64, write_u64};
        use std::time::Duration;

        let (mut g, _) = guest_with_a_thread();
        let mut ts: Vec<super::Thread> = g.served.threads.values().cloned().collect();
        ts.sort_by_key(|t| t.tid);
        let (a, b) = (ts[ts.len() - 2].clone(), ts[ts.len() - 1].clone());
        let set = |g: &mut crate::guest::Guest, da: u64, db: u64| {
            // blocked through `set_status`, which keeps the run table's count of blocked threads
            for (t, d) in [(&a, da), (&b, db)] {
                super::set_status(g, t.index, BLOCKED);
                write_u64(g.memory_view(), entry(t.index) + E_DEADLINE, d);
            }
            let mem = g.memory_view();
            write_u64(mem, g.served.vcpu.monitor + CLOCK_MODE, 2);
            write_u64(mem, g.served.vcpu.monitor + CLOCK_RATE, 1_400_000_000);
            write_u64(mem, g.served.vcpu.monitor, 1_400_000_000); // one second: the clock reads 10,000,000
            write_u64(mem, RUN_TABLE + CLOCK_WARP, 0);
        };
        let status = |g: &crate::guest::Guest, t: &super::Thread| read_u64(g.memory_view(), entry(t.index) + E_STATUS);
        let warp = |g: &crate::guest::Guest| read_u64(g.memory_view(), RUN_TABLE + CLOCK_WARP);
        let both = [a.clone(), b.clone()];

        set(&mut g, 30_000_000, 20_000_000);
        assert_eq!(
            timed_waiter(&mut g, &both, Duration::ZERO).tid,
            b.tid,
            "the earliest deadline"
        );
        assert_eq!((warp(&g), status(&g, &b), status(&g, &a)), (10_000_000, READY, BLOCKED));

        set(&mut g, 20_000_000, 20_000_000);
        assert_eq!(
            timed_waiter(&mut g, &both, Duration::ZERO).tid,
            a.tid,
            "a tie goes to the lower id"
        );

        set(&mut g, 5_000_000, 20_000_000);
        assert_eq!(timed_waiter(&mut g, &both, Duration::ZERO).tid, a.tid);
        assert_eq!(warp(&g), 0, "a deadline already passed moves no clock");

        set(&mut g, 30_000_000, 20_000_000);
        write_u64(g.memory_view(), g.served.vcpu.monitor + CLOCK_MODE, 0);
        assert_eq!(
            timed_waiter(&mut g, &both, Duration::ZERO).tid,
            a.tid,
            "the host's clock: the first in rotation"
        );
        assert_eq!(warp(&g), 0);
    }

    /// HOST_UNIMPLEMENTED (request 4) names the service by its API id and form and keeps the call's return address, the
    /// word at the saved RSP, for the record; an RSP that is not mapped is a failure. HOST_CLOCK (request 3) answers the
    /// host's monotonic count in mode 0, `counter_now` (Sprint 219; reviewer-3's Medium 2 of its review of Sprint 219).
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn requests_three_and_four_answer_as_the_frozen_host() {
        use crate::host_request::{HostError, unimplemented};
        use guest_runtime::layout::HEAP_VA;

        let (mut g, _) = guest_with_a_thread();
        let names = guest_runtime::services::api_names(&g.services).unwrap();
        let at = g.layout.offset(HEAP_VA).unwrap() as usize;
        let slot = (g.served.vcpu.state + 8 * 4) as usize;
        let mem = g.memory.as_mut_slice();
        mem[slot..slot + 8].copy_from_slice(&HEAP_VA.to_le_bytes());
        mem[at..at + 8].copy_from_slice(&0x1_4000_1234u64.to_le_bytes());
        let stop = unimplemented(&mut g, 2, 7);
        let service = format!("{} argument form 0x7", names[1]);
        assert!(matches!(&stop, HostError::NotServed(m) if *m == service), "{stop:?}");
        assert_eq!(g.served.unimplemented, Some((service, 0x1_4000_1234)));
        g.memory.as_mut_slice()[slot..slot + 8].copy_from_slice(&0x10u64.to_le_bytes());
        g.served.unimplemented = None;
        assert!(matches!(unimplemented(&mut g, 2, 7), HostError::Failed(_)));
        assert_eq!(g.served.unimplemented, None);
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
        let mut pe = g.served.images[0].pe.clone();
        pe.tls = Some(pe_image::Tls {
            start: pe.image_base,
            end: pe.image_base,
            index: pe.image_base,
            zero_fill: 0,
        });
        for n in 0..pe_loader::TLS_SLOTS - 1 {
            g.served.images.push(pe_loader::Image {
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
        let at = entry(g.served.threads[&tid].index);
        let mem = g.memory_view();
        write_u64(mem, at + E_X, 0x1111);
        write_u64(mem, at + E_X + 8 * 4, 0x7_0000_1000);
        write_u64(mem, at + E_X + 8 * 17, 0x1_4000_2000);
        write_u64(mem, at + E_X + 8 * 19, 1);
        write_u64(mem, at + E_X + 8 * u64::from(REG_UNDEFINED_FLAGS), 0);
        let lazy = at + E_MONITOR + E_MONITOR_AT[0];
        write_u64(mem, lazy, 0x10);
        write_u64(mem, lazy + 8, 0);
        write_u64(mem, lazy + 16, 0x03);
        mem.write(lazy + 0x18, &[1]);
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
        write_u64(g.memory_view(), at + E_STATUS, ENDED);
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
        let (dealloc, limit, _) = g.served.threads[&tid].stack;
        let teb = g.served.threads[&tid].teb;
        let stack_limit = |g: &Guest| {
            let at = g.layout.offset(teb + 0x10).unwrap() as usize;
            u64::from_le_bytes(g.memory.as_slice()[at..at + 8].try_into().unwrap())
        };
        let query = |g: &mut Guest, va: u64| {
            let view = GuestView {
                mem: crate::guest::memory_view(&g.memory),
                layout: &g.layout,
            };
            let info = g.served.arena.query(&view, va).unwrap();
            (info.state, info.protect)
        };
        let guarded = (MEM_COMMIT, PAGE_READWRITE | PAGE_GUARD);
        assert_eq!(stack_limit(&g), limit);
        assert_eq!(query(&mut g, limit - PAGE), guarded, "default.query.limit-1");
        // a touch of the first guard page: StackLimit one page down, three guard pages below, reserved below them
        assert_eq!(stack_grow(&mut g, tid, limit - PAGE + 8, false, &mut || Ok(())), Ok(1));
        let one = limit - PAGE;
        assert_eq!(stack_limit(&g), one, "default.grow moved_1");
        assert_eq!(g.served.threads[&tid].stack.1, one);
        assert_eq!(query(&mut g, one), (MEM_COMMIT, PAGE_READWRITE), "default.grown.limit");
        for k in 1..=3 {
            assert_eq!(query(&mut g, one - k * PAGE), guarded, "default.grown.limit-{k}p");
        }
        assert_eq!(query(&mut g, one - 4 * PAGE).0, MEM_RESERVE, "default.grown.limit-4p");
        // a probe two pages below StackLimit: two pages, as two touches would
        assert_eq!(stack_grow(&mut g, tid, one - 2 * PAGE, true, &mut || Ok(())), Ok(1));
        let three = one - 2 * PAGE;
        assert_eq!(stack_limit(&g), three, "two pages more");
        // a probe five pages below, past the guard: a probe grows where a touch may not
        assert_eq!(stack_grow(&mut g, tid, three - 5 * PAGE, true, &mut || Ok(())), Ok(1));
        let eight = three - 5 * PAGE;
        assert_eq!(stack_limit(&g), eight, "five pages more");
        // a touch below the guard: an access violation, the stack unmoved (row skip)
        assert_eq!(stack_grow(&mut g, tid, eight - 4 * PAGE, false, &mut || Ok(())), Ok(0));
        assert_eq!(stack_limit(&g), eight, "skip: moved 0");
        // a probe that would bring the guard to the reservation's last page: 0 and nothing changed
        assert_eq!(stack_grow(&mut g, tid, dealloc + PAGE, true, &mut || Ok(())), Ok(0));
        assert_eq!(stack_limit(&g), eight);
        assert_eq!(query(&mut g, eight - 4 * PAGE).0, MEM_RESERVE, "no page committed");
    }
}
