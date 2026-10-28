// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! x86-64 instructions to explicit operations (Sprint 151), carried from `src/w2m/lift.py` (v0.2 §5 machine-semantics
//! IR; Sprint 029). Each decoded instruction (`x86_decoder::decode`) with its operands (`x86_decoder::operands`, equal
//! to the C's on every record and sweep instruction, Sprint 144) becomes a short list of operations over numbered
//! temporaries, which `lower.rs` lowers to AArch64. The operations carried so far are those link 5's first group lifts
//! (`addr`, `imm`, `ld`, `st`, `alu`, `flags`, `br`); an instruction whose class has no Rust rule yet is a `LiftError`
//! naming it. Semantics follow the Intel SDM Vol 2 pages; a 32-bit register write zero-extends to 64 bits and 8- and
//! 16-bit writes keep the other bits (SDM Vol 1 3.4.1.1).

use std::fmt;

use x86_decoder::operands::{GPR8_LEGACY, GPR8_REX, GPR16, GPR32, GPR64};
use x86_decoder::{DecodeError, Fields, decode};

/// A numbered temporary (lift.py `_OpBuilder.t`).
pub type Temp = u32;

/// RSP, RAX, RDX, RCX, RSI, RDI, RBX (lift.py 90-92).
pub const RSP: u8 = 4;
pub const RAX: u8 = 0;
pub const RDX: u8 = 2;
pub const RCX: u8 = 1;
pub const RSI: u8 = 6;
pub const RDI: u8 = 7;
pub const RBX: u8 = 3;

/// A location (lift.py 10): ("r", index, bits, high8) or ("m", t_addr, bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Loc {
    R { idx: u8, bits: u32, hi: bool },
    M { addr: Temp, bytes: u32 },
}

/// One operation (lift.py 10-66), for the kinds the Rust rules emit so far.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// ("addr", t, base, index, scale, disp, abits, seg): base and index are register indices or None; a RIP-relative
    /// address is resolved at lift time into `disp` with no base
    Addr {
        t: Temp,
        base: Option<u8>,
        index: Option<u8>,
        scale: u32,
        disp: i64,
        abits: u32,
        gs: bool,
    },
    Imm {
        t: Temp,
        value: u64,
    },
    Ld {
        t: Temp,
        loc: Loc,
    },
    St {
        loc: Loc,
        t: Temp,
    },
    /// ("alu", t, op, a, b, bits)
    Alu {
        t: Temp,
        op: &'static str,
        a: Temp,
        b: Temp,
        bits: u32,
    },
    /// ("flags", kind, a, b, r, bits)
    Flags {
        kind: &'static str,
        a: Temp,
        b: Temp,
        r: Temp,
        bits: u32,
    },
    /// ("br", cond, t): cond "always" or an SDM condition code (Vol 1 B.1)
    Br {
        cond: &'static str,
        t: Temp,
    },
    /// ("set", cond, loc): SETcc
    Set {
        cond: &'static str,
        loc: Loc,
    },
    /// ("sel", t, c, x, y): t = x if temporary c is nonzero, else y
    Sel {
        t: Temp,
        c: Temp,
        x: Temp,
        y: Temp,
    },
    /// ("cst", c, loc, t): store t to loc when temporary c is nonzero
    Cst {
        c: Temp,
        loc: Loc,
        t: Temp,
    },
    /// ("cstz", c, loc, t): store t to loc when temporary c is zero
    Cstz {
        c: Temp,
        loc: Loc,
        t: Temp,
    },
    /// ("cmov", cond, loc, t): CMOVcc; a 32-bit register destination is zero-extended either way
    Cmov {
        cond: &'static str,
        loc: Loc,
        t: Temp,
    },
    /// ("div", bits, t[, signed]): DIV r/m, IDIV with signed
    Div {
        bits: u32,
        t: Temp,
        signed: bool,
    },
    /// ("ud",): an instruction the guest profile makes undefined (#UD)
    Ud,
    /// ("df", 0|1): CLD, STD
    Df {
        set: u32,
    },
    /// ("cpuid",): EAX, EBX, ECX, EDX from specs/guest_cpuid.json for EAX, ECX
    Cpuid,
    /// ("rep", kind, size): REP STOS / REP MOVS (SDM; Vol 1 7.3.9): while RCX != 0, store AL/AX/EAX/RAX (stos) or the
    /// element at [RSI] (movs) to [RDI], step RDI (and RSI) by size, down when DF is set, and decrement RCX;
    /// interruptible, so a fault leaves the registers at the faulting iteration (the one exception to the commit rule)
    Rep {
        kind: &'static str,
        size: u32,
    },
    /// ("mxcsr", "st"|"ld", loc): STMXCSR / LDMXCSR, MXCSR to or from 4 bytes of memory; LDMXCSR with a reserved bit
    /// (16-31) set faults before any write
    Mxcsr {
        store: bool,
        loc: Loc,
    },
    /// ("fence", kind): SFENCE "st", LFENCE "ld", MFENCE "full"
    Fence {
        kind: &'static str,
    },
    /// ("fcw", "st", loc): FNSTCW, the x87 control word to 2 bytes of memory
    Fcw {
        loc: Loc,
    },
    /// ("pause",): no architectural effect; the VM counts it
    Pause,
    /// ("xgetbv",): EDX:EAX = XCR0 (7) for ECX 0, #GP otherwise
    Xgetbv,
    /// ("cx16", t): CMPXCHG16B at the address in temporary t, one atomic read-and-write
    Cx16 {
        addr: Temp,
    },
    /// ("sse", iclass, dst, src, align[, imm]): one legacy-SSE instruction; dst and src are locations or XMM registers;
    /// align 16 when a memory operand must be 16-byte aligned (#GP otherwise); computed after every read and before
    /// any write (lift.py 52-56)
    Sse {
        ic: String,
        dst: SseLoc,
        src: SseLoc,
        align: u32,
        imm: Option<u64>,
    },
    /// ("x86", ic, dst, srcs, bits): one integer instruction the model computes whole (Sprint 050; integer_model.py):
    /// dst a loc or None, srcs the temporaries it reads; implicit operands are read by the model, as one model step
    X86 {
        ic: String,
        dst: Option<Loc>,
        srcs: Vec<Temp>,
        bits: u32,
    },
}

/// An SSE operand (lift.py 177-178): ("x", n) for XMM register n, or a general register or memory location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SseLoc {
    X(u8),
    Loc(Loc),
}

impl SseLoc {
    /// The memory operand's address temporary, if this is one.
    pub fn addr(&self) -> Option<Temp> {
        match self {
            SseLoc::Loc(Loc::M { addr, .. }) => Some(*addr),
            _ => None,
        }
    }
}

/// `_operand` (lift.py 175-178) for an SSE rule: an XMM register as ("x", n), anything else as `operand` reads it.
/// The YMM registers come with the VEX group.
pub fn sse_operand(b: &mut Builder, s: &str, next_ip: u64) -> Result<SseLoc, LiftError> {
    if let Some(n) = s.strip_prefix("R:XMM") {
        let n: u8 = n.parse().map_err(|_| LiftError(format!("operand {s}")))?;
        return Ok(SseLoc::X(n));
    }
    match operand(b, s, next_ip, None)? {
        (Operand::Loc(loc), _) => Ok(SseLoc::Loc(loc)),
        _ => Err(LiftError(format!("operand {s}: not a location"))),
    }
}

/// lift.py 154-159.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lifted {
    pub address: u64,
    pub length: u64,
    pub iclass: String,
    pub ops: Vec<Op>,
}

/// lift.py 150-151: the instruction decodes but has no lifting rule yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiftError(pub String);

impl fmt::Display for LiftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LiftError {}

/// `REG` (lift.py 79-89): a register name -> (index, bits, high8).
pub fn reg(name: &str) -> Option<(u8, u32, bool)> {
    let at = |table: &[&str]| table.iter().position(|n| *n == name).map(|i| i as u8);
    if let Some(i) = at(&GPR64) {
        return Some((i, 64, false));
    }
    if let Some(i) = at(&GPR32) {
        return Some((i, 32, false));
    }
    if let Some(i) = at(&GPR16) {
        return Some((i, 16, false));
    }
    if let Some(i) = at(&GPR8_REX) {
        return Some((i, 8, false));
    }
    at(&GPR8_LEGACY[4..]).map(|i| (i, 8, true)) // AH, CH, DH, BH: bits 8-15 of registers 0-3
}

/// `CC` (lift.py 95-96): the iclass suffix -> the condition code.
pub fn cc(suffix: &str) -> Option<&'static str> {
    const CC: [(&str, &str); 16] = [
        ("O", "o"),
        ("NO", "no"),
        ("B", "b"),
        ("NB", "nb"),
        ("Z", "z"),
        ("NZ", "nz"),
        ("BE", "be"),
        ("NBE", "nbe"),
        ("S", "s"),
        ("NS", "ns"),
        ("P", "p"),
        ("NP", "np"),
        ("L", "l"),
        ("NL", "nl"),
        ("LE", "le"),
        ("NLE", "nle"),
    ];
    CC.iter().find(|(k, _)| *k == suffix).map(|(_, v)| *v)
}

/// `ALU` (lift.py 145).
pub fn alu_op(iclass: &str) -> Option<&'static str> {
    Some(match iclass {
        "ADD" => "add",
        "SUB" | "CMP" => "sub",
        "AND" | "TEST" => "and",
        "OR" => "or",
        "XOR" => "xor",
        _ => return None,
    })
}

/// `_OpBuilder` (lift.py 162-172).
#[derive(Debug, Default)]
pub struct Builder {
    pub ops: Vec<Op>,
    n: Temp,
}

impl Builder {
    pub fn t(&mut self) -> Temp {
        self.n += 1;
        self.n
    }

    pub fn emit(&mut self, op: Op) {
        self.ops.push(op);
    }
}

/// What `_operand` gives: a location, an effective address's temporary, or an immediate's temporary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    Loc(Loc),
    Addr(Temp),
    Imm(Temp),
}

/// `_operand` (lift.py 175-214): one xedshim-format operand -> (kind, loc or temporary, bits).
pub fn operand(b: &mut Builder, s: &str, next_ip: u64, width_hint: Option<u32>) -> Result<(Operand, u32), LiftError> {
    let p: Vec<&str> = s.split(':').collect();
    let bad = || LiftError(format!("operand {s}"));
    if p[0] == "R" && (p[1].starts_with("XMM") || p[1].starts_with("YMM")) {
        // the vector registers come with the SSE and VEX groups
        return Err(LiftError(format!("operand {s}: vector registers are a later group")));
    }
    if p[0] == "R" {
        let (idx, bits, hi) = reg(p[1]).ok_or_else(bad)?;
        return Ok((Operand::Loc(Loc::R { idx, bits, hi }), bits));
    }
    if p[0] == "M" || p[0] == "A" {
        let (base, index, scale, disp) = if p[0] == "M" {
            (p[2], p[3], p[4], p[5])
        } else {
            (p[1], p[2], p[3], p[4])
        };
        let t = b.t();
        // the 0x67 prefix: a 32-bit address from 32-bit registers, R8D-R15D among them (SDM Vol 1 3.3.7; Sprint 100)
        let named = if base != "INVALID" { base } else { index };
        let abits = if named == "EIP" || reg(named).is_some_and(|r| r.1 == 32) {
            32
        } else {
            64
        };
        let seg = if p[0] == "M" && (p[1] == "FS" || p[1] == "GS") {
            Some(p[1])
        } else {
            None
        };
        if seg == Some("FS") {
            // Windows x64 user code reaches the TEB through GS (Sprint 044)
            return Err(LiftError(format!(
                "FS-relative memory operand {s}: the FS base is not modeled"
            )));
        }
        let disp: i64 = disp.parse().map_err(|_| bad())?;
        let scale: u32 = scale.parse().map_err(|_| bad())?;
        if base == "RIP" || base == "EIP" {
            b.emit(Op::Addr {
                t,
                base: None,
                index: None,
                scale: 1,
                disp: (next_ip as i64).wrapping_add(disp),
                abits,
                gs: seg == Some("GS"),
            });
        } else {
            let r = |n: &str| -> Result<Option<u8>, LiftError> {
                if n == "INVALID" {
                    Ok(None)
                } else {
                    Ok(Some(reg(n).ok_or_else(bad)?.0))
                }
            };
            b.emit(Op::Addr {
                t,
                base: r(base)?,
                index: r(index)?,
                scale,
                disp,
                abits,
                gs: seg == Some("GS"),
            });
        }
        if p[0] == "A" {
            return Ok((Operand::Addr(t), abits));
        }
        let bytes: u32 = p[6].parse().map_err(|_| bad())?;
        return Ok((Operand::Loc(Loc::M { addr: t, bytes }), bytes * 8));
    }
    if p[0] == "I" {
        let width = p[1].parse::<u32>().map_err(|_| bad())? * 8;
        let mut raw = u128::from_str_radix(p[2], 16).map_err(|_| bad())?;
        let t = b.t();
        let target = width_hint.unwrap_or(width);
        if width < target && raw >> (width - 1) & 1 == 1 {
            // imm sign-extends to operand size
            raw |= ((1u128 << target) - 1) ^ ((1u128 << width) - 1);
        }
        b.emit(Op::Imm { t, value: raw as u64 });
        return Ok((Operand::Imm(t), width));
    }
    if p[0] == "J" {
        let t = b.t();
        let rel: i64 = p[1].parse().map_err(|_| bad())?;
        b.emit(Op::Imm {
            t,
            value: (next_ip as i64).wrapping_add(rel) as u64,
        });
        return Ok((Operand::Imm(t), 64));
    }
    Err(bad())
}

/// `_val` (lift.py 217-223): a location loaded into a temporary; a temporary as it is.
pub fn val(b: &mut Builder, o: Operand) -> Temp {
    match o {
        Operand::Loc(loc) => {
            let t = b.t();
            b.emit(Op::Ld { t, loc });
            t
        }
        Operand::Addr(t) | Operand::Imm(t) => t,
    }
}

/// `_push` (lift.py 226-233): RSP -= size; store `size` bytes at the new RSP (size 2 for a 66-prefixed PUSH).
pub fn push(b: &mut Builder, value: Temp, size: u32) {
    let (sp, step, nsp) = (b.t(), b.t(), b.t());
    b.emit(Op::Ld {
        t: sp,
        loc: Loc::R {
            idx: RSP,
            bits: 64,
            hi: false,
        },
    });
    b.emit(Op::Imm {
        t: step,
        value: u64::from(size),
    });
    b.emit(Op::Alu {
        t: nsp,
        op: "sub",
        a: sp,
        b: step,
        bits: 64,
    });
    // the store, the only fault point, before RSP moves
    b.emit(Op::St {
        loc: Loc::M { addr: nsp, bytes: size },
        t: value,
    });
    b.emit(Op::St {
        loc: Loc::R {
            idx: RSP,
            bits: 64,
            hi: false,
        },
        t: nsp,
    });
}

/// `_pop` (lift.py 236-243).
pub fn pop(b: &mut Builder) -> Temp {
    let (sp, eight, nsp, v) = (b.t(), b.t(), b.t(), b.t());
    b.emit(Op::Ld {
        t: sp,
        loc: Loc::R {
            idx: RSP,
            bits: 64,
            hi: false,
        },
    });
    b.emit(Op::Ld {
        t: v,
        loc: Loc::M { addr: sp, bytes: 8 },
    });
    b.emit(Op::Imm { t: eight, value: 8 });
    b.emit(Op::Alu {
        t: nsp,
        op: "add",
        a: sp,
        b: eight,
        bits: 64,
    });
    b.emit(Op::St {
        loc: Loc::R {
            idx: RSP,
            bits: 64,
            hi: false,
        },
        t: nsp,
    });
    v
}

/// `commit_order` (lift.py 246-301): None when `ins` obeys the commit rule (specs/guest_state.json 0.2.0): no register,
/// flag or DF write before the instruction's last memory access, and at most one memory store, after every memory
/// load. Otherwise the first violation, named. The kinds the Rust rules do not emit yet are not in `Op`.
pub fn commit_order(ins: &Lifted) -> Option<String> {
    let memory_access = |op: &Op| match op {
        // ("sse", ic, dst, src, ...): a store to a memory destination, else a load from a memory source
        Op::Sse { dst, .. } if dst.addr().is_some() => Some("store"),
        Op::Sse { src, .. } if src.addr().is_some() => Some("load"),
        Op::Ld { loc: Loc::M { .. }, .. } | Op::Mxcsr { store: false, .. } => Some("load"),
        Op::St { loc: Loc::M { .. }, .. }
        | Op::Set { loc: Loc::M { .. }, .. }
        | Op::Cst { loc: Loc::M { .. }, .. }
        | Op::Cstz { loc: Loc::M { .. }, .. }
        | Op::Mxcsr { store: true, .. }
        | Op::Fcw { .. }
        | Op::Cx16 { .. }
        | Op::X86 {
            dst: Some(Loc::M { .. }),
            ..
        } => Some("store"),
        _ => None,
    };
    let effect = |op: &Op| {
        matches!(op, Op::Sse { dst, .. } if dst.addr().is_none())
            || matches!(
                op,
                Op::St { loc: Loc::R { .. }, .. }
                    | Op::Flags { .. }
                    | Op::Df { .. }
                    | Op::Cmov { .. }
                    | Op::Cpuid
                    | Op::Div { .. }
                    | Op::Set { loc: Loc::R { .. }, .. }
                    | Op::Cst { loc: Loc::R { .. }, .. }
                    | Op::Cstz { loc: Loc::R { .. }, .. }
                    | Op::Mxcsr { store: false, .. }
                    | Op::X86 { .. }
            )
    };
    let last_access = ins.ops.iter().rposition(|op| memory_access(op).is_some())?;
    let stores: Vec<usize> = (0..ins.ops.len())
        .filter(|&i| memory_access(&ins.ops[i]) == Some("store"))
        .collect();
    if stores.len() > 1 {
        return Some(format!(
            "{} at 0x{:x}: {} memory stores",
            ins.iclass,
            ins.address,
            stores.len()
        ));
    }
    if stores.first().is_some_and(|&s| s != last_access) {
        return Some(format!(
            "{} at 0x{:x}: a memory access after the store",
            ins.iclass, ins.address
        ));
    }
    for (i, op) in ins.ops[..last_access].iter().enumerate() {
        if effect(op) {
            let kind = match op {
                Op::Flags { .. } => "flags",
                Op::Df { .. } => "df",
                Op::Cpuid => "cpuid",
                Op::Mxcsr { .. } => "mxcsr",
                Op::X86 { .. } => "x86",
                Op::Sse { .. } => "sse",
                Op::Cmov { .. } => "cmov",
                Op::Div { .. } => "div",
                Op::Set { .. } => "set",
                Op::Cst { .. } => "cst",
                Op::Cstz { .. } => "cstz",
                _ => "st",
            };
            return Some(format!(
                "{} at 0x{:x}: {kind} at operation {i} before memory access {last_access}",
                ins.iclass, ins.address
            ));
        }
    }
    None
}

/// `rules.Ctx` (rules/__init__.py 27-34).
pub struct Ctx<'a> {
    pub b: &'a mut Builder,
    pub ic: String,
    pub f: &'a Fields,
    pub ops: Vec<String>,
    pub nxt: u64,
    pub address: u64,
}

/// lift.py 341-342: the classes whose rows may carry no operand template.
const NO_TEMPLATE_OK: [&str; 15] = [
    "NOP",
    "RET_NEAR",
    "CLD",
    "STD",
    "CPUID",
    "CDQ",
    "CDQE",
    "CQO",
    "CWDE",
    "SFENCE",
    "LFENCE",
    "MFENCE",
    "XGETBV",
    "VZEROUPPER",
    "VZEROALL",
];

/// lift.py 304: the classes with no SSE form.
const MMX_ONLY: [&str; 3] = ["EMMS", "MOVDQ2Q", "PSHUFW"];

/// `lift` (lift.py 327-355): the instruction at the start of `code`, located at guest `address`. Bytes the decode spec
/// has no row for are a LiftError too, so the run stops by name.
pub fn lift(code: &[u8], address: u64) -> Result<Lifted, LiftError> {
    let d = decode(code, None).map_err(|e: DecodeError| {
        let shown: String = code.iter().take(15).map(|b| format!("{b:02x}")).collect();
        LiftError(format!("{e} at 0x{address:x} ({shown})"))
    })?;
    let f = &d.fields;
    let ic = d.iclass.clone();
    let nxt = address + d.length as u64;
    let ops = match f.ops {
        Some(t) if !t.is_empty() => {
            x86_decoder::operands::operands(f).map_err(|e| LiftError(format!("{e:?} at 0x{address:x}")))?
        }
        _ => Vec::new(),
    };
    let declared_none = f.ops == Some("-"); // the spec says the row has no explicit operand (Sprint 065)
    let implicit_ok = NO_TEMPLATE_OK.contains(&ic.as_str())
        || crate::rules::REP_STRING.contains(&ic.as_str())
        || crate::rules::IMPLICIT_050.contains(&ic.as_str());
    if ops.is_empty() && !declared_none && !implicit_ok {
        // a row without an operand template (moffs MOV A0-A3, implicit forms): not lifted yet
        let shown: String = code[..d.length].iter().map(|b| format!("{b:02x}")).collect();
        return Err(LiftError(format!(
            "no operand template for {ic} at 0x{address:x} ({shown})"
        )));
    }
    if MMX_ONLY.contains(&ic.as_str()) || ops.iter().any(|o| o.starts_with("MM:")) {
        // lift_mmx (lift.py 307-324) comes with the MMX group
        return Err(LiftError(format!(
            "{ic} at 0x{address:x}: the MMX forms are a later group"
        )));
    }
    let Some(rule) = crate::rules::rule(&ic) else {
        return Err(LiftError(format!("no lifting rule for {ic} at 0x{address:x}")));
    };
    let mut b = Builder::default();
    rule(&mut Ctx {
        b: &mut b,
        ic: ic.clone(),
        f,
        ops,
        nxt,
        address,
    })?;
    Ok(Lifted {
        address,
        length: d.length as u64,
        iclass: ic,
        ops: b.ops,
    })
}
