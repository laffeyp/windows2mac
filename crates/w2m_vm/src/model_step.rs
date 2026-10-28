// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The model's exits served (Sprint 181; moved from `cases.rs`, reviewer-2's Medium 2 of its review of 175-177):
//! `_slow_sse` (vm_backend 4268-4345) for hvc 10 and 11, the VM's state into the model, one step, and the result back.
//! The case runner and the run loop call the same function, so the case runner's judged cases judge what the run loop
//! runs.

use guest_runtime::layout::{MONITOR_DATA, MONITOR_SIZE, SAVE_AREA, STATE_OFF};
use hypervisor::{HvError, Reg, Vcpu};

use crate::cases::{AF_OPERANDS, DF_BYTE, FLAG_BIT, FLAG_REG, PF_RESULT, X87_ST_OFF, X87_SW_OFF};
use crate::guest::Guest;
use crate::monitor_code::SAVE_VECTORS;
use crate::run::{FCW_OFF, MXCSR_OFF, REG_UNDEFINED_FLAGS, fpcr_for};

fn read_u64(mem: &[u8], off: u64) -> u64 {
    u64::from_le_bytes(mem[off as usize..off as usize + 8].try_into().unwrap())
}

fn write_u64(mem: &mut [u8], off: u64, v: u64) {
    mem[off as usize..off as usize + 8].copy_from_slice(&v.to_le_bytes());
}

/// PF from a result's low byte (vm_backend `x86_flags`).
fn parity_even(v: u64) -> bool {
    (v & 0xFF).count_ones().is_multiple_of(2)
}

/// vm_backend 1117: (FPSR bit, MXCSR bit): IOC IE, DZC ZE, OFC OE, UFC UE, IXC PE, IDC DE.
const FPSR_TO_MXCSR: [(u32, u32); 6] = [(0, 0), (1, 2), (2, 3), (3, 4), (4, 5), (7, 1)];

/// Guest memory as the model reads it (`loader.Memory`): an access wholly inside mapped, contiguous memory.
struct GuestModelMemory<'a>(&'a mut Guest);

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
        Ok(self.0.memory.as_slice()[at..at + n].to_vec())
    }

    fn write(&mut self, va: u64, data: &[u8]) -> Result<(), crate::model::Fault> {
        let at = self.span(va, data.len())?;
        self.0.memory.as_mut_slice()[at..at + data.len()].copy_from_slice(data);
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
    let mem = guest.memory.as_slice();
    let undefined = x[REG_UNDEFINED_FLAGS as usize];
    let defined = |f: char| undefined >> FLAG_BIT.iter().find(|(k, _)| *k == f).unwrap().1 & 1 == 0;
    let reg_flag = |f: char| defined(f).then(|| x[usize::from(FLAG_REG.iter().find(|(k, _)| *k == f).unwrap().1)] != 0);
    let (af_a, af_b, pf_r) = (
        read_u64(mem, md + AF_OPERANDS),
        read_u64(mem, md + AF_OPERANDS + 8),
        read_u64(mem, md + PF_RESULT),
    );
    let stored = u32::from_le_bytes(
        mem[(md + MXCSR_OFF) as usize..(md + MXCSR_OFF) as usize + 4]
            .try_into()
            .unwrap(),
    );
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
        df: mem[(md + DF_BYTE) as usize] != 0,
        mxcsr: stored | mxcsr_flags_from_fpsr(fpsr) as u32,
        fcw: u16::from_le_bytes(
            mem[(md + FCW_OFF) as usize..(md + FCW_OFF) as usize + 2]
                .try_into()
                .unwrap(),
        ),
        rip,
        xmm: [0; 16],
        x87_sw: 0,
        x87_tags: 0,
        x87_regs: [0; 8],
    };
    for k in 0..16u8 {
        th.xmm[usize::from(k)] = vcpu.simd(k)?;
    }
    // the x87 state as `_slow_sse` reads it (4293-4298): FSW and the abridged tags from the state block, ST(i) in FXSAVE
    // order, physical register (TOP + i) & 7
    let st = (MONITOR_SIZE + STATE_OFF) as usize;
    th.x87_sw = u16::from_le_bytes(
        mem[st + X87_SW_OFF as usize..st + X87_SW_OFF as usize + 2]
            .try_into()
            .unwrap(),
    );
    th.x87_tags = mem[st + X87_SW_OFF as usize + 2];
    let top = usize::from(th.x87_sw >> 11 & 7);
    for i in 0..8 {
        let at = st + X87_ST_OFF as usize + 16 * i;
        let mut b = [0u8; 16];
        b[..10].copy_from_slice(&mem[at..at + 10]);
        th.x87_regs[(top + i) & 7] = u128::from_le_bytes(b);
    }
    let result = step(&mut th, &mut GuestModelMemory(guest), &ins, code);
    let mem = guest.memory.as_mut_slice();
    mem[(md + MXCSR_OFF) as usize..(md + MXCSR_OFF) as usize + 4].copy_from_slice(&th.mxcsr.to_le_bytes());
    // `_store_x87` (4325-4330), after a step or a fault alike (a faulting MMX store has set TOP 0, set 065b)
    mem[st + X87_SW_OFF as usize..st + X87_SW_OFF as usize + 2].copy_from_slice(&th.x87_sw.to_le_bytes());
    mem[st + X87_SW_OFF as usize + 2] = th.x87_tags;
    let top = usize::from(th.x87_sw >> 11 & 7);
    for i in 0..8 {
        let at = st + X87_ST_OFF as usize + 16 * i;
        mem[at..at + 10].copy_from_slice(&th.x87_regs[(top + i) & 7].to_le_bytes()[..10]);
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
            write_u64(mem, md + AF_OPERANDS, if f.a == Some(true) { 0x10 } else { 0 });
            write_u64(mem, md + AF_OPERANDS + 8, 0);
            write_u64(mem, md + PF_RESULT, if f.p == Some(true) { 0 } else { 1 });
            mem[(md + DF_BYTE) as usize] = u8::from(th.df);
            mem[(md + FCW_OFF) as usize..(md + FCW_OFF) as usize + 2].copy_from_slice(&th.fcw.to_le_bytes());
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
                write_u64(mem, save + 8 * i as u64, *v);
            }
            write_u64(mem, save + 0x118, u64::from(code));
            write_u64(mem, save + 0x120, rip);
            for k in 0..16u8 {
                let q = vcpu.simd(k)?;
                let at = (save + SAVE_VECTORS + 16 * u64::from(k)) as usize;
                mem[at..at + 16].copy_from_slice(&q.to_le_bytes());
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
