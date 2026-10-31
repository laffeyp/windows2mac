// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The model's exits served (Sprint 181; moved from `cases.rs`, reviewer-2's Medium 2 of its review of 175-177):
//! `_slow_sse` (vm_backend 4268-4345) for hvc 10 and 11, the VM's state into the model, one step, and the result back.
//! The case runner and the run loop call the same function, so the case runner's judged cases judge what the run loop
//! runs.

use guest_runtime::layout::{MONITOR_DATA, MONITOR_SIZE, SAVE_AREA, STATE_OFF};
use hypervisor::{HvError, Reg, Vcpu};

use crate::cases::{AF_OPERANDS, DF_BYTE, FLAG_BIT, FLAG_REG, PF_RESULT, X87_ST_OFF, X87_SW_OFF, YMM_HI_OFF};
use crate::concurrent_memory::ConcurrentGuestMemory;
use crate::guest::Guest;
use crate::monitor_code::SAVE_VECTORS;
use crate::run::{FCW_OFF, MXCSR_OFF, REG_UNDEFINED_FLAGS, fpcr_for};

/// The `N` bytes at `off`, read through the view.
fn array<const N: usize>(mem: ConcurrentGuestMemory<'_>, off: u64) -> [u8; N] {
    let mut out = [0; N];
    mem.read(off, &mut out);
    out
}

/// PF from a result's low byte (vm_backend `x86_flags`).
fn parity_even(v: u64) -> bool {
    (v & 0xFF).count_ones().is_multiple_of(2)
}

/// vm_backend 1117: (FPSR bit, MXCSR bit): IOC IE, DZC ZE, OFC OE, UFC UE, IXC PE, IDC DE.
const FPSR_TO_MXCSR: [(u32, u32); 6] = [(0, 0), (1, 2), (2, 3), (3, 4), (4, 5), (7, 1)];

/// Guest memory as the model reads it (`loader.Memory`): an access wholly inside mapped, contiguous memory, made through
/// the view (Sprint 287, census D7), the guest's own loads and stores while another vCPU may write the same pages.
struct GuestModelMemory<'a>(&'a Guest);

impl GuestModelMemory<'_> {
    fn span(&self, va: u64, n: usize) -> Result<usize, crate::model::Fault> {
        let fault = || crate::model::Fault::Guest {
            code: crate::model::STATUS_ACCESS_VIOLATION,
            why: format!("access of {n} bytes at 0x{va:x} outside mapped memory"),
        };
        let first = self.0.layout.offset(va).ok_or_else(fault)?;
        let last = self.0.layout.offset(va + n as u64 - 1).ok_or_else(fault)?;
        if last != first + n as u64 - 1 {
            return Err(fault());
        }
        Ok(first as usize)
    }
}

impl crate::model::Memory for GuestModelMemory<'_> {
    fn read(&self, va: u64, n: usize) -> Result<Vec<u8>, crate::model::Fault> {
        let at = self.span(va, n)?;
        let mut out = vec![0; n];
        self.0.memory_view().read(at as u64, &mut out);
        Ok(out)
    }

    fn write(&mut self, va: u64, data: &[u8]) -> Result<(), crate::model::Fault> {
        let at = self.span(va, data.len())?;
        self.0.memory_view().write(at as u64, data);
        Ok(())
    }
}

/// What a model exit came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelStep {
    /// stepped; the vCPU resumes at the dispatcher
    Resumed,
    /// a guest fault raised through the runtime's exception path
    Raised,
    /// an operation the model does not run: the case runner counts it, the run loop stops by name
    Refused,
    /// a flag read while undefined: the run stops, as the Python's UndefinedFlag stops it
    Undefined(char),
}

/// `_slow_sse` (vm_backend 4268-4323): the VM's registers, flags, DF, MXCSR, FCW, XMM and x87 state into the model,
/// one step of the instruction at `rip` over guest memory, and the result back; the vCPU resumes at the dispatcher with
/// x17 the next RIP. A guest fault goes to the runtime's exception path as `_raise_in_guest` (4332-4342) sends it.
pub fn model_step(guest: &mut Guest, vcpu: &Vcpu<'_>, code: &[u8], rip: u64) -> Result<ModelStep, HvError> {
    use crate::model::{Fault, Flags, Thread, step};
    // the one lifter (Sprint 177: its MMX and x87 ops, which the model steps)
    let Ok(ins) = translator::lift::lift(code, rip) else {
        return Ok(ModelStep::Refused);
    };
    if ins.ops.iter().any(|op| matches!(op, translator::lift::Op::Cpuid)) {
        return Ok(ModelStep::Refused);
    }
    let md = MONITOR_DATA;
    let mut x = [0u64; 31];
    for (k, v) in x.iter_mut().enumerate() {
        *v = vcpu.reg(Reg::X(k as u8))?;
    }
    let mem = guest.memory_view();
    let undefined = x[REG_UNDEFINED_FLAGS as usize];
    let defined = |f: char| undefined >> FLAG_BIT.iter().find(|(k, _)| *k == f).unwrap().1 & 1 == 0;
    let reg_flag = |f: char| defined(f).then(|| x[usize::from(FLAG_REG.iter().find(|(k, _)| *k == f).unwrap().1)] != 0);
    let (af_a, af_b, pf_r) = (
        mem.read_u64(md + AF_OPERANDS),
        mem.read_u64(md + AF_OPERANDS + 8),
        mem.read_u64(md + PF_RESULT),
    );
    let stored = mem.read_u32(md + MXCSR_OFF);
    let fpsr = vcpu.reg(Reg::Fpsr)?;
    let mut th = Thread {
        regs: x[..16].try_into().unwrap(),
        // `x86_flags` (5368-5375): C Z S O from their registers, P and A lazy from the monitor data page
        flags: Flags {
            c: reg_flag('C'),
            z: reg_flag('Z'),
            s: reg_flag('S'),
            o: reg_flag('O'),
            p: defined('P').then(|| parity_even(pf_r)),
            a: defined('A').then_some((af_a ^ af_b ^ pf_r) & 0x10 != 0),
        },
        df: array::<1>(mem, md + DF_BYTE)[0] != 0,
        mxcsr: stored | mxcsr_flags_from_fpsr(fpsr) as u32,
        fcw: u16::from_le_bytes(array(mem, md + FCW_OFF)),
        rip,
        xmm: [0; 18],
        ymm_hi: [0; 16],
        x87_sw: 0,
        x87_tags: 0,
        x87_regs: [0; 8],
    };
    for k in 0..16u8 {
        th.xmm[usize::from(k)] = vcpu.simd(k)?;
    }
    // YMM0-15 bits 255:128 from the monitor data page (4282-4283)
    let ymm_hi = md + YMM_HI_OFF;
    for (i, v) in th.ymm_hi.iter_mut().enumerate() {
        *v = u128::from_le_bytes(array(mem, ymm_hi + 16 * i as u64));
    }
    // the x87 state as `_slow_sse` reads it (4293-4298): FSW and the abridged tags from the state block, ST(i) in FXSAVE
    // order, physical register (TOP + i) & 7
    let st = MONITOR_SIZE + STATE_OFF;
    th.x87_sw = u16::from_le_bytes(array(mem, st + X87_SW_OFF));
    th.x87_tags = array::<1>(mem, st + X87_SW_OFF + 2)[0];
    let top = usize::from(th.x87_sw >> 11 & 7);
    for i in 0..8 {
        let mut b = [0u8; 16];
        mem.read(st + X87_ST_OFF + 16 * i as u64, &mut b[..10]);
        th.x87_regs[(top + i) & 7] = u128::from_le_bytes(b);
    }
    let result = step(&mut th, &mut GuestModelMemory(guest), &ins, code);
    let mem = guest.memory_view();
    mem.write_u32(md + MXCSR_OFF, th.mxcsr);
    // `_store_x87` (4325-4330), after a step or a fault alike (a faulting MMX store has set TOP 0, set 065b)
    mem.write(st + X87_SW_OFF, &th.x87_sw.to_le_bytes());
    mem.write(st + X87_SW_OFF + 2, &[th.x87_tags]);
    let top = usize::from(th.x87_sw >> 11 & 7);
    for i in 0..8 {
        mem.write(
            st + X87_ST_OFF + 16 * i as u64,
            &th.x87_regs[(top + i) & 7].to_le_bytes()[..10],
        );
    }
    match result {
        Ok(()) => {
            for (k, v) in th.regs.iter().enumerate() {
                vcpu.set_reg(Reg::X(k as u8), *v)?;
            }
            let f = th.flags;
            let mut undefined = 0u64;
            for (name, value) in [('C', f.c), ('Z', f.z), ('S', f.s), ('O', f.o), ('P', f.p), ('A', f.a)] {
                let bit = FLAG_BIT.iter().find(|(k, _)| *k == name).unwrap().1;
                match value {
                    None => undefined |= 1 << bit,
                    Some(v) => {
                        if let Some((_, reg)) = FLAG_REG.iter().find(|(k, _)| *k == name) {
                            vcpu.set_reg(Reg::X(*reg), u64::from(v))?;
                        }
                    }
                }
            }
            vcpu.set_reg(Reg::X(REG_UNDEFINED_FLAGS), undefined)?;
            // `set_af_df` (2089-2092): lazy operands that read back as AF and PF, and DF
            mem.write_u64(md + AF_OPERANDS, if f.a == Some(true) { 0x10 } else { 0 });
            mem.write_u64(md + AF_OPERANDS + 8, 0);
            mem.write_u64(md + PF_RESULT, if f.p == Some(true) { 0 } else { 1 });
            mem.write(md + DF_BYTE, &[u8::from(th.df)]);
            mem.write(md + FCW_OFF, &th.fcw.to_le_bytes());
            // YMM0-15 bits 255:128 back (4315-4316); a fault leaves the page's halves as they were
            for (i, v) in th.ymm_hi.iter().enumerate() {
                mem.write(ymm_hi + 16 * i as u64, &v.to_le_bytes());
            }
            // the vectors with FPCR from MXCSR, FPSR's flags now in the stored word (4318)
            vcpu.set_reg(Reg::Fpcr, fpcr_for(th.mxcsr))?;
            // XMM0-15 back, as `_slow_sse` sets the vectors after a step (Sprint 178: the SSE slice writes them)
            for k in 0..16u8 {
                vcpu.set_simd(k, th.xmm[usize::from(k)])?;
            }
            vcpu.set_reg(Reg::Fpsr, 0)?;
            vcpu.set_reg(Reg::X(17), th.rip)?;
            vcpu.set_reg(Reg::Pc, guest.code.dispatch_va)?;
            Ok(ModelStep::Resumed)
        }
        Err(Fault::Guest { code, .. }) => {
            // `_raise_in_guest`: x0-x30 as the exit left them, the code and the RIP, the vectors, then the runtime's
            // exception call
            let save = md + SAVE_AREA;
            for (i, v) in x.iter().enumerate() {
                mem.write_u64(save + 8 * i as u64, *v);
            }
            mem.write_u64(save + 0x118, u64::from(code));
            mem.write_u64(save + 0x120, rip);
            for k in 0..16u8 {
                let q = vcpu.simd(k)?;
                mem.write(save + SAVE_VECTORS + 16 * u64::from(k), &q.to_le_bytes());
            }
            vcpu.set_reg(Reg::Fpsr, 0)?;
            vcpu.set_reg(Reg::Pc, guest.code.exception_call_va)?;
            Ok(ModelStep::Raised)
        }
        Err(Fault::Undefined(f)) => Ok(ModelStep::Undefined(f)),
        Err(Fault::Refused(_)) => Ok(ModelStep::Refused),
    }
}

/// `mxcsr_flags_from_fpsr` (vm_backend 1146-1147).
pub fn mxcsr_flags_from_fpsr(fpsr: u64) -> u64 {
    FPSR_TO_MXCSR
        .iter()
        .filter(|(f, _)| fpsr >> f & 1 == 1)
        .map(|(_, m)| 1u64 << m)
        .sum()
}
