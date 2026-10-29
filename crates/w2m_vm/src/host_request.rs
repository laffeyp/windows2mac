// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The guest runtime's requests to the host (Sprint 147): `hvc #8` with the transport page, carried from
//! `src/w2m/vm_backend.py` `_serve_host_request` (3808-3888) for the requests a fault's walk makes. Any other request
//! stops by name, as a request the host cannot serve stops the Python's run.

use std::fmt;

use guest_runtime::layout::{MONITOR_SIZE, TRANSPORT_OFF};
use hypervisor::{HvError, Vcpu};

use crate::guest::{Guest, GuestError, GuestView};

/// runtime.h 69-71, 324; vm_backend 282-316.
pub const HOST_WRITE: u64 = 1;
pub const HOST_EXIT: u64 = 2;
pub const HOST_RUNTIME_FAULT: u64 = 5;
pub const HOST_MODULE: u64 = 15;
pub const HOST_PROC: u64 = 16;
pub const HOST_MODULE_PATH: u64 = 17;
pub const HOST_MODULE_AT: u64 = 18;
/// virtual_memory.c's requests (vm_backend 285)
pub const HOST_VIRTUAL_ALLOC: u64 = 19;
pub const HOST_VIRTUAL_FREE: u64 = 20;
pub const HOST_VIRTUAL_PROTECT: u64 = 21;
pub const HOST_VIRTUAL_QUERY: u64 = 22;
pub const HOST_EXCEPTION: u64 = 31;
use crate::file_bridge::{
    FILE_DUP, FILE_FSTAT, FILE_LIST, FILE_LOOKUP, FILE_MKDIR, FILE_RMDIR, FILE_STAT, FILE_UNLINK,
};
/// vm_backend 283: the wall clock as FILETIME (process.c's `w2m_filetime_now`, when the clock is the host's)
pub const HOST_FILETIME: u64 = 14;
/// vm_backend 287: a0 bytes of the host's CSPRNG at guest address a1 (bcrypt.c, com.c)
pub const HOST_RANDOM: u64 = 23;
/// vm_backend 288: a view of a0 bytes of file descriptor a1 with protection a2 (file.c)
pub const HOST_MAP_VIEW: u64 = 29;

unsafe extern "C" {
    /// `sys/random.h` 38
    fn getentropy(buffer: *mut std::ffi::c_void, size: usize) -> i32;
}
pub const HOST_THREAD: u64 = 30;
/// processes.c's and sync_objects.c's requests to `kernel.py` (vm_backend 314; Sprint 193)
pub const HOST_KERNEL: u64 = 33;
/// threads.c's sub-requests (runtime.h 328; vm_backend 332-333)
pub const THREAD_CREATE: u64 = 1;
pub const THREAD_EXIT_STUB: u64 = 2;
pub const THREAD_END: u64 = 3;
pub const THREAD_SUSPEND: u64 = 4;
pub const THREAD_RESUME: u64 = 5;
pub const THREAD_CONTEXT: u64 = 6;
pub const THREAD_EXIT_CODE: u64 = 7;
pub const THREAD_STACK_GROW: u64 = 9;
/// vm_backend 333 (Sprint 121): threads.c readied a thread while a vCPU waits (8, the retired signal request, unused)
pub const THREAD_WAKE: u64 = 10;
/// runtime.h 69: the host's clock (`W2M_HOST_CLOCK`, 3), and a service the runtime does not implement
/// (`W2M_HOST_UNIMPLEMENTED`, 4)
pub const HOST_CLOCK: u64 = 3;
pub const HOST_UNIMPLEMENTED: u64 = 4;
pub const EXCEPTION_STUBS: u64 = 1;
pub const EXCEPTION_CAPTURE_XMM: u64 = 2;

/// What a request asked of the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Served {
    /// answered: the guest goes on after its `hvc`
    Answered,
    /// the process ended with this code (HOST_EXIT)
    Exit(u32),
}

/// A request the host does not serve, or could not (reviewer-3's High 1 of its review of Sprint 211: one variant had
/// carried both, so a failure read as a service not implemented).
#[derive(Debug)]
pub enum HostError {
    /// a request w2m does not serve: one the Rust host has not carried yet, or one the frozen host refuses with
    /// `Unimplemented` (a descriptor not open, the arena's refusals); the run stops as unserved
    NotServed(String),
    /// a request w2m serves that failed in the serving (an unmapped buffer, a host call refused, a helper's error); the
    /// run stops as a fault. `modules` tells its refusals from its failures (`ModuleError`, Sprint 218); the helpers in
    /// `kernel`, `threads` and `file_bridge` return both as one string, so theirs land here until they carry typed errors
    Failed(String),
    Guest(GuestError),
    Hv(HvError),
}

impl From<crate::modules::ModuleError> for HostError {
    fn from(e: crate::modules::ModuleError) -> HostError {
        match e {
            crate::modules::ModuleError::NotServed(why) => HostError::NotServed(why),
            crate::modules::ModuleError::Failed(why) => HostError::Failed(why),
        }
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostError::NotServed(m) | HostError::Failed(m) => f.write_str(m),
            HostError::Guest(e) => write!(f, "{e:?}"),
            HostError::Hv(e) => write!(f, "{e:?}"),
        }
    }
}

impl std::error::Error for HostError {}

impl From<HvError> for HostError {
    fn from(e: HvError) -> HostError {
        HostError::Hv(e)
    }
}

impl From<GuestError> for HostError {
    fn from(e: GuestError) -> HostError {
        HostError::Guest(e)
    }
}

/// runtime.h `w2m_transport`: the op at +0, six arguments from +8 (the second answer in the sixth, +48), the result
/// at +56.
const TRANSPORT: usize = (MONITOR_SIZE + TRANSPORT_OFF) as usize;

/// `wake_all` (vm_backend 3700-3706): every blocked thread of this process checks its wait again.
pub fn wake_blocked(guest: &mut Guest) {
    let blocked: Vec<u64> = guest
        .threads
        .values()
        .map(|t| t.index)
        .filter(|&i| {
            let at = crate::threads::entry(i) + crate::threads::E_STATUS;
            read_u64(guest.memory.as_slice(), at as usize) == crate::threads::BLOCKED
        })
        .collect();
    for i in blocked {
        crate::threads::set_status(guest, i, crate::threads::READY);
    }
}

/// A child's KERNEL 33 request, served by the supervisor in its turn (Sprint 197): the result, with the second word
/// written and a pending wake applied; None in the supervisor's own process.
fn child_request(guest: &mut Guest, a: [u64; 4]) -> Result<Option<u64>, HostError> {
    let Some(crate::turns::Link::Child(link)) = &guest.link else {
        return Ok(None);
    };
    let (result, second, wake) = link.request(a, &guest.layout.maps).map_err(HostError::Failed)?;
    if wake {
        wake_blocked(guest);
    }
    let mem = guest.memory.as_mut_slice();
    mem[TRANSPORT + 48..TRANSPORT + 56].copy_from_slice(&second.to_le_bytes());
    // a pipe's bytes, which the supervisor wrote into this process's memory: the exit ledger's written ranges, as the
    // frozen host's child writes them through its own `_GuestView` (kernel.py 254, the one kernel write; Sprint 206)
    if a[0] == crate::kernel::PIPE_READ
        && second == 0
        && result > 0
        && let Some(at) = guest.layout.offset(a[2])
    {
        let data = guest.memory.as_slice()[at as usize..(at + result) as usize].to_vec();
        if let Some(l) = &mut guest.exit_ledger {
            l.wrote(a[2], &data);
        }
    }
    Ok(Some(result))
}

/// `environment_now` (vm_backend 3598-3612): the environment block's entries as the guest holds them now
/// (ProcessParameters.Environment, which environment.c moves on each change), what a child started with no environment
/// of its own gets (Sprint 199; reviewer-2's Low 1 of its review of 196-198: the parent's start block before).
fn environment_now(guest: &Guest) -> Result<Vec<String>, String> {
    use guest_runtime::layout::{ENVIRONMENT_OFF, ENVIRONMENT_SIZE, PROCESS_VA};
    let mem = guest.memory.as_slice();
    let parameters = guest
        .layout
        .offset(crate::process::PARAMETERS_VA + 0x80)
        .ok_or("ProcessParameters is not mapped")? as usize;
    let at = read_u64(mem, parameters);
    let off = at.wrapping_sub(PROCESS_VA);
    if !(ENVIRONMENT_OFF..=ENVIRONMENT_OFF + ENVIRONMENT_SIZE / 2).contains(&off) {
        return Err(format!("ProcessParameters.Environment at 0x{at:x}, outside its area"));
    }
    let start = (MONITOR_SIZE + off) as usize;
    let block = &mem[start..start + (ENVIRONMENT_SIZE / 2) as usize];
    let units: Vec<u16> = block
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    let end = units
        .windows(2)
        .position(|w| w == [0, 0])
        .ok_or("the environment block has no end within its area")?;
    Ok(String::from_utf16_lossy(&units[..end])
        .split('\0')
        .filter(|e| !e.is_empty())
        .map(str::to_string)
        .collect())
}

/// A child's start as `spawner` makes it (kernel.py 437-468), for `w2m-vm --child`: the command line, its ids, the
/// inherited handles and current directory, the parent's environment now, process group, clock and program directory.
fn child_start(guest: &Guest, child: &crate::kernel::Spawn) -> Result<serde_json::Value, String> {
    let s = &guest.start;
    let environment = environment_now(guest)?;
    Ok(serde_json::json!({
        "command_line": child.command_line,
        "pid": child.pid,
        "tid": child.tid,
        "inherited": child.inherited.iter().map(|(h, k, o)| [*h, *k, *o]).collect::<Vec<_>>(),
        "directory": child.directory,
        "environment": environment,
        "process_group": s.process_group,
        "instruction_clock": s.instruction_clock.map(|(rate, epoch)| [rate, epoch]),
        "app_root": s.app_root.as_ref().map(|p| p.to_string_lossy().to_string()),
        "parent_image": s.image_name,
        "program_path": s.program_path.as_ref().map(|p| p.to_string_lossy().to_string()),
        // the parent's drive and read-only setting: the child's bridge shares them (kernel.py 460; Sprint 204)
        "drive": guest.files.as_ref().map(|f| f.drive.to_string_lossy().to_string()),
        "read_only": guest.files.as_ref().map(|f| f.read_only),
        "limited": guest.limited,
        // the parent's exit ledger, which the child's rows sit beside (Sprint 206)
        "exit_ledger": guest.exit_ledger.as_ref().map(|l| l.path.to_string_lossy().to_string()),
    }))
}

/// Guest memory as `kernel.rs` reads and writes it, through the layout.
struct KernelMemory<'a>(&'a mut Guest);

impl crate::kernel::Memory for KernelMemory<'_> {
    fn read(&self, va: u64, n: usize) -> Result<Vec<u8>, String> {
        let at = self
            .0
            .layout
            .offset(va)
            .ok_or_else(|| format!("a kernel request at unmapped 0x{va:x}"))? as usize;
        Ok(self.0.memory.as_slice()[at..at + n].to_vec())
    }
    fn write(&mut self, va: u64, data: &[u8]) -> Result<(), String> {
        let at = self
            .0
            .layout
            .offset(va)
            .ok_or_else(|| format!("a kernel request at unmapped 0x{va:x}"))? as usize;
        self.0.memory.as_mut_slice()[at..at + data.len()].copy_from_slice(data);
        // the exit ledger's written ranges, as the frozen host's fixture logs `_GuestView.write` (Sprint 206)
        if let Some(l) = &mut self.0.exit_ledger {
            l.wrote(va, data);
        }
        Ok(())
    }
}

/// MEMORY_BASIC_INFORMATION as vm_backend 4044-4045 packs it, `struct.pack("<QQIHHQIIIi", ...)`: BaseAddress 0,
/// AllocationBase 8, AllocationProtect 16, PartitionId and a pad 20-23 zero, RegionSize 24, State 32, Protect 36,
/// Type 40, the tail's pad 44 zero; 48 bytes (Sprint 190).
pub fn memory_basic_information(info: &virtual_memory::Info) -> [u8; 48] {
    let mut raw = [0u8; 48];
    raw[0..8].copy_from_slice(&info.base.to_le_bytes());
    raw[8..16].copy_from_slice(&info.allocation_base.to_le_bytes());
    raw[16..20].copy_from_slice(&info.allocation_protect.to_le_bytes());
    raw[24..32].copy_from_slice(&info.region_size.to_le_bytes());
    raw[32..36].copy_from_slice(&info.state.to_le_bytes());
    raw[36..40].copy_from_slice(&info.protect.to_le_bytes());
    raw[40..44].copy_from_slice(&info.kind.to_le_bytes());
    raw
}

fn read_u64(mem: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(mem[at..at + 8].try_into().unwrap())
}

/// `Modules.containing` (modules.py 149-158): the module holding `address`: the program's image, a synthetic image
/// (kernel32, kernelbase, ntdll, a system DLL; one page each), or a DLL the loader mapped; 0 for none.
pub fn containing(guest: &Guest, address: u64) -> u64 {
    if let Some((base, size)) = guest.image_range
        && base <= address
        && address < base + size
    {
        return base;
    }
    let p = &guest.process;
    let synthetic = [p.kernel32, p.kernelbase, p.ntdll]
        .into_iter()
        .chain(p.system.values().map(|(h, _)| *h));
    for h in synthetic {
        if h != 0 && h <= address && address < h + 0x1000 {
            return h;
        }
    }
    for &(base, size) in &guest.dll_ranges {
        if base <= address && address < base + size {
            return base;
        }
    }
    0
}

/// HOST_MAP_VIEW (vm_backend 3857-3864; Sprint 211): a view of `size` bytes of descriptor `fd` with `protect`, the
/// arena's pages for it, then the file's first `size` bytes copied in; the answer (second word 0, the view's base). A
/// descriptor not open stops by the frozen host's words. The copy is not in the exit ledger's written column: the
/// frozen host writes it with `self.mem.write` (vm_backend 3862), the loader's memory (2407), which its fixture does not
/// wrap (reviewer-3's Low 6 of its review of Sprint 210).
pub fn map_view(guest: &mut Guest, size: u64, fd: u64, protect: u64) -> Result<(u64, u64), HostError> {
    let bytes = guest
        .files
        .as_ref()
        .and_then(|f| f.view(fd, size))
        .ok_or_else(|| HostError::NotServed(format!("a file view of descriptor {fd}, which is not open")))?
        .map_err(HostError::Failed)?;
    let mut view = GuestView {
        mem: guest.memory.as_mut_slice(),
        layout: &guest.layout,
    };
    let base = guest
        .arena
        .map_view(&mut view, size, protect as u32)
        .map_err(|e| HostError::NotServed(format!("{e:?}")))?;
    let at = guest
        .layout
        .offset(base)
        .ok_or_else(|| HostError::Failed(format!("a file view at unmapped 0x{base:x}")))? as usize;
    guest.memory.as_mut_slice()[at..at + bytes.len()].copy_from_slice(&bytes);
    Ok((0, base))
}

/// HOST_UNIMPLEMENTED (vm_backend 3878-3883; Sprint 219): runtime.c 56's report that service id `a0` does not run
/// in argument form `a1`. The service is named from the runtime's API ids (`api_names`, id n the n-th, from 1) and
/// the form, its call's return address read from the word at the saved RSP; both are kept in `Guest::unimplemented`
/// for the record's SERVICE_UNIMPLEMENTED, and the run stops as a request not served. An RSP that is not mapped is a
/// failure, where the frozen host's read raises (reviewer-3's Medium 2 of its review of Sprint 219).
pub fn unimplemented(guest: &mut Guest, a0: u64, a1: u64) -> HostError {
    let names = match guest_runtime::services::api_names(&guest.services) {
        Ok(names) => names,
        Err(why) => return HostError::Failed(why),
    };
    let name = (a0 as usize)
        .checked_sub(1)
        .and_then(|i| names.get(i))
        .cloned()
        .unwrap_or_else(|| format!("api {a0}"));
    let service = format!("{name} argument form 0x{a1:x}");
    let Some((ret, _)) = guest.called_service() else {
        return HostError::Failed(format!("{service}: the saved RSP is not a mapped address"));
    };
    guest.unimplemented = Some((service.clone(), ret));
    HostError::NotServed(service)
}

/// `_serve_host_request` (3808-3888) for HOST_WRITE, HOST_EXIT, HOST_RUNTIME_FAULT, HOST_MODULE_AT, HOST_EXCEPTION, and
/// HOST_THREAD's THREAD_CREATE, THREAD_EXIT_STUB, THREAD_END, THREAD_EXIT_CODE and THREAD_STACK_GROW (the main
/// thread's).
pub fn serve(guest: &mut Guest, vcpu: &Vcpu) -> Result<Served, HostError> {
    let (op, a0, a1, a2, a3) = {
        let mem = guest.memory.as_slice();
        (
            read_u64(mem, TRANSPORT),
            read_u64(mem, TRANSPORT + 8),
            read_u64(mem, TRANSPORT + 16),
            read_u64(mem, TRANSPORT + 24),
            read_u64(mem, TRANSPORT + 32),
        )
    };
    let result = match op {
        HOST_EXIT => return Ok(Served::Exit((a0 & 0xFFFF_FFFF) as u32)),
        HOST_WRITE => {
            // `self.out += self.mem.read(a0, a1)`: a1 bytes of the guest's output at a0 (Sprint 155)
            let at = guest
                .layout
                .offset(a0)
                .ok_or_else(|| HostError::Failed(format!("a write from unmapped 0x{a0:x}")))?
                as usize;
            let bytes = guest.memory.as_slice()[at..at + a1 as usize].to_vec();
            guest.stdout.extend(bytes);
            a1
        }
        HOST_RUNTIME_FAULT => {
            return Err(HostError::Failed(format!(
                "guest runtime fault: ELR 0x{a0:x} ESR 0x{a1:x} (EC 0x{:x}), not in translated code or not a data abort",
                a1 >> 26 & 0x3F
            )));
        }
        HOST_MODULE_AT => {
            // the second answer: a DLL just loaded leaves its DllMain stub (`_pending_init`, vm_backend 3873-3874)
            let r = containing(guest, a0);
            let init = std::mem::take(&mut guest.pending_init);
            let mem = guest.memory.as_mut_slice();
            mem[TRANSPORT + 48..TRANSPORT + 56].copy_from_slice(&init.to_le_bytes());
            r
        }
        // `_module_request` (vm_backend 3889-3904; Sprint 189): the name or the path through guest memory, the answer
        // `crate::modules` gives, a refusal a stop by name; the second word a just-loaded DLL's DllMain stub, else 0
        HOST_MODULE | HOST_PROC | HOST_MODULE_PATH => {
            let read = |guest: &Guest, va: u64, n: u64| -> Result<Vec<u8>, HostError> {
                let at = guest
                    .layout
                    .offset(va)
                    .ok_or_else(|| HostError::Failed(format!("a module request's name at unmapped 0x{va:x}")))?
                    as usize;
                Ok(guest.memory.as_slice()[at..at + n as usize].to_vec())
            };
            let r = match op {
                HOST_MODULE => {
                    let units: Vec<u16> = read(guest, a0, 2 * a1)?
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|c| u16::from_le_bytes(*c))
                        .collect();
                    crate::modules::module(guest, &String::from_utf16_lossy(&units), a2 != 0)?
                }
                HOST_PROC => {
                    // latin-1: each byte its own code point (vm_backend 3895)
                    let name: String = read(guest, a1, a2)?.iter().map(|b| char::from(*b)).collect();
                    // GetProcAddress's answer as the capture's process sees it (`_traced_proc`, vm_backend 3895)
                    let va = crate::modules::proc(guest, a0, &name)?;
                    guest.traced_proc(va)?
                }
                _ => match crate::modules::file_name(guest, a0) {
                    // HOST_MODULE_PATH: buffer a1, a2 characters; the path's length, M64 for no module
                    None => u64::MAX,
                    Some(path) => {
                        let raw: Vec<u8> = path.encode_utf16().flat_map(u16::to_le_bytes).collect();
                        let n = raw.len().min(2 * a2 as usize);
                        if a2 != 0 {
                            let at = guest.layout.offset(a1).ok_or_else(|| {
                                HostError::Failed(format!("a module path's buffer at unmapped 0x{a1:x}"))
                            })? as usize;
                            guest.memory.as_mut_slice()[at..at + n].copy_from_slice(&raw[..n]);
                            // the Python writes it through `_GuestView` (vm_backend 3904): the ledger's written ranges
                            if let Some(l) = &mut guest.exit_ledger {
                                l.wrote(a1, &raw[..n]);
                            }
                        }
                        path.encode_utf16().count() as u64
                    }
                },
            };
            // a DLL just loaded: its DllMain stub (vm_backend 3873-3874; Sprint 205)
            let init = std::mem::take(&mut guest.pending_init);
            let mem = guest.memory.as_mut_slice();
            mem[TRANSPORT + 48..TRANSPORT + 56].copy_from_slice(&init.to_le_bytes());
            r
        }
        // `_virtual_request` (vm_backend 4028-4047) and its dispatch (3865-3869; Sprint 190): the arena's answer, the
        // second word (the error, or VirtualProtect's old protection with the error above it) at the transport's 48
        HOST_VIRTUAL_ALLOC | HOST_VIRTUAL_FREE | HOST_VIRTUAL_PROTECT | HOST_VIRTUAL_QUERY => {
            let mut view = GuestView {
                mem: guest.memory.as_mut_slice(),
                layout: &guest.layout,
            };
            let refused = |e: virtual_memory::Unimplemented| HostError::NotServed(format!("{e:?}"));
            let (result, second) = match op {
                HOST_VIRTUAL_ALLOC => {
                    let (address, err) = guest
                        .arena
                        .alloc(&mut view, a0, a1, a2 as u32, a3 as u32)
                        .map_err(refused)?;
                    (address, u64::from(err))
                }
                HOST_VIRTUAL_FREE => {
                    let (ok, err) = guest.arena.free(&mut view, a0, a1, a2 as u32).map_err(refused)?;
                    (ok, u64::from(err))
                }
                HOST_VIRTUAL_PROTECT => {
                    let (ok, old, err) = guest.arena.protect(&view, a0, a1, a2 as u32).map_err(refused)?;
                    (ok, u64::from(old) | u64::from(err) << 32)
                }
                _ => {
                    // HOST_VIRTUAL_QUERY: a1 is the guest's buffer; the arena's refusal (virtual_memory.py 253's
                    // Unimplemented, an address outside the arena and the images) stops by name, as the Python's does
                    let info = guest.arena.query(&view, a0).map_err(refused)?;
                    let raw = memory_basic_information(&info);
                    let at = guest
                        .layout
                        .offset(a1)
                        .ok_or_else(|| HostError::Failed(format!("VirtualQuery's buffer at unmapped 0x{a1:x}")))?
                        as usize;
                    guest.memory.as_mut_slice()[at..at + raw.len()].copy_from_slice(&raw);
                    (raw.len() as u64, 0)
                }
            };
            let mem = guest.memory.as_mut_slice();
            mem[TRANSPORT + 48..TRANSPORT + 56].copy_from_slice(&second.to_le_bytes());
            result
        }
        // `kernel.py`'s objects for this process (vm_backend 3819-3831; Sprint 193): the answer and its second word, and
        // on a wake every blocked thread ready (`wake_all`, 3700-3706)
        HOST_KERNEL => {
            // a child's request goes to the supervisor, which serves it against the child's shared memory in the
            // supervisor's turn (Sprint 197)
            if let Some(result) = child_request(guest, [a0, a1, a2, a3])? {
                result
            } else {
                let pid = guest.process.process_id;
                let mut objects = std::mem::take(&mut guest.kernel);
                objects.add(pid);
                let answered = objects.request(pid, [a0, a1, a2, a3], &mut KernelMemory(guest));
                guest.kernel = objects;
                let answer = answered.map_err(HostError::Failed)?;
                if let Some(child) = &answer.spawn {
                    // `spawner` (kernel.py 437-468): a second `w2m-vm`, run in its turn (Sprint 197)
                    let start = child_start(guest, child).map_err(HostError::Failed)?;
                    if guest.link.is_none() {
                        guest.link = Some(crate::turns::Link::Supervisor(
                            crate::turns::Supervisor::new(pid, guest.start.instruction_clock.is_some())
                                .map_err(HostError::Failed)?,
                        ));
                    }
                    if let Some(crate::turns::Link::Supervisor(s)) = &mut guest.link {
                        s.spawn(child, start).map_err(HostError::Failed)?;
                    }
                    // a parent shares the vCPU with its children from here: time slices (`_set_budget`)
                    crate::threads::set_budget(guest);
                }
                if answer.wake {
                    wake_blocked(guest);
                    if let Some(crate::turns::Link::Supervisor(s)) = &guest.link {
                        s.wake_children();
                    }
                }
                let mem = guest.memory.as_mut_slice();
                mem[TRANSPORT + 48..TRANSPORT + 56].copy_from_slice(&answer.second.to_le_bytes());
                answer.result
            }
        }
        // the file primitives (vm_backend 3811-3818; Sprint 204): the process's file bridge, the second word at the
        // transport's 48
        op if (FILE_LOOKUP..=FILE_UNLINK).contains(&op)
            || matches!(
                op,
                FILE_STAT | FILE_MKDIR | FILE_RMDIR | FILE_FSTAT | FILE_DUP | FILE_LIST
            ) =>
        {
            let Some(mut files) = guest.files.take() else {
                return Err(HostError::Failed(
                    "a file call in a run with no file-system root".into(),
                ));
            };
            let served = files.request(&mut KernelMemory(guest), op, [a0, a1, a2, a3]);
            guest.files = Some(files);
            let (result, second) = served.map_err(HostError::Failed)?;
            let mem = guest.memory.as_mut_slice();
            mem[TRANSPORT + 48..TRANSPORT + 56].copy_from_slice(&second.to_le_bytes());
            result
        }
        // vm_backend 3840-3842 (Sprint 203): a0 bytes at a1, on the instruction clock from the process's generator
        // seeded 0x5EED (`random.Random(0x5EED).randbytes`), else from the host's CSPRNG as `os.urandom` reads it
        HOST_RANDOM => {
            let n = a0 as usize;
            let bytes = match &mut guest.random {
                Some(r) => r.randbytes(n),
                None => {
                    let mut b = vec![0u8; n];
                    for chunk in b.chunks_mut(256) {
                        // getentropy(2): 256 bytes at most a call
                        if unsafe { getentropy(chunk.as_mut_ptr().cast(), chunk.len()) } != 0 {
                            return Err(HostError::Failed(format!(
                                "getentropy: {}",
                                std::io::Error::last_os_error()
                            )));
                        }
                    }
                    b
                }
            };
            crate::kernel::Memory::write(&mut KernelMemory(guest), a1, &bytes).map_err(HostError::Failed)?;
            a0
        }
        // vm_backend 3843-3844 (Sprint 218): the system time, 100 ns intervals since 1601-01-01, as
        // GetSystemTimeAsFileTime gives it; a probe's static C runtime reads it as it starts
        HOST_FILETIME => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| HostError::Failed(format!("the wall clock: {e}")))?;
            (now.as_nanos() / 100) as u64 + crate::file_bridge::FILETIME_UNIX_EPOCH
        }
        HOST_MAP_VIEW => {
            let (second, base) = map_view(guest, a0, a1, a2)?;
            let mem = guest.memory.as_mut_slice();
            mem[TRANSPORT + 48..TRANSPORT + 56].copy_from_slice(&second.to_le_bytes());
            base
        }
        HOST_EXCEPTION if a0 == EXCEPTION_STUBS => guest.guest_stubs()?,
        HOST_EXCEPTION if a0 == EXCEPTION_CAPTURE_XMM => {
            let mut bytes = Vec::with_capacity(256);
            for n in 0..16 {
                bytes.extend(vcpu.simd(n)?.to_le_bytes());
            }
            let at = guest
                .layout
                .offset(a1)
                .ok_or_else(|| HostError::Failed(format!("XMM capture to unmapped 0x{a1:x}")))?
                as usize;
            guest.memory.as_mut_slice()[at..at + 256].copy_from_slice(&bytes);
            0
        }
        HOST_EXCEPTION => {
            return Err(HostError::Failed(format!(
                "guest runtime: unknown exception request {a0}"
            )));
        }
        // `_thread_request` (vm_backend 3190-3195) and `_stack_grow` (3161-3188): the current thread's stack; the main
        // one's (the process area's TEB) is committed whole and not held in the arena: not grown; a created thread's,
        // `threads::stack_grow` (Sprint 213): 1 grown, 0 an access violation
        HOST_THREAD if a0 == THREAD_STACK_GROW => {
            let current = crate::threads::current_tid(guest);
            crate::threads::stack_grow(guest, current, a1, a2 != 0).map_err(HostError::Failed)?
        }
        // `_thread_request` (3190-3192, 3845-3847): the transport's a0 the sub-request, its a1-a3 the request's a0-a2
        HOST_THREAD if a0 == THREAD_CREATE => {
            crate::threads::create_thread(guest, a1, a2, (a3 & 0xFFFF_FFFF) as u32, a3 >> 32)
                .map_err(HostError::Failed)?
        }
        // `_thread_request` (3200-3211, Sprint 157): ExitThread's detach stub, W2mThreadEnd's exit code, a thread's
        // exit code by id
        HOST_THREAD if a0 == THREAD_EXIT_STUB => crate::threads::exit_stub(guest).map_err(HostError::Failed)?,
        HOST_THREAD if a0 == THREAD_END => {
            crate::threads::end_thread(guest, (a1 & 0xFFFF_FFFF) as u32).map_err(HostError::Failed)?;
            0
        }
        // `_thread_request` (3212-3223; Sprint 215): SuspendThread and ResumeThread, the previous count
        HOST_THREAD if a0 == THREAD_SUSPEND || a0 == THREAD_RESUME => {
            crate::threads::suspend(guest, (a1 & 0xFFFF_FFFF) as u32, a0 == THREAD_RESUME).map_err(HostError::Failed)?
        }
        // `_thread_request` (3224-3253; Sprint 215): GetThreadContext into the caller's CONTEXT at a2
        HOST_THREAD if a0 == THREAD_CONTEXT => {
            crate::threads::thread_context(guest, (a1 & 0xFFFF_FFFF) as u32, a2).map_err(HostError::Failed)?
        }
        HOST_THREAD if a0 == THREAD_EXIT_CODE => {
            u64::from(crate::threads::exit_code(guest, (a1 & 0xFFFF_FFFF) as u32).map_err(HostError::Failed)?)
        }
        // vm_backend 3196-3199: threads.c readied a thread while a vCPU waits; the frozen host counts the wake and
        // notifies the vCPUs that wait, and one vCPU has none to notify (Sprint 219: found by comparing the two hosts'
        // request lists, not by a run)
        HOST_THREAD if a0 == THREAD_WAKE => 0,
        HOST_THREAD => {
            return Err(HostError::NotServed(format!(
                "thread request {a0}: not served by the Rust host yet"
            )));
        }
        // vm_backend 3838-3839: the host's clock in 100 ns units, which process.c 43 asks when the clock is the host's
        // (mode 1), the monotonic count `counter_now` gives outside the instruction clock (Sprint 219)
        HOST_CLOCK => crate::threads::counter_now(guest),
        // vm_backend 3878-3883: runtime.c 56's report of a service the runtime does not implement in this argument
        // form; the run stops on the service's name and the form (Sprint 219: it stopped on "host request 4")
        HOST_UNIMPLEMENTED => return Err(unimplemented(guest, a0, a1)),
        _ => {
            return Err(HostError::NotServed(format!(
                "host request {op}: not served by the Rust host yet"
            )));
        }
    };
    let mem = guest.memory.as_mut_slice();
    mem[TRANSPORT + 56..TRANSPORT + 64].copy_from_slice(&result.to_le_bytes());
    Ok(Served::Answered)
}

#[cfg(test)]
mod tests {
    use super::{HostError, memory_basic_information};
    use crate::modules::ModuleError;

    /// A module refusal is a request not served, which ends a run as unserved; a module failure is a fault (Sprint 218;
    /// before, both were `Failed` and the batch run's steam_api64.dll refusal read as a fault).
    #[test]
    fn a_module_refusal_is_not_served_and_a_failure_is_a_fault() {
        let refused = HostError::from(ModuleError::NotServed(
            "LoadLibrary x.dll: a Windows DLL w2m does not model".into(),
        ));
        assert!(matches!(refused, HostError::NotServed(m) if m.contains("does not model")));
        let failed = HostError::from(ModuleError::Failed("x.dll: not a PE".into()));
        assert!(matches!(failed, HostError::Failed(m) if m == "x.dll: not a PE"));
    }

    /// `struct.pack("<QQIHHQIIIi", base, allocation_base, allocation_protect, 0, 0, region_size, state, protect, type,
    /// 0)` (vm_backend 4044-4045) for a committed read-write page of a reservation, its bytes written out by hand.
    #[test]
    fn virtual_query_writes_memory_basic_information_as_the_python_packs_it() {
        let info = virtual_memory::Info {
            base: 0x1_2345_6000,
            allocation_base: 0x1_2340_0000,
            allocation_protect: 0x04,
            region_size: 0x3000,
            state: 0x1000,
            protect: 0x04,
            kind: 0x2_0000,
        };
        let mut want = Vec::new();
        want.extend(0x1_2345_6000u64.to_le_bytes());
        want.extend(0x1_2340_0000u64.to_le_bytes());
        want.extend(4u32.to_le_bytes());
        want.extend([0u8; 4]);
        want.extend(0x3000u64.to_le_bytes());
        want.extend(0x1000u32.to_le_bytes());
        want.extend(4u32.to_le_bytes());
        want.extend(0x2_0000u32.to_le_bytes());
        want.extend([0u8; 4]);
        assert_eq!(memory_basic_information(&info).to_vec(), want);
    }

    /// HOST_MAP_VIEW as the batch run asks it at row 925 (Sprint 211; vm_backend 3857-3864): 0x3b9a00 bytes of an open
    /// file, PAGE_READONLY. The answer is a 64 KB-aligned base holding the file's first bytes, second word 0, and
    /// VirtualQuery there gives MEM_MAPPED with the protection; a descriptor not open stops by the frozen host's words.
    /// The guest needs the runtime image clang builds, so the test runs on the Mac.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn a_file_view_holds_the_files_bytes_in_a_mapped_region() {
        use super::{Guest, GuestView, KernelMemory, map_view};
        use crate::file_bridge::{FILE_OPEN, FileBridge};
        use crate::guest::vm_compiler_symbols;
        use crate::process::Start;
        use guest_runtime::layout::{HEAP_VA, RUNTIME_IMAGE_VA, runtime_defines};
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
        // the file: 0x3b9a00 bytes, each its offset's low byte plus one, so offset 0 holds 1
        let app = std::env::temp_dir().join(format!("w2m-map-view-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&app);
        std::fs::create_dir_all(&app).unwrap();
        let size = 0x3b_9a00u64;
        let bytes: Vec<u8> = (0..size).map(|i| (i as u8).wrapping_add(1)).collect();
        std::fs::write(app.join("data.unity3d"), &bytes).unwrap();
        g.files = Some(FileBridge::new(&app, true, None).unwrap());
        let name: Vec<u8> = "data.unity3d".encode_utf16().flat_map(u16::to_le_bytes).collect();
        crate::kernel::Memory::write(&mut KernelMemory(&mut g), HEAP_VA, &name).unwrap();
        let mut files = g.files.take().unwrap();
        let (fd, _) = files
            .request(&mut KernelMemory(&mut g), FILE_OPEN, [HEAP_VA, 12, 0, 0])
            .unwrap();
        g.files = Some(files);
        let refused = map_view(&mut g, size, fd + 1, 2).unwrap_err();
        assert_eq!(
            refused.to_string(),
            format!("a file view of descriptor {}, which is not open", fd + 1)
        );
        // the frozen host's Unimplemented: a request not served, not a failure (reviewer-3's High 1 of its review of 211)
        assert!(matches!(refused, super::HostError::NotServed(_)), "{refused:?}");
        let (second, base) = map_view(&mut g, size, fd, 2).unwrap();
        assert_eq!(base % 0x1_0000, 0, "64 KB-aligned");
        assert_eq!(second, 0, "the second word");
        let at = g.layout.offset(base).unwrap() as usize;
        assert_eq!(g.memory.as_slice()[at], 1, "the file's first byte at the base");
        assert_eq!(&g.memory.as_slice()[at..at + size as usize], &bytes[..]);
        let view = GuestView {
            mem: g.memory.as_mut_slice(),
            layout: &g.layout,
        };
        let info = g.arena.query(&view, base).unwrap();
        assert_eq!(
            (info.kind, info.protect, info.allocation_protect),
            (
                virtual_memory::MEM_MAPPED,
                virtual_memory::PAGE_READONLY,
                virtual_memory::PAGE_READONLY
            )
        );
        let _ = std::fs::remove_dir_all(&app);
    }
}
