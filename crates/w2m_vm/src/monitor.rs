// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The monitor area in a VM, and a call into the guest runtime (Sprint 134). The area is the first `MONITOR_SIZE`
//! bytes of guest memory, at `IPA_BASE` (vm_backend 2421: `(MONITOR_VA, IPA_BASE, MONITOR_SIZE)`); the runtime image
//! sits at `RUNTIME_IMAGE` in it (vm_backend 2417-2419).

use guest_runtime::Image;
use guest_runtime::layout::{IPA_BASE, MONITOR_SIZE, MONITOR_VA, RUNTIME_IMAGE, RUNTIME_STACK, RUNTIME_STACK_SIZE};
use hypervisor::{Exit, GuestMemory, HvError, Reg, Vcpu, Vm, memory, sys_reg};

use crate::monitor_code::{self, MonitorCode};
use crate::tables::{TableError, write_translation_tables};
use crate::{
    ACTLR_ENTSO, CPACR_FPEN, CPSR_EL1H, MAIR, RETURN_HVC, RETURN_STUB, SCTLR, TCR, VECTOR_HVC_BASE, VECTORS, hvc,
};

/// Why the monitor could not be set up or a call failed.
#[derive(Debug)]
pub enum MonitorError {
    Hv(HvError),
    Memory(std::io::Error),
    Tables(TableError),
    /// A runtime segment outside the image's area or a symbol the image lacks.
    Image(String),
    /// The vCPU stopped somewhere other than the return stub: the exit, and ESR_EL1, FAR_EL1 and ELR_EL1 when a
    /// vector was taken.
    Stopped {
        exit: Exit,
        esr: u64,
        far: u64,
        elr: u64,
    },
}

impl From<HvError> for MonitorError {
    fn from(e: HvError) -> Self {
        MonitorError::Hv(e)
    }
}

/// The monitor area's memory with the runtime image and the tables in it, ready to map.
pub struct MonitorMemory {
    pub memory: GuestMemory,
    pub ttbr: u64,
}

impl MonitorMemory {
    /// The area with `image` copied where it was linked, the vectors and the return stub written, and stage-1 tables
    /// for the area at `MONITOR_VA`; `omit` leaves those maps' VA ranges out of the tables (the planted fault).
    pub fn new(image: &Image, omit: &[(u64, u64)]) -> Result<MonitorMemory, MonitorError> {
        let mut memory = Self::with_image(image)?;
        let mem = memory.as_mut_slice();
        for slot in 0..16u16 {
            let at = (VECTORS + 0x80 * u64::from(slot)) as usize;
            mem[at..at + 4].copy_from_slice(&hvc(VECTOR_HVC_BASE + slot).to_le_bytes());
        }
        Self::finish(memory, omit, &[])
    }

    /// The area with `image` copied where it was linked and the Python host's monitor code written (Sprint 136):
    /// the vectors, the dispatcher, the empty dispatch table and the exception entry; tables for the area and for
    /// `extra` maps `(va, ipa, size)`.
    pub fn with_monitor_code(
        image: &Image,
        extra: &[(u64, u64, u64)],
    ) -> Result<(MonitorMemory, MonitorCode), MonitorError> {
        let mut memory = Self::with_image(image)?;
        let code = monitor_code::write(memory.as_mut_slice(), &image.symbols);
        Ok((Self::finish(memory, &[], extra)?, code))
    }

    fn with_image(image: &Image) -> Result<GuestMemory, MonitorError> {
        let mut memory = GuestMemory::new(MONITOR_SIZE as usize).map_err(MonitorError::Memory)?;
        let mem = memory.as_mut_slice();
        for (va, data) in &image.segments {
            let off = va
                .checked_sub(MONITOR_VA)
                .filter(|o| *o >= RUNTIME_IMAGE)
                .ok_or_else(|| MonitorError::Image(format!("a segment at {va:#x} below the image's area")))?
                as usize;
            mem.get_mut(off..off + data.len())
                .ok_or_else(|| MonitorError::Image(format!("a segment at {va:#x} past the monitor area")))?
                .copy_from_slice(data);
        }
        Ok(memory)
    }

    /// The return stub, then the tables: the area (less `omit`'s ranges) and `extra`.
    fn finish(
        mut memory: GuestMemory,
        omit: &[(u64, u64)],
        extra: &[(u64, u64, u64)],
    ) -> Result<MonitorMemory, MonitorError> {
        let mem = memory.as_mut_slice();
        let stub = RETURN_STUB as usize;
        mem[stub..stub + 4].copy_from_slice(&hvc(RETURN_HVC).to_le_bytes());
        let mut maps: Vec<(u64, u64, u64)> = if omit.is_empty() {
            vec![(MONITOR_VA, IPA_BASE, MONITOR_SIZE)]
        } else {
            // the area in 2 MB pieces, those `omit` covers left out
            (0..MONITOR_SIZE)
                .step_by(guest_runtime::layout::BLOCK as usize)
                .map(|o| (MONITOR_VA + o, IPA_BASE + o, guest_runtime::layout::BLOCK))
                .filter(|(va, _, size)| !omit.iter().any(|(s, e)| va < e && va + size > *s))
                .collect()
        };
        maps.extend_from_slice(extra);
        let (ttbr, _) = write_translation_tables(mem, &maps).map_err(MonitorError::Tables)?;
        Ok(MonitorMemory { memory, ttbr })
    }

    pub fn read_u32(&self, monitor_offset: u64) -> u32 {
        let o = monitor_offset as usize;
        u32::from_le_bytes(self.memory.as_slice()[o..o + 4].try_into().unwrap())
    }

    pub fn read_u64(&self, monitor_offset: u64) -> u64 {
        let o = monitor_offset as usize;
        u64::from_le_bytes(self.memory.as_slice()[o..o + 8].try_into().unwrap())
    }
}

/// Maps the monitor area into `vm` at `IPA_BASE`.
pub fn map(vm: &Vm, area: &MonitorMemory) -> Result<(), HvError> {
    vm.map(&area.memory, IPA_BASE, memory::READ | memory::WRITE | memory::EXEC)
}

/// Sets the vCPU's system registers as the Python host set them (vm_backend 4980-4981).
pub fn set_system_registers(vcpu: &Vcpu<'_>, ttbr: u64) -> Result<(), HvError> {
    for (reg, value) in [
        (sys_reg::CPACR_EL1, CPACR_FPEN),
        (sys_reg::MAIR_EL1, MAIR),
        (sys_reg::TCR_EL1, TCR),
        (sys_reg::TTBR0_EL1, ttbr),
        (sys_reg::VBAR_EL1, MONITOR_VA + VECTORS),
        (sys_reg::SCTLR_EL1, SCTLR),
        (sys_reg::ACTLR_EL1, ACTLR_ENTSO),
    ] {
        vcpu.set_sys_reg(reg, value)?;
    }
    vcpu.set_sys_reg(sys_reg::SP_EL1, MONITOR_VA + RUNTIME_STACK + RUNTIME_STACK_SIZE)
}

/// Calls the runtime function at `address` with `args` in x0 up, at EL1 on the runtime's stack, returning to the stub;
/// x0 after the return.
pub fn call(vcpu: &Vcpu<'_>, address: u64, args: &[u64]) -> Result<u64, MonitorError> {
    for (i, a) in args.iter().enumerate() {
        vcpu.set_reg(Reg::X(i as u8), *a)?;
    }
    vcpu.set_reg(Reg::X(30), MONITOR_VA + RETURN_STUB)?;
    vcpu.set_sys_reg(sys_reg::SP_EL1, MONITOR_VA + RUNTIME_STACK + RUNTIME_STACK_SIZE)?;
    vcpu.set_reg(Reg::Cpsr, CPSR_EL1H)?;
    vcpu.set_reg(Reg::Pc, address)?;
    loop {
        match vcpu.run()? {
            Exit::VtimerActivated => vcpu.set_vtimer_mask(true)?,
            e if e.hvc_immediate() == Some(RETURN_HVC) => return Ok(vcpu.reg(Reg::X(0))?),
            exit => {
                return Err(MonitorError::Stopped {
                    exit,
                    esr: vcpu.sys_reg(sys_reg::ESR_EL1)?,
                    far: vcpu.sys_reg(sys_reg::FAR_EL1)?,
                    elr: vcpu.sys_reg(sys_reg::ELR_EL1)?,
                });
            }
        }
    }
}
