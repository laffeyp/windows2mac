// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The flags a block leaves live (Sprint 254; moved from `w2m_vm::compile`, where Sprint 155 carried it from
//! `vm_backend.py`'s `_flags_live_out` and `_flags_live_in`, 4778-4825): from the block's direct successors, each scanned
//! on its straight path for the flags it reads before writing them. What the scan may read is the caller's: the run's
//! host compiler reads only pages already behind translated code; the translation cache reads all of the image's code.

use crate::lift::{Lifted, Op};
use crate::lower::{ALL_FLAGS, CompileError, insn_flags, nzcv_setter};

/// vm_backend 861: the instructions a successor is scanned for, at most.
pub const LIVENESS_SCAN: usize = 8;

/// A scan's failure: the address where the lowering names no flags for a condition, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanError {
    pub address: u64,
    pub error: CompileError,
}

/// `_flags_live_out`: the flags read after a block whose last instruction is `last` and which ends at `end`; a
/// successor whose address is not a constant reads all. `code(address)` gives the instruction there when the scan may
/// read it, and None where it must stop.
pub fn live_out(last: &Lifted, end: u64, code: &mut impl FnMut(u64) -> Option<Lifted>) -> Result<u32, ScanError> {
    let Some((cond, t)) = last.ops.iter().find_map(|op| match op {
        Op::Br { cond, t } => Some((*cond, *t)),
        _ => None,
    }) else {
        return live_in(end, code);
    };
    let Some(target) = last.ops.iter().find_map(|op| match op {
        Op::Imm { t: it, value } if *it == t => Some(*value),
        _ => None,
    }) else {
        return Ok(ALL_FLAGS);
    };
    let taken = live_in(target, code)?;
    Ok(taken | if cond == "always" { 0 } else { live_in(end, code)? })
}

/// `_flags_live_in`: the flags the code at `addr` reads before writing them, from at most `LIVENESS_SCAN`
/// instructions on its straight path, following an unconditional jump to a constant; every flag still undecided when
/// the scan stops is live.
pub fn live_in(mut addr: u64, code: &mut impl FnMut(u64) -> Option<Lifted>) -> Result<u32, ScanError> {
    let (mut live, mut undecided) = (0u32, ALL_FLAGS);
    let mut previous: Option<Lifted> = None;
    for _ in 0..LIVENESS_SCAN {
        let Some(ins) = code(addr) else {
            break;
        };
        let (read, written) = insn_flags(&ins, previous.as_ref().and_then(nzcv_setter))
            .map_err(|error| ScanError { address: addr, error })?;
        live |= read & undecided;
        undecided &= !(read | written);
        if undecided == 0 {
            return Ok(live);
        }
        let br = ins.ops.iter().find_map(|op| match op {
            Op::Br { cond, t } => Some((*cond, *t)),
            _ => None,
        });
        let Some((cond, t)) = br else {
            addr += ins.length;
            previous = Some(ins);
            continue;
        };
        let target = ins.ops.iter().find_map(|op| match op {
            Op::Imm { t: it, value } if *it == t => Some(*value),
            _ => None,
        });
        match (cond, target) {
            ("always", Some(target)) => {
                addr = target;
                previous = None;
            }
            _ => break,
        }
    }
    Ok(live | undecided)
}
