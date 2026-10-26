// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The guest runtime's requests to the host (Sprint 147): `hvc #8` with the transport page, carried from
//! `src/w2m/vm_backend.py` `_serve_host_request` (3808-3888) for the requests a fault's walk makes. Any other request
//! stops by name, as a request the host cannot serve stops the Python's run.

use std::fmt;

use guest_runtime::layout::{MONITOR_SIZE, TRANSPORT_OFF};
use hypervisor::{HvError, Vcpu};

use crate::guest::{Guest, GuestError};

/// runtime.h 69-71, 324; vm_backend 282-316.
pub const HOST_WRITE: u64 = 1;
pub const HOST_EXIT: u64 = 2;
pub const HOST_RUNTIME_FAULT: u64 = 5;
pub const HOST_MODULE_AT: u64 = 18;
pub const HOST_EXCEPTION: u64 = 31;
pub const HOST_THREAD: u64 = 30;
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
