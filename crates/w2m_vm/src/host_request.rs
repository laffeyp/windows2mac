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
pub const HOST_THREAD: u64 = 30;
/// processes.c's and sync_objects.c's requests to `kernel.py` (vm_backend 314; Sprint 193)
pub const HOST_KERNEL: u64 = 33;
/// threads.c's sub-requests (runtime.h 328; vm_backend 332-333)
pub const THREAD_CREATE: u64 = 1;
pub const THREAD_EXIT_STUB: u64 = 2;
pub const THREAD_END: u64 = 3;
pub const THREAD_EXIT_CODE: u64 = 7;
pub const THREAD_STACK_GROW: u64 = 9;
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

/// A request the host does not serve, or could not.
#[derive(Debug)]
pub enum HostError {
    Unserved(String),
    Guest(GuestError),
    Hv(HvError),
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostError::Unserved(m) => f.write_str(m),
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
    let (result, second, wake) = link.request(a, &guest.layout.maps).map_err(HostError::Unserved)?;
    if wake {
        wake_blocked(guest);
    }
    let mem = guest.memory.as_mut_slice();
    mem[TRANSPORT + 48..TRANSPORT + 56].copy_from_slice(&second.to_le_bytes());
    Ok(Some(result))
}

/// A child's start as `spawner` makes it (kernel.py 437-468), for `w2m-vm --child`: the command line, its ids, the
/// inherited handles and current directory, the parent's environment, process group, clock and program directory.
fn child_start(guest: &Guest, child: &crate::kernel::Spawn) -> Result<serde_json::Value, String> {
    let s = &guest.start;
    Ok(serde_json::json!({
        "command_line": child.command_line,
        "pid": child.pid,
        "tid": child.tid,
        "inherited": child.inherited.iter().map(|(h, k, o)| [*h, *k, *o]).collect::<Vec<_>>(),
        "directory": child.directory,
        "environment": s.environment,
        "process_group": s.process_group,
        "instruction_clock": s.instruction_clock.map(|(rate, epoch)| [rate, epoch]),
        "app_root": s.app_root.as_ref().map(|p| p.to_string_lossy().to_string()),
        "parent_image": s.image_name,
        "program_path": s.program_path.as_ref().map(|p| p.to_string_lossy().to_string()),
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
                .ok_or_else(|| HostError::Unserved(format!("a write from unmapped 0x{a0:x}")))?
                as usize;
            let bytes = guest.memory.as_slice()[at..at + a1 as usize].to_vec();
            guest.stdout.extend(bytes);
            a1
        }
        HOST_RUNTIME_FAULT => {
            return Err(HostError::Unserved(format!(
                "guest runtime fault: ELR 0x{a0:x} ESR 0x{a1:x} (EC 0x{:x}), not in translated code or not a data abort",
                a1 >> 26 & 0x3F
            )));
        }
        HOST_MODULE_AT => {
            // the second answer: a DLL just loaded leaves its DllMain stub (`_pending_init`); none here
            let r = containing(guest, a0);
            let mem = guest.memory.as_mut_slice();
            mem[TRANSPORT + 48..TRANSPORT + 56].fill(0);
            r
        }
        // `_module_request` (vm_backend 3889-3904; Sprint 189): the name or the path through guest memory, the answer
        // `crate::modules` gives, a refusal a stop by name; DllMain's stub word 0, no DLL being loaded here
        HOST_MODULE | HOST_PROC | HOST_MODULE_PATH => {
            let read = |guest: &Guest, va: u64, n: u64| -> Result<Vec<u8>, HostError> {
                let at = guest
                    .layout
                    .offset(va)
                    .ok_or_else(|| HostError::Unserved(format!("a module request's name at unmapped 0x{va:x}")))?
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
                    crate::modules::module(guest, &String::from_utf16_lossy(&units), a2 != 0)
                        .map_err(HostError::Unserved)?
                }
                HOST_PROC => {
                    // latin-1: each byte its own code point (vm_backend 3895)
                    let name: String = read(guest, a1, a2)?.iter().map(|b| char::from(*b)).collect();
                    // GetProcAddress's answer as the capture's process sees it (`_traced_proc`, vm_backend 3895)
                    let va = crate::modules::proc(guest, a0, &name).map_err(HostError::Unserved)?;
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
                                HostError::Unserved(format!("a module path's buffer at unmapped 0x{a1:x}"))
                            })? as usize;
                            guest.memory.as_mut_slice()[at..at + n].copy_from_slice(&raw[..n]);
                        }
                        path.encode_utf16().count() as u64
                    }
                },
            };
            let mem = guest.memory.as_mut_slice();
            mem[TRANSPORT + 48..TRANSPORT + 56].fill(0);
            r
        }
        // `_virtual_request` (vm_backend 4028-4047) and its dispatch (3865-3869; Sprint 190): the arena's answer, the
        // second word (the error, or VirtualProtect's old protection with the error above it) at the transport's 48
        HOST_VIRTUAL_ALLOC | HOST_VIRTUAL_FREE | HOST_VIRTUAL_PROTECT | HOST_VIRTUAL_QUERY => {
            let mut view = GuestView {
                mem: guest.memory.as_mut_slice(),
                layout: &guest.layout,
            };
            let refused = |e: virtual_memory::Unimplemented| HostError::Unserved(format!("{e:?}"));
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
                        .ok_or_else(|| HostError::Unserved(format!("VirtualQuery's buffer at unmapped 0x{a1:x}")))?
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
                let answer = answered.map_err(HostError::Unserved)?;
                if let Some(child) = &answer.spawn {
                    // `spawner` (kernel.py 437-468): a second `w2m-vm`, run in its turn (Sprint 197)
                    let start = child_start(guest, child).map_err(HostError::Unserved)?;
                    if guest.link.is_none() {
                        guest.link = Some(crate::turns::Link::Supervisor(
                            crate::turns::Supervisor::new(pid, guest.start.instruction_clock.is_some())
                                .map_err(HostError::Unserved)?,
                        ));
                    }
                    if let Some(crate::turns::Link::Supervisor(s)) = &mut guest.link {
                        s.spawn(child, start).map_err(HostError::Unserved)?;
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
        HOST_EXCEPTION if a0 == EXCEPTION_STUBS => guest.guest_stubs()?,
        HOST_EXCEPTION if a0 == EXCEPTION_CAPTURE_XMM => {
            let mut bytes = Vec::with_capacity(256);
            for n in 0..16 {
                bytes.extend(vcpu.simd(n)?.to_le_bytes());
            }
            let at = guest
                .layout
                .offset(a1)
                .ok_or_else(|| HostError::Unserved(format!("XMM capture to unmapped 0x{a1:x}")))?
                as usize;
            guest.memory.as_mut_slice()[at..at + 256].copy_from_slice(&bytes);
            0
        }
        HOST_EXCEPTION => {
            return Err(HostError::Unserved(format!(
                "guest runtime: unknown exception request {a0}"
            )));
        }
        // `_thread_request` (vm_backend 3190-3195) and `_stack_grow` (3161-3188): the current thread's stack; the main
        // one's (the process area's TEB) is committed whole and not held in the arena: not grown. A created thread's
        // growth is not carried yet.
        HOST_THREAD if a0 == THREAD_STACK_GROW => {
            let current = crate::threads::current_tid(guest);
            let stack = guest.threads.get(&current).map_or(guest.process.stack, |t| t.stack);
            if guest.arena.reservations.contains_key(&stack.0) {
                return Err(HostError::Unserved(format!(
                    "thread {current}'s stack grown to 0x{a1:x}: a created thread's growth, not carried yet"
                )));
            }
            0
        }
        // `_thread_request` (3190-3192, 3845-3847): the transport's a0 the sub-request, its a1-a3 the request's a0-a2
        HOST_THREAD if a0 == THREAD_CREATE => {
            crate::threads::create_thread(guest, a1, a2, (a3 & 0xFFFF_FFFF) as u32, a3 >> 32)
                .map_err(HostError::Unserved)?
        }
        // `_thread_request` (3200-3211, Sprint 157): ExitThread's detach stub, W2mThreadEnd's exit code, a thread's
        // exit code by id
        HOST_THREAD if a0 == THREAD_EXIT_STUB => crate::threads::exit_stub(guest).map_err(HostError::Unserved)?,
        HOST_THREAD if a0 == THREAD_END => {
            crate::threads::end_thread(guest, (a1 & 0xFFFF_FFFF) as u32).map_err(HostError::Unserved)?;
            0
        }
        HOST_THREAD if a0 == THREAD_EXIT_CODE => {
            u64::from(crate::threads::exit_code(guest, (a1 & 0xFFFF_FFFF) as u32).map_err(HostError::Unserved)?)
        }
        HOST_THREAD => {
            return Err(HostError::Unserved(format!(
                "thread request {a0}: not served by the Rust host yet"
            )));
        }
        _ => {
            return Err(HostError::Unserved(format!(
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
    use super::memory_basic_information;

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
}
