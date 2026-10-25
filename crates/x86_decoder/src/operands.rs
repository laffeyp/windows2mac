// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The explicit operands of a decoded instruction (Sprint 144), carried from `src/w2m/x86ops.py`: the row's operand
//! template (SDM Vol 2D Appendix A notation) resolved against the decoded fields, in the xedshim format the Python
//! wrote, `R:<reg>`, `M:<seg>:<base>:<index>:<scale>:<disp>:<bytes>`, `A:<base>:<index>:<scale>:<disp>`,
//! `I:<width>:<hex>`, `J:<disp>`, `MM:<n>`, `ST:<n>`. Each item cites the lines it carries.

use crate::Fields;

/// x86ops 17-25.
pub const GPR64: [&str; 16] = [
    "RAX", "RCX", "RDX", "RBX", "RSP", "RBP", "RSI", "RDI", "R8", "R9", "R10", "R11", "R12", "R13", "R14", "R15",
];
pub const GPR32: [&str; 16] = [
    "EAX", "ECX", "EDX", "EBX", "ESP", "EBP", "ESI", "EDI", "R8D", "R9D", "R10D", "R11D", "R12D", "R13D", "R14D",
    "R15D",
];
pub const GPR16: [&str; 16] = [
    "AX", "CX", "DX", "BX", "SP", "BP", "SI", "DI", "R8W", "R9W", "R10W", "R11W", "R12W", "R13W", "R14W", "R15W",
];
pub const GPR8_REX: [&str; 16] = [
    "AL", "CL", "DL", "BL", "SPL", "BPL", "SIL", "DIL", "R8B", "R9B", "R10B", "R11B", "R12B", "R13B", "R14B", "R15B",
];
pub const GPR8_LEGACY: [&str; 8] = ["AL", "CL", "DL", "BL", "AH", "CH", "DH", "BH"];

/// Why the operands cannot be given (x86ops 28-29, and where the Python raised otherwise: an assertion, a size letter
/// SIZE has no entry for, a legacy byte register past BH).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperandError {
    /// the row has no operand template
    NoTemplate,
    /// the template cannot be resolved against these fields
    Unresolved(String),
}

fn fail<T>(m: String) -> Result<T, OperandError> {
    Err(OperandError::Unresolved(m))
}

/// `SIZE` (x86ops 22): a size letter's bits.
fn size(letter: &str) -> Option<u32> {
    match letter {
        "b" => Some(8),
        "w" => Some(16),
        "d" => Some(32),
        "q" => Some(64),
        "dq" => Some(128),
        _ => None,
    }
}

/// `_reg` (x86ops 32-35).
fn reg(n: u8, bits: u32, rex: u8) -> Result<String, OperandError> {
    let n = usize::from(n);
    let name = match bits {
        8 if rex != 0 => GPR8_REX.get(n),
        8 => GPR8_LEGACY.get(n),
        64 => GPR64.get(n),
        32 => GPR32.get(n),
        16 => GPR16.get(n),
        _ => None,
    };
    name.map(|s| s.to_string())
        .ok_or_else(|| OperandError::Unresolved(format!("register {n} at {bits} bits")))
}

/// `_bits` (x86ops 38-43).
fn bits(letter: &str, f: &Fields) -> Result<u32, OperandError> {
    match letter {
        "z" => Ok(if f.opsize == 16 { 16 } else { 32 }),
        // d, or q with REX.W (a 66 mandatory prefix does not narrow it)
        "y" => Ok(if f.rex >> 3 & 1 != 0 { 64 } else { 32 }),
        "v" => Ok(u32::from(f.opsize)),
        l => size(l).ok_or_else(|| OperandError::Unresolved(format!("size letter {l:?}"))),
    }
}

/// `_mem` (x86ops 46-66): `M` with `nbytes`, or `A` for `None`.
fn mem(f: &Fields, nbytes: Option<u32>) -> Result<String, OperandError> {
    let Some(modrm) = f.modrm else {
        return fail("a memory operand with no ModRM".into());
    };
    let (md, rm) = (modrm >> 6, modrm & 7);
    let names = if f.asize == 64 { &GPR64 } else { &GPR32 };
    let (mut base, mut index, mut scale) = ("INVALID", "INVALID", 1u32);
    if rm == 4 {
        let Some(sib) = f.sib else {
            return fail("a SIB operand with no SIB".into());
        };
        scale = 1 << (sib >> 6);
        let ix = ((sib >> 3) & 7) | ((f.rex >> 1) & 1) << 3;
        if ix != 4 {
            index = names[usize::from(ix)];
        }
        if !((sib & 7) == 5 && md == 0) {
            base = names[usize::from((sib & 7) | (f.rex & 1) << 3)];
        }
    } else if md == 0 && rm == 5 {
        base = if f.asize == 64 { "RIP" } else { "EIP" };
    } else {
        base = names[usize::from(rm | (f.rex & 1) << 3)];
    }
    let seg = f.prefixes.iter().rev().find_map(|p| match p {
        0x64 => Some("FS"),
        0x65 => Some("GS"),
        _ => None,
    });
    Ok(match nbytes {
        None => format!("A:{base}:{index}:{scale}:{}", f.disp),
        Some(n) => format!("M:{}:{base}:{index}:{scale}:{}:{n}", seg.unwrap_or("INVALID"), f.disp),
    })
}

fn need_modrm(f: &Fields, token: &str) -> Result<u8, OperandError> {
    f.modrm
        .ok_or_else(|| OperandError::Unresolved(format!("{token} with no ModRM")))
}

fn imm_le(f: &Fields) -> u64 {
    f.imm
        .iter()
        .enumerate()
        .fold(0, |v, (k, b)| v | u64::from(*b) << (8 * k))
}

/// `operands` (x86ops 69-155): the explicit operands, in template order.
pub fn operands(f: &Fields) -> Result<Vec<String>, OperandError> {
    let Some(template) = f.ops.filter(|t| !t.is_empty()) else {
        return Err(OperandError::NoTemplate);
    };
    if template == "-" {
        return Ok(Vec::new()); // no explicit operand (FSIN, FNCLEX, FWAIT, FNSTSW AX; Sprint 065)
    }
    let vec_name = |n: u8| format!("{}{n}", if f.vl != 0 { "YMM" } else { "XMM" });
    let mut out = Vec::new();
    for t in template.split(',') {
        if let Some(n) = t.strip_prefix("Mf") {
            // x87 memory of the template's bytes (Mf2 .. Mf10; Sprint 065)
            let m = need_modrm(f, t)?;
            if m >> 6 == 3 {
                return fail(format!("{t} with a register ModRM"));
            }
            out.push(mem(f, Some(n.parse().unwrap()))?);
            continue;
        }
        if t == "STi" {
            out.push(format!("ST:{}", need_modrm(f, t)? & 7));
            continue;
        }
        if t == "P" || t == "N" || t.starts_with('Q') {
            // MMX: P from ModRM.reg, N from r/m, Q from r/m or memory; REX.R and REX.B do not extend it
            let m = need_modrm(f, t)?;
            if t == "P" {
                out.push(format!("MM:{}", m >> 3 & 7));
            } else if m >> 6 == 3 {
                out.push(format!("MM:{}", m & 7));
            } else {
                let b = size(&t[1..]).ok_or_else(|| OperandError::Unresolved(format!("size of {t}")))?;
                out.push(mem(f, Some(b / 8))?);
            }
            continue;
        }
        let operand = if t == "Vx" {
            let m = need_modrm(f, t)?;
            format!("R:{}", vec_name(((m >> 3) & 7) | ((f.rex >> 2) & 1) << 3))
        } else if let Some(letter) = t.strip_prefix('B') {
            // a general register VEX.vvvv names (BMI; Sprint 049)
            let v = f
                .vvvv
                .ok_or_else(|| OperandError::Unresolved(format!("{t} with no vvvv")))?;
            format!("R:{}", reg(v ^ 0xF, bits(&letter[..1], f)?, 1)?)
        } else if t == "H" || t == "Hx" {
            let v = f
                .vvvv
                .ok_or_else(|| OperandError::Unresolved(format!("{t} with no vvvv")))?;
            if t == "Hx" {
                format!("R:{}", vec_name(v ^ 0xF))
            } else {
                format!("R:XMM{}", v ^ 0xF)
            }
        } else if t == "Wx" {
            let m = need_modrm(f, t)?;
            if m >> 6 == 3 {
                format!("R:{}", vec_name((m & 7) | (f.rex & 1) << 3))
            } else {
                mem(f, Some(if f.vl != 0 { 32 } else { 16 }))?
            }
        } else if t == "V" {
            let m = need_modrm(f, t)?;
            format!("R:XMM{}", ((m >> 3) & 7) | ((f.rex >> 2) & 1) << 3)
        } else if t == "Edb" {
            // r32 or m8 from ModRM.r/m (PINSRB; Sprint 050)
            let m = need_modrm(f, t)?;
            if m >> 6 == 3 {
                format!("R:{}", reg((m & 7) | (f.rex & 1) << 3, 32, f.rex)?)
            } else {
                mem(f, Some(1))?
            }
        } else if let Some(letters) = t.strip_prefix('W') {
            let m = need_modrm(f, t)?;
            if m >> 6 == 3 {
                format!("R:XMM{}", (m & 7) | (f.rex & 1) << 3)
            } else {
                let b = size(letters).ok_or_else(|| OperandError::Unresolved(format!("size of {t}")))?;
                mem(f, Some(b / 8))?
            }
        } else if let Some(letter) = t.strip_prefix('E') {
            let b = bits(&letter[..1], f)?;
            let m = need_modrm(f, t)?;
            if m >> 6 == 3 {
                format!("R:{}", reg((m & 7) | (f.rex & 1) << 3, b, f.rex)?)
            } else {
                mem(f, Some(b / 8))?
            }
        } else if let Some(letter) = t.strip_prefix('G') {
            let m = need_modrm(f, t)?;
            format!(
                "R:{}",
                reg(((m >> 3) & 7) | ((f.rex >> 2) & 1) << 3, bits(&letter[..1], f)?, f.rex)?
            )
        } else if let Some(letter) = t.strip_prefix('Z') {
            format!(
                "R:{}",
                reg((f.opcode & 7) | (f.rex & 1) << 3, bits(&letter[..1], f)?, f.rex)?
            )
        } else if t == "M" {
            mem(f, None)?
        } else if t == "Mcl" {
            mem(f, Some(64))? // a cache line, 64 bytes (PREFETCHh; Sprint 044)
        } else if t == "Mqdq" {
            mem(f, Some(if f.opsize == 64 { 16 } else { 8 }))? // CMPXCHG8B/16B (Sprint 046)
        } else if t.starts_with('I') {
            format!("I:{}:{:x}", f.imm.len(), imm_le(f))
        } else if t.starts_with('J') {
            let bits = 8 * f.imm.len() as u32;
            let u = imm_le(f);
            let v = if bits > 0 && bits < 64 && u >> (bits - 1) & 1 == 1 {
                (u | !0 << bits) as i64
            } else {
                u as i64
            };
            format!("J:{v}")
        } else if t == "1" {
            "I:1:1".to_string()
        } else if t == "AL" || t == "CL" {
            format!("R:{t}")
        } else if t == "rAX" {
            format!("R:{}", reg(0, u32::from(f.opsize), f.rex)?)
        } else {
            return fail(format!("operand notation {t:?} not handled"));
        };
        out.push(operand);
    }
    Ok(out)
}
