// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! One lifted instruction to AArch64 words (Sprint 151), carried from `src/w2m/vm_backend.py` `compile_insn`
//! (1417-1885) and the helpers it calls: the flag setters (482-519, 559-710), the NZCV conditions and flag liveness
//! (815-920), the condition reads (973-1010), the temporaries (1013-1060), operand forms, address folding and fused
//! moves (1186-1348). The ops carried are those link 5's first group lifts; any other op, or a flags kind no rule of the
//! group emits, is a `CompileError` naming it. Each item cites the lines it carries.
//!
//! The constants the Python switched (FLAGS_FROM_NZCV, FOLD_ADDRESSES, OPERAND_FORMS, FUSE_LOAD_STORE, FLAG_LIVENESS)
//! were all on in the frozen host, and the C stencils were derived from that code: here they are always on.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use a64_encoder::assembler::{Assembler, Target};
use a64_encoder::{self as a64, LSL, LSR, ZR};

use crate::lift::{Lifted, Loc, Op, Temp};

/// vm_backend 357-371.
pub const TEMP_REGS: [u32; 6] = [18, 25, 26, 27, 29, 30];
pub const REG_CF: u32 = 19;
pub const REG_ZF: u32 = 20;
pub const REG_SF: u32 = 21;
pub const REG_OF: u32 = 22;
pub const REG_PF: u32 = 23;
pub const REG_UNDEFINED_FLAGS: u32 = 24;
pub const U_CF: u32 = 1;
pub const U_ZF: u32 = 2;
pub const U_SF: u32 = 4;
pub const U_OF: u32 = 8;
pub const U_PF: u32 = 16;
pub const U_AF: u32 = 32;
/// vm_backend 851.
pub const ALL_FLAGS: u32 = U_CF | U_ZF | U_SF | U_OF | U_PF | U_AF;
/// MONITOR_DATA + these, reached through x28 (vm_backend 89, 93, 94).
const AF_OPERANDS: u32 = 0x20;
const PF_RESULT: u32 = AF_OPERANDS + 16;
/// vm_backend 97: the state block's gs_base (the TEB).
const STATE_GS_BASE: u64 = 0xb0;
/// vm_backend 1124: the NZCV kinds by `vmc_flags.nzcv_from`'s index (vm_compiler.h 25): [none, NZCV_CONDITIONS...].
pub const NZCV_KINDS: [&str; 5] = ["sub", "add", "inc", "logic", "dec"];

/// vm_backend 392: an operation or form the compiler cannot lower.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError(pub String);

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CompileError {}

fn bit(f: char) -> u32 {
    match f {
        'C' => 0,
        'Z' => 1,
        'S' => 2,
        'O' => 3,
        'P' => 4,
        _ => 5,
    }
}

fn flag_reg(f: char) -> u32 {
    match f {
        'C' => REG_CF,
        'Z' => REG_ZF,
        'S' => REG_SF,
        'O' => REG_OF,
        _ => REG_PF,
    }
}

/// `_mask` (482-486).
fn mask(a: &mut Assembler, r: u32, bits: u32) {
    if bits == 32 {
        a.emit(&[a64::mov32(r, r)]);
    } else if bits < 64 {
        a.emit(&[a64::ubfx(r, r, 0, bits)]);
    }
}

/// `_nzcv` (497-509): NZCV as the add or subtract of `bits`-wide ra and rb sets it; an 8- or 16-bit operation runs in
/// the top of a 32-bit one. Uses x16 for the narrow forms.
fn nzcv(a: &mut Assembler, add: bool, ra: u32, rb: u32, bits: u32) {
    if bits == 64 {
        a.emit(&[if add {
            a64::adds(ZR, ra, rb)
        } else {
            a64::subs(ZR, ra, rb)
        }]);
    } else if bits == 32 {
        a.emit(&[if add {
            a64::adds32(ZR, ra, rb, 0)
        } else {
            a64::subs32(ZR, ra, rb, 0)
        }]);
    } else {
        let s = 32 - bits;
        a.emit(&[
            a64::lsl32(16, ra, s),
            if add {
                a64::adds32(ZR, 16, rb, s)
            } else {
                a64::subs32(ZR, 16, rb, s)
            },
        ]);
    }
}

/// `_adds_subs` (512-515).
fn adds_subs(add: bool, rd: u32, rn: u32, rm: u32, bits: u32) -> u32 {
    let w = if add {
        a64::adds(rd, rn, rm)
    } else {
        a64::subs(rd, rn, rm)
    };
    if bits == 64 { w } else { w & !(1 << 31) }
}

/// `_set_pf` (559-561): PF lazily, the result stored at PF_RESULT.
fn set_pf(a: &mut Assembler, r: u32) {
    a.emit(&[a64::str(8, r, 28, PF_RESULT)]);
}

/// `_pf_into_x16` (569-573).
fn pf_into_x16(a: &mut Assembler) {
    a.emit(&[
        a64::ldr(8, 16, 28, PF_RESULT),
        a64::ubfx(16, 16, 0, 8),
        a64::eor(16, 16, 16, LSR, 4),
        a64::eor(16, 16, 16, LSR, 2),
        a64::eor(16, 16, 16, LSR, 1),
        a64::ubfx(16, 16, 0, 1),
        a64::cmpi(16, 0),
        a64::cset(16, "eq"),
    ]);
}

/// `_set_flags` (583-709) for the kinds group 1 emits, with FLAGS_FROM_NZCV on: ADD, SUB (and INC, DEC, which share
/// the branch) through NZCV, and the logic ops. `live` holds the flags a later instruction reads; `nzcv_out` asks for
/// NZCV as well (the next instruction's Jcc branches on it).
#[allow(clippy::too_many_arguments)]
fn set_flags(
    a: &mut Assembler,
    kind: &str,
    ra: u32,
    rb: u32,
    rr: u32,
    bits: u32,
    nzcv_ready: bool,
    live: u32,
    nzcv_out: bool,
) -> Result<(), CompileError> {
    if matches!(kind, "add" | "inc" | "sub" | "dec") {
        // NZCV from one ADDS or SUBS gives CF ZF SF OF; INC and DEC keep CF (SDM INC, DEC)
        if !nzcv_ready && (live & (U_CF | U_ZF | U_SF | U_OF) != 0 || nzcv_out) {
            nzcv(a, matches!(kind, "add" | "inc"), ra, rb, bits);
        }
        for (flag, reg, cond) in [(U_ZF, REG_ZF, "eq"), (U_SF, REG_SF, "mi"), (U_OF, REG_OF, "vs")] {
            if live & flag != 0 {
                a.emit(&[a64::cset(reg, cond)]);
            }
        }
        if matches!(kind, "add" | "sub") && live & U_CF != 0 {
            a.emit(&[a64::cset(REG_CF, if kind == "add" { "cs" } else { "lo" })]);
        }
        if live != 0 {
            // MOV X24, XZR: every flag defined; AND X24, X24, #1: every flag defined but CF, which keeps its state
            a.emit(&[if matches!(kind, "add" | "sub") {
                a64::mov(REG_UNDEFINED_FLAGS, ZR)
            } else {
                a64::and_low(REG_UNDEFINED_FLAGS, REG_UNDEFINED_FLAGS, 1)
            }]);
        }
        if live & U_AF != 0 {
            a.emit(&[a64::stp(ra, rb, 28, i64::from(AF_OPERANDS))]); // AF's operands; AF reads the result too
        }
        if live & (U_AF | U_PF) != 0 {
            a.emit(&[a64::str(8, rr, 28, PF_RESULT)]); // PF's result
        }
        return Ok(());
    }
    if kind == "logic" {
        // SDM AND/OR/XOR/TEST: CF OF 0, AF undefined
        if live & U_CF != 0 {
            a.emit(&[a64::mov(REG_CF, ZR)]);
        }
        if live & U_OF != 0 {
            a.emit(&[a64::mov(REG_OF, ZR)]);
        }
        if live != 0 {
            a.emit(&[a64::movz(REG_UNDEFINED_FLAGS, u64::from(U_AF), 0)]);
        }
        if live & U_ZF != 0 || nzcv_out {
            a.emit(&[a64::cmpi(rr, 0)]);
        }
        if live & U_ZF != 0 {
            a.emit(&[a64::cset(REG_ZF, "eq")]);
        }
        if live & U_SF != 0 {
            a.emit(&[a64::ubfx(REG_SF, rr, bits - 1, 1)]);
        }
        if live & U_PF != 0 {
            set_pf(a, rr);
        }
        return Ok(());
    }
    Err(CompileError(format!("flags kind {kind}: a later group")))
}

/// `NZCV_CONDITIONS` (820-829): a conditional branch right after the flags' setter branches on NZCV itself.
pub fn nzcv_condition(kind: &str, cc: &str) -> Option<&'static str> {
    let sub: &[(&str, &str)] = &[
        ("z", "eq"),
        ("nz", "ne"),
        ("b", "lo"),
        ("nb", "hs"),
        ("be", "ls"),
        ("nbe", "hi"),
        ("l", "lt"),
        ("nl", "ge"),
        ("le", "le"),
        ("nle", "gt"),
        ("s", "mi"),
        ("ns", "pl"),
        ("o", "vs"),
        ("no", "vc"),
    ];
    let add: &[(&str, &str)] = &[
        ("z", "eq"),
        ("nz", "ne"),
        ("b", "cs"),
        ("nb", "cc"),
        ("l", "lt"),
        ("nl", "ge"),
        ("le", "le"),
        ("nle", "gt"),
        ("s", "mi"),
        ("ns", "pl"),
        ("o", "vs"),
        ("no", "vc"),
    ];
    let inc: &[(&str, &str)] = &[
        ("z", "eq"),
        ("nz", "ne"),
        ("l", "lt"),
        ("nl", "ge"),
        ("le", "le"),
        ("nle", "gt"),
        ("s", "mi"),
        ("ns", "pl"),
        ("o", "vs"),
        ("no", "vc"),
    ];
    let logic: &[(&str, &str)] = &[("z", "eq"), ("nz", "ne")];
    let table = match kind {
        "sub" => sub,
        "add" => add,
        "inc" | "dec" => inc,
        "logic" => logic,
        _ => return None,
    };
    table.iter().find(|(k, _)| *k == cc).map(|(_, v)| *v)
}

/// `INVERSE` (830-831): each Arm condition's inverse, named as `a64.COND` first names it, never `cs` or `cc`.
fn inverse(c: &str) -> &'static str {
    match c {
        "eq" => "ne",
        "ne" => "eq",
        "cs" | "hs" => "lo",
        "cc" | "lo" => "hs",
        "mi" => "pl",
        "pl" => "mi",
        "vs" => "vc",
        "vc" => "vs",
        "hi" => "ls",
        "ls" => "hi",
        "ge" => "lt",
        "lt" => "ge",
        "gt" => "le",
        _ => "gt",
    }
}

/// `nzcv_setter` (834-839): the flags kind whose NZCV a conditional branch right after `ins` may branch on.
pub fn nzcv_setter(ins: &Lifted) -> Option<&'static str> {
    match ins.ops.last() {
        Some(Op::Flags { kind, .. }) if matches!(*kind, "sub" | "add" | "inc" | "logic" | "dec") => Some(kind),
        _ => None,
    }
}

/// `COND_FLAGS` (852-853).
fn cond_flags(cc: &str) -> u32 {
    match cc.strip_prefix('n').unwrap_or(cc) {
        "o" => U_OF,
        "b" => U_CF,
        "z" => U_ZF,
        "s" => U_SF,
        "p" => U_PF,
        "be" => U_CF | U_ZF,
        "l" => U_SF | U_OF,
        _ => U_ZF | U_SF | U_OF, // "le"
    }
}

/// `FLAGS_WRITTEN` (854-855).
fn flags_written(kind: &str) -> u32 {
    match kind {
        "add" | "sub" | "adc" | "sbb" | "logic" | "andn" | "mul" | "imul" | "bsf" | "popcnt" => ALL_FLAGS,
        "inc" | "dec" => ALL_FLAGS & !U_CF,
        "bt" => ALL_FLAGS & !U_ZF,
        _ => 0,
    }
}

/// `_op_flags` (864-889): (flags read, flags written) by one lifted op.
fn op_flags(op: &Op, constant: &BTreeMap<Temp, u64>) -> (u32, u32) {
    match op {
        Op::Flags {
            kind, b: count, bits, ..
        } => {
            if matches!(*kind, "shr" | "shl" | "sar" | "rol" | "ror") {
                let m = if *bits == 64 { 63 } else { 31 };
                match constant.get(count) {
                    Some(c) if c & m != 0 => (
                        0,
                        if matches!(*kind, "rol" | "ror") {
                            U_CF | U_OF
                        } else {
                            ALL_FLAGS
                        },
                    ),
                    _ => (0, 0),
                }
            } else {
                (0, flags_written(kind))
            }
        }
        Op::Alu { op, .. } => (if matches!(*op, "adc" | "sbb") { U_CF } else { 0 }, 0),
        Op::Br { cond, .. } => (if *cond == "always" { 0 } else { cond_flags(cond) }, 0),
        Op::Imm { .. } | Op::Addr { .. } | Op::Ld { .. } | Op::St { .. } => (0, 0),
    }
}

/// `_op_flag_masks` (892-904): `_op_flags` of each op; a Jcc that branches on the previous instruction's NZCV reads
/// no flag register.
fn op_flag_masks(ins: &Lifted, nzcv_kind: Option<&str>) -> Vec<(u32, u32)> {
    let mut constant = BTreeMap::new();
    let mut out = Vec::new();
    for op in &ins.ops {
        if let Op::Imm { t, value } = op {
            constant.insert(*t, *value);
        }
        let (mut r, w) = op_flags(op, &constant);
        if let Op::Br { cond, .. } = op
            && nzcv_kind.is_some_and(|k| nzcv_condition(k, cond).is_some())
        {
            r = 0;
        }
        out.push((r, w));
    }
    out
}

/// `insn_flags` (907-913): (flags `ins` reads before writing them, flags it writes).
pub fn insn_flags(ins: &Lifted, nzcv_kind: Option<&str>) -> (u32, u32) {
    let (mut read, mut written) = (0, 0);
    for (r, w) in op_flag_masks(ins, nzcv_kind) {
        read |= r & !written;
        written |= w;
    }
    (read, written)
}

/// `nzcv_branch` (916-919): `nxt` is a Jcc that branches on `setter`'s NZCV.
pub fn nzcv_branch(setter: &Lifted, nxt: &Lifted) -> bool {
    let Some(kind) = nzcv_setter(setter) else {
        return false;
    };
    nxt.ops
        .iter()
        .any(|op| matches!(op, Op::Br { cond, .. } if nzcv_condition(kind, cond).is_some()))
}

/// `_exit_if_undefined` (973-975): exit with HVC 0x40|bit when flag `f` is undefined.
fn exit_if_undefined(a: &mut Assembler, f: char) {
    a.emit(&[a64::tbz(REG_UNDEFINED_FLAGS, bit(f), 8), a64::hvc(0x40 | bit(f))]);
}

/// `_eval_condition` (978-1010): x16 = condition `cc`, reading flags in `runtime.Runtime._cond`'s order.
fn eval_condition(a: &mut Assembler, cc: &str) -> Result<(), CompileError> {
    let neg = cc.starts_with('n');
    let key = if neg { &cc[1..] } else { cc };
    let single = |k: &str| match k {
        "o" => Some('O'),
        "b" => Some('C'),
        "z" => Some('Z'),
        "s" => Some('S'),
        "p" => Some('P'),
        _ => None,
    };
    if key == "p" {
        exit_if_undefined(a, 'P');
        pf_into_x16(a);
    } else if let Some(f) = single(key) {
        exit_if_undefined(a, f);
        a.emit(&[a64::mov(16, flag_reg(f))]);
    } else if key == "be" || key == "le" {
        let end = a.label();
        let first = if key == "be" { 'C' } else { 'Z' };
        exit_if_undefined(a, first);
        a.emit(&[a64::mov(16, flag_reg(first))]);
        a.branch(|off| a64::cbnz(16, off), end);
        if key == "be" {
            exit_if_undefined(a, 'Z');
            a.emit(&[a64::mov(16, REG_ZF)]);
        } else {
            exit_if_undefined(a, 'S');
            exit_if_undefined(a, 'O');
            a.emit(&[a64::eor(16, REG_SF, REG_OF, LSL, 0)]);
        }
        a.bind(end);
    } else if key == "l" {
        exit_if_undefined(a, 'S');
        exit_if_undefined(a, 'O');
        a.emit(&[a64::eor(16, REG_SF, REG_OF, LSL, 0)]);
    } else {
        return Err(CompileError(format!("condition {cc}")));
    }
    if neg {
        a.emit(&[a64::cmpi(16, 0), a64::cset(16, "eq")]);
    }
    Ok(())
}

/// `_temps_read` (1013-1055) for the ops carried.
fn temps_read(op: &Op) -> Vec<Temp> {
    match op {
        Op::Ld {
            loc: Loc::M { addr, .. },
            ..
        } => vec![*addr],
        Op::Ld { .. } => vec![],
        Op::St { loc, t } => match loc {
            Loc::M { addr, .. } => vec![*addr, *t],
            Loc::R { .. } => vec![*t],
        },
        Op::Alu { a, b, .. } => vec![*a, *b],
        Op::Flags { a, b, r, .. } => vec![*a, *b, *r],
        Op::Br { t, .. } => vec![*t],
        Op::Imm { .. } | Op::Addr { .. } => vec![],
    }
}

/// `_temps_written` (1058-1059).
fn temps_written(op: &Op) -> Vec<Temp> {
    match op {
        Op::Imm { t, .. } | Op::Addr { t, .. } | Op::Ld { t, .. } | Op::Alu { t, .. } => vec![*t],
        _ => vec![],
    }
}

/// An address's Arm form (vm_backend 1192): LDR/STR's scaled offset, LDUR/STUR's -256..255, or a register offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AddressForm {
    Imm { base: u32, disp: i64 },
    Unscaled { base: u32, disp: i64 },
    Reg { base: u32, index: u32, scaled: bool },
}

/// `_reads_low32` (1206-1216): `o` uses only the low 32 bits of temporary `t`.
fn reads_low32(o: &Op, t: Temp) -> bool {
    match o {
        Op::Alu { op, bits, .. } => *bits == 32 && matches!(*op, "add" | "sub" | "and" | "or" | "xor"),
        Op::Flags { kind, bits, .. } => *bits == 32 && matches!(*kind, "add" | "sub" | "inc" | "dec" | "logic"),
        Op::St { loc, t: v } if *v == t => match loc {
            Loc::M { addr, bytes } => *bytes == 4 && *addr != t,
            Loc::R { bits, hi, .. } => *bits == 32 && !hi,
        },
        _ => false,
    }
}

/// What an op writes as a location (`_operand_forms`' `writes`).
fn writes(o: &Op) -> Option<Loc> {
    match o {
        Op::St { loc, .. } => Some(*loc),
        _ => None,
    }
}

/// `FORWARD_AFTER` (1202) over the ops carried.
fn forward_after(o: &Op) -> bool {
    matches!(
        o,
        Op::Alu { .. } | Op::Flags { .. } | Op::Imm { .. } | Op::Br { .. } | Op::St { .. }
    )
}

type Forms = (BTreeMap<Temp, u32>, BTreeMap<Temp, u64>, BTreeMap<Temp, u32>);

/// `_operand_forms` (1219-1268): (temporaries read as the guest register they load, constants kept out of registers,
/// results computed straight into the guest register they are stored to). The ops at `dead` emit nothing and read
/// nothing.
fn operand_forms(ops: &[Op], last: &BTreeMap<Temp, usize>, dead: &BTreeSet<usize>) -> Forms {
    let (mut alias, mut quiet, mut forward) = (BTreeMap::new(), BTreeMap::new(), BTreeMap::new());
    let mut readers: BTreeMap<Temp, Vec<usize>> = BTreeMap::new();
    for (i, op) in ops.iter().enumerate() {
        if dead.contains(&i) {
            continue;
        }
        for t in temps_read(op) {
            readers.entry(t).or_default().push(i);
        }
    }
    for (i, op) in ops.iter().enumerate() {
        match op {
            Op::Ld {
                t,
                loc: Loc::R { idx, bits, hi: false },
            } if matches!(*bits, 32 | 64) && last.contains_key(t) => {
                let rs = readers.get(t).cloned().unwrap_or_default();
                if *bits == 32 && !rs.iter().all(|&j| reads_low32(&ops[j], *t)) {
                    continue;
                }
                let written = ops[i + 1..last[t]]
                    .iter()
                    .any(|o| matches!(writes(o), Some(Loc::R { idx: w, .. }) if w == *idx));
                if !written {
                    alias.insert(*t, u32::from(*idx));
                }
            }
            Op::Imm { t, value } if readers.contains_key(t) => {
                let rs = &readers[t];
                let all_add_sub = rs.iter().all(|&j| {
                    matches!(&ops[j], Op::Alu { op, a, b, .. } if matches!(*op, "add" | "sub") && *b == *t && *a != *t)
                });
                // an ADD or SUB immediate (0-4095), or a constant only a branch reads (into x17 directly)
                let branch_only = rs.len() == 1 && matches!(&ops[rs[0]], Op::Br { t: bt, .. } if *bt == *t);
                if (*value < 4096 && all_add_sub) || branch_only {
                    quiet.insert(*t, *value);
                }
            }
            _ => {}
        }
    }
    for (i, op) in ops.iter().enumerate() {
        let (t, width) = match op {
            Op::Alu { t, bits, .. } => (*t, *bits),
            Op::Ld {
                t,
                loc: Loc::M { bytes, .. },
            } if !alias.contains_key(t) => (*t, 8 * bytes),
            _ => continue,
        };
        let stores: Vec<usize> = readers
            .get(&t)
            .map(|rs| {
                rs.iter()
                    .copied()
                    .filter(|&j| !matches!(ops[j], Op::Flags { .. }))
                    .collect()
            })
            .unwrap_or_default();
        if stores.len() != 1 || !matches!(&ops[stores[0]], Op::St { t: v, .. } if *v == t) {
            continue;
        }
        let Op::St {
            loc: Loc::R { idx, bits, hi: false },
            ..
        } = ops[stores[0]]
        else {
            continue;
        };
        if bits != width || !matches!(width, 32 | 64) {
            continue;
        }
        let idx = u32::from(idx);
        let ok = ops
            .iter()
            .enumerate()
            .filter(|(j, _)| *j > i && !dead.contains(j))
            .all(|(j, o)| {
                forward_after(o)
                    && match writes(o) {
                        None => true,
                        Some(Loc::R { idx: w, .. }) => u32::from(w) != idx || j == stores[0],
                        Some(Loc::M { .. }) => false,
                    }
                    && !temps_read(o).iter().any(|r| alias.get(r) == Some(&idx))
            });
        if ok {
            forward.insert(t, idx);
        }
    }
    (alias, quiet, forward)
}

/// `_folded_addresses` (1271-1302): the `addr` temporaries whose every use is a plain load or store, each with the Arm
/// form that reaches it. 64-bit addresses with no segment only.
fn folded_addresses(ops: &[Op]) -> BTreeMap<Temp, AddressForm> {
    let mut users: BTreeMap<Temp, Vec<u32>> = BTreeMap::new();
    let mut other: BTreeSet<Temp> = BTreeSet::new();
    for op in ops {
        match op {
            Op::Ld {
                loc: Loc::M { addr, bytes },
                ..
            } => users.entry(*addr).or_default().push(*bytes),
            Op::St {
                loc: Loc::M { addr, bytes },
                ..
            } => {
                users.entry(*addr).or_default().push(*bytes);
                other.extend(temps_read(op).into_iter().filter(|t| t != addr));
            }
            _ => other.extend(temps_read(op)),
        }
    }
    let mut folded = BTreeMap::new();
    for op in ops {
        let Op::Addr {
            t,
            base,
            index,
            scale,
            disp,
            abits,
            gs,
        } = op
        else {
            continue;
        };
        let (Some(base), false, 64, Some(sizes)) = (base, *gs, *abits, users.get(t)) else {
            continue;
        };
        if other.contains(t) {
            continue;
        }
        let sizes: BTreeSet<u32> = sizes.iter().copied().collect();
        let base = u32::from(*base);
        match index {
            None => {
                let scaled = sizes.iter().all(|&s| {
                    let s = i64::from(s);
                    disp.rem_euclid(s) == 0 && (0..4096).contains(&disp.div_euclid(s))
                });
                if scaled {
                    folded.insert(*t, AddressForm::Imm { base, disp: *disp });
                } else if (-256..=255).contains(disp) {
                    folded.insert(*t, AddressForm::Unscaled { base, disp: *disp });
                }
            }
            Some(index) => {
                if *disp == 0 && (*scale == 1 || sizes == BTreeSet::from([*scale])) {
                    folded.insert(
                        *t,
                        AddressForm::Reg {
                            base,
                            index: u32::from(*index),
                            scaled: *scale != 1,
                        },
                    );
                }
            }
        }
    }
    folded
}

/// `_mem` (1305-1311): the load or store of `size` bytes between rt and a folded address.
fn mem(load: bool, size: u32, rt: u32, form: AddressForm) -> u32 {
    match form {
        AddressForm::Imm { base, disp } => {
            if load {
                a64::ldr(size, rt, base, disp as u32)
            } else {
                a64::str(size, rt, base, disp as u32)
            }
        }
        AddressForm::Unscaled { base, disp } => {
            if load {
                a64::ldur(size, rt, base, disp)
            } else {
                a64::stur(size, rt, base, disp)
            }
        }
        AddressForm::Reg { base, index, scaled } => {
            if load {
                a64::ldr_reg(size, rt, base, index, scaled)
            } else {
                a64::str_reg(size, rt, base, index, scaled)
            }
        }
    }
}

/// `_fused_move` (1314-1346): a load whose temporary only the next op stores, as one instruction straight between the
/// guest's register and memory or register. False when the pair is none of the forms.
fn fused_move(
    a: &mut Assembler,
    ops: &[Op],
    i: usize,
    last: &BTreeMap<Temp, usize>,
    reg: &BTreeMap<Temp, u32>,
    folded: &BTreeMap<Temp, AddressForm>,
) -> bool {
    let Op::Ld { t, loc: src } = ops[i] else {
        return false;
    };
    let Some(Op::St { loc: dst, t: v }) = ops.get(i + 1) else {
        return false;
    };
    if *v != t || last.get(&t) != Some(&(i + 1)) {
        return false;
    }
    match (src, *dst) {
        (
            Loc::R {
                idx: s,
                bits: sbits,
                hi: shi,
            },
            Loc::R {
                idx: d,
                bits: dbits,
                hi: dhi,
            },
        ) => {
            if shi || dhi || sbits != dbits || !matches!(sbits, 32 | 64) {
                return false;
            }
            let (d, s) = (u32::from(d), u32::from(s));
            a.emit(&[if sbits == 64 { a64::mov(d, s) } else { a64::mov32(d, s) }]);
            true
        }
        (
            Loc::M { addr, bytes },
            Loc::R {
                idx: d,
                bits: dbits,
                hi: dhi,
            },
        ) => {
            if dhi || !matches!(dbits, 32 | 64) || bytes * 8 > dbits {
                return false;
            }
            let d = u32::from(d);
            a.emit(&[match folded.get(&addr) {
                Some(f) => mem(true, bytes, d, *f),
                None => a64::ldr(bytes, d, reg[&addr], 0),
            }]);
            true
        }
        (
            Loc::R {
                idx: s,
                bits: sbits,
                hi: shi,
            },
            Loc::M { addr, bytes },
        ) => {
            if shi || sbits != bytes * 8 {
                return false;
            }
            let s = u32::from(s);
            a.emit(&[match folded.get(&addr) {
                Some(f) => mem(false, bytes, s, *f),
                None => a64::str(bytes, s, reg[&addr], 0),
            }]);
            true
        }
        _ => false,
    }
}

/// `compile_insn` (1417-1885): append `ins` to `a`. A taken branch puts the x86 target in x17 and jumps to `exit_va`.
/// `nzcv_from`: the flags kind of the previous instruction's NZCV; `live_after`: the flags a later instruction reads;
/// `nzcv_out`: the next instruction's Jcc branches on this one's NZCV. Returns true when control never falls through.
/// `state_va` is the process area's state block (PROCESS_VA + STATE_OFF), for a GS-relative address.
pub fn compile_insn(
    a: &mut Assembler,
    ins: &Lifted,
    exit_va: u64,
    state_va: u64,
    nzcv_from: Option<&str>,
    live_after: u32,
    nzcv_out: bool,
) -> Result<bool, CompileError> {
    let ops = &ins.ops;
    // flag liveness (1425-1434): each flags op with the flags read after it
    let mut flags_live: BTreeMap<usize, u32> = BTreeMap::new();
    let mut live = live_after;
    if ops.iter().any(|op| matches!(op, Op::Flags { .. })) {
        let masks = op_flag_masks(ins, nzcv_from);
        for i in (0..ops.len()).rev() {
            if matches!(ops[i], Op::Flags { .. }) {
                flags_live.insert(i, live);
            }
            live = live & !masks[i].1 | masks[i].0;
        }
    }
    let last_flags = flags_live.keys().next_back().copied();
    let dead: BTreeSet<usize> = flags_live
        .iter()
        .filter(|(i, v)| **v == 0 && !(nzcv_out && Some(**i) == last_flags))
        .map(|(i, _)| *i)
        .collect();
    let mut last: BTreeMap<Temp, usize> = BTreeMap::new();
    for (i, op) in ops.iter().enumerate() {
        if !dead.contains(&i) {
            for t in temps_read(op) {
                last.insert(t, i);
            }
        }
    }
    let mut free: Vec<u32> = TEMP_REGS.to_vec();
    let mut reg: BTreeMap<Temp, u32> = BTreeMap::new();
    let mut ends = false;
    let mut nzcv_ready = false; // an ALU op fused with the flags op that follows
    let mut fused_store = false; // a load fused with the store that follows
    let mut constant: BTreeMap<Temp, u64> = BTreeMap::new(); // temporaries an `imm` of this instruction set
    let folded = folded_addresses(ops);
    let (alias, quiet, forward) = operand_forms(ops, &last, &dead);
    let mut pinned = alias.clone(); // temporaries that are guest registers, never freed
    pinned.extend(forward.iter().map(|(k, v)| (*k, *v)));
    let at = |t: Temp, reg: &BTreeMap<Temp, u32>| -> Result<u32, CompileError> {
        reg.get(&t)
            .copied()
            .ok_or_else(|| CompileError(format!("temporary {t} has no register at 0x{:x}", ins.address)))
    };

    for (i, op) in ops.iter().enumerate() {
        for t in temps_written(op) {
            if let Some(r) = pinned.get(&t) {
                reg.insert(t, *r);
                continue;
            }
            if quiet.contains_key(&t) {
                continue;
            }
            if free.is_empty() {
                return Err(CompileError(format!(
                    "more than {} live temporaries at 0x{:x}",
                    TEMP_REGS.len(),
                    ins.address
                )));
            }
            reg.insert(t, free.remove(0));
        }
        if !matches!(op, Op::Imm { .. }) {
            for t in temps_written(op) {
                constant.remove(&t);
            }
        }
        match op {
            Op::Imm { t, value } => {
                if !quiet.contains_key(t) {
                    a.emit(&a64::mov_imm(at(*t, &reg)?, *value));
                }
                constant.insert(*t, *value);
            }
            Op::Addr { t, .. } if folded.contains_key(t) => {} // the loads and stores reach it in their own form
            Op::Addr {
                t,
                base,
                index,
                scale,
                disp,
                abits,
                gs,
            } => {
                let rd = at(*t, &reg)?;
                let log = scale.trailing_zeros();
                match base {
                    Some(base) if -4096 < *disp && *disp < 4096 => {
                        // base (+ index) and a small displacement as ADD/SUB immediates, not a MOV of it
                        let base = u32::from(*base);
                        match index {
                            Some(index) => a.emit(&[a64::add(rd, base, u32::from(*index), LSL, log)]),
                            None => a.emit(&[a64::mov(rd, base)]),
                        }
                        if *disp > 0 {
                            a.emit(&[a64::addi(rd, rd, *disp as u32)]);
                        } else if *disp < 0 {
                            a.emit(&[a64::subi(rd, rd, (-*disp) as u32)]);
                        }
                    }
                    _ => {
                        a.emit(&a64::mov_imm(rd, *disp as u64));
                        if let Some(base) = base {
                            a.emit(&[a64::add(rd, rd, u32::from(*base), LSL, 0)]);
                        }
                        if let Some(index) = index {
                            a.emit(&[a64::add(rd, rd, u32::from(*index), LSL, log)]);
                        }
                    }
                }
                mask(a, rd, *abits);
                if *gs {
                    // + the thread's GS base: state block gs_base (the TEB)
                    a.emit(&a64::mov_imm(16, state_va + STATE_GS_BASE));
                    a.emit(&[a64::ldr(8, 16, 16, 0), a64::add(rd, rd, 16, LSL, 0)]);
                }
            }
            Op::Ld { .. } if fused_move(a, ops, i, &last, &reg, &folded) => {
                fused_store = true; // the store that follows is done
            }
            Op::Ld { t, .. } if alias.contains_key(t) => {} // read as the guest register itself
            Op::Ld { t, loc } => {
                let rt = at(*t, &reg)?;
                match loc {
                    Loc::R { idx, bits, hi } => {
                        let idx = u32::from(*idx);
                        a.emit(&[if *hi {
                            a64::ubfx(rt, idx, 8, 8)
                        } else if *bits == 64 {
                            a64::mov(rt, idx)
                        } else if *bits == 32 {
                            a64::mov32(rt, idx)
                        } else {
                            a64::ubfx(rt, idx, 0, *bits)
                        }]);
                    }
                    Loc::M { addr, bytes } => match folded.get(addr) {
                        Some(f) => a.emit(&[mem(true, *bytes, rt, *f)]),
                        None => a.emit(&[a64::ldr(*bytes, rt, at(*addr, &reg)?, 0)]),
                    },
                }
            }
            Op::St { .. } if fused_store => fused_store = false,
            Op::St {
                loc: Loc::R { idx, .. },
                t,
            } if forward.get(t) == Some(&u32::from(*idx)) => {} // computed in place
            Op::St { loc, t } => {
                let rt = at(*t, &reg)?;
                store(a, loc, rt, &folded, &reg)?;
            }
            Op::Alu {
                t,
                op: o,
                a: x,
                b: y,
                bits,
            } => {
                let (rd, rx) = (at(*t, &reg)?, at(*x, &reg)?);
                let ry = reg.get(y).copied();
                // the flags op this ALU op feeds: the next op past any store of the result
                let feeds = (i + 1..ops.len()).find(|&j| !matches!(ops[j], Op::St { .. }));
                let nxt = feeds.filter(|j| !dead.contains(j)).map(|j| &ops[j]);
                if matches!(*o, "add" | "sub")
                    && matches!(*bits, 32 | 64)
                    && let Some(Op::Flags {
                        kind,
                        a: fa,
                        b: fb,
                        r: fr,
                        bits: fbits,
                    }) = nxt
                    && (if *o == "add" {
                        matches!(*kind, "add" | "inc")
                    } else {
                        matches!(*kind, "sub" | "dec")
                    })
                    && (*fa, *fb, *fr, *fbits) == (*x, *y, *t, *bits)
                {
                    // the result and NZCV from one ADDS or SUBS; the 32-bit form zero-extends as x86 does, so no mask
                    // follows, and the flags op that comes next skips its own
                    a.emit(&[adds_subs(*o == "add", rd, rx, ry.unwrap_or(u32::MAX), *bits)]);
                    nzcv_ready = true;
                    continue; // as the Python's `continue`: no temporary is released after this op
                }
                let mut bits = *bits;
                if let Some(v) = quiet.get(y) {
                    // ADD or SUB with its immediate
                    let v = *v as u32;
                    a.emit(&[match (*o == "add", bits == 32) {
                        (true, true) => a64::addi32(rd, rx, v),
                        (false, true) => a64::subi32(rd, rx, v),
                        (true, false) => a64::addi(rd, rx, v),
                        (false, false) => a64::subi(rd, rx, v),
                    }]);
                    if bits == 32 {
                        bits = 64; // the 32-bit form zero-extends: no mask
                    }
                } else if matches!(*o, "add" | "sub" | "and" | "or" | "xor") {
                    let ry = ry.ok_or_else(|| CompileError(format!("alu {o}: no register for its operand")))?;
                    let w = match *o {
                        "add" => a64::add(rd, rx, ry, LSL, 0),
                        "sub" => a64::sub(rd, rx, ry),
                        "and" => a64::and(rd, rx, ry),
                        "or" => a64::orr(rd, rx, ry),
                        _ => a64::eor(rd, rx, ry, LSL, 0),
                    };
                    if bits == 32 {
                        a.emit(&[w & !(1 << 31)]); // the W form zero-extends as x86 does: no mask
                        bits = 64;
                    } else {
                        a.emit(&[w]);
                    }
                } else if *o == "sext32" {
                    a.emit(&[a64::sxtw(rd, rx)]);
                } else {
                    return Err(CompileError(format!("alu {o}: a later group")));
                }
                mask(a, rd, bits);
            }
            Op::Flags {
                kind,
                a: x,
                b: y,
                r,
                bits,
            } => {
                let (live, nzcv) = (flags_live[&i], nzcv_out && Some(i) == last_flags);
                if live == 0 && !nzcv {
                    // no later instruction reads them (flag liveness)
                } else if matches!(*kind, "shr" | "shl" | "sar" | "rol" | "ror") {
                    return Err(CompileError(format!("flags kind {kind}: a later group")));
                } else {
                    let (rx, ry, rr) = (at(*x, &reg)?, at(*y, &reg)?, at(*r, &reg)?);
                    set_flags(a, kind, rx, ry, rr, *bits, nzcv_ready, live, nzcv)?;
                }
                nzcv_ready = false;
            }
            Op::Br { cond, t } => {
                let target: Vec<u32> = match quiet.get(t) {
                    Some(v) => a64::mov_imm(17, *v),
                    None => vec![a64::mov(17, at(*t, &reg)?)],
                };
                if *cond == "always" {
                    a.emit(&target);
                    a.branch(a64::b, Target::Va(exit_va));
                    ends = true;
                } else {
                    let skip = a.label();
                    match nzcv_from.and_then(|k| nzcv_condition(k, cond)) {
                        Some(arm) => {
                            // the previous instruction's NZCV
                            let inv = inverse(arm);
                            a.branch(move |off| a64::bcond(inv, off), skip);
                        }
                        None => {
                            eval_condition(a, cond)?;
                            a.branch(|off| a64::cbz(16, off), skip);
                        }
                    }
                    a.emit(&target);
                    a.branch(a64::b, Target::Va(exit_va));
                    a.bind(skip);
                }
            }
        }
        for t in temps_read(op).into_iter().chain(temps_written(op)) {
            if reg.contains_key(&t) && last.get(&t).is_none_or(|&l| l <= i) {
                let r = reg.remove(&t).unwrap();
                if !pinned.contains_key(&t) {
                    free.push(r);
                }
            }
        }
    }
    Ok(ends)
}

/// compile_insn's `store` (1446-1460).
fn store(
    a: &mut Assembler,
    loc: &Loc,
    rt: u32,
    folded: &BTreeMap<Temp, AddressForm>,
    reg: &BTreeMap<Temp, u32>,
) -> Result<(), CompileError> {
    match loc {
        Loc::R { idx, bits, hi } => {
            let idx = u32::from(*idx);
            a.emit(&[if *hi {
                a64::bfi(idx, rt, 8, 8)
            } else if *bits == 64 {
                a64::mov(idx, rt)
            } else if *bits == 32 {
                a64::mov32(idx, rt)
            } else {
                a64::bfi(idx, rt, 0, *bits)
            }]);
        }
        Loc::M { addr, bytes } => match folded.get(addr) {
            Some(f) => a.emit(&[mem(false, *bytes, rt, *f)]),
            None => {
                let ra = reg
                    .get(addr)
                    .ok_or_else(|| CompileError(format!("address temporary {addr} has no register")))?;
                a.emit(&[a64::str(*bytes, rt, *ra, 0)]);
            }
        },
    }
    Ok(())
}

/// `tests/test_operand_forms.py`'s four cases (the guards no class run can reach), carried by hand.
#[cfg(test)]
mod tests {
    use super::*;

    const RAX: u8 = 0;
    const RBX: u8 = 3;

    fn r(idx: u8, bits: u32) -> Loc {
        Loc::R { idx, bits, hi: false }
    }

    fn last_reads(ops: &[Op]) -> BTreeMap<Temp, usize> {
        let mut last = BTreeMap::new();
        for (i, op) in ops.iter().enumerate() {
            for t in temps_read(op) {
                last.insert(t, i);
            }
        }
        last
    }

    fn add_then_store(dst_bits: u32, a_bits: u32, b_bits: u32, alu_bits: u32, dst: u8, flags: bool) -> Vec<Op> {
        let mut ops = vec![
            Op::Ld {
                t: 1,
                loc: r(RAX, a_bits),
            },
            Op::Ld {
                t: 2,
                loc: r(RBX, b_bits),
            },
            Op::Alu {
                t: 3,
                op: "add",
                a: 1,
                b: 2,
                bits: alu_bits,
            },
        ];
        if flags {
            ops.push(Op::Flags {
                kind: "add",
                a: 1,
                b: 2,
                r: 3,
                bits: alu_bits,
            });
        }
        ops.push(Op::St {
            loc: r(dst, dst_bits),
            t: 3,
        });
        ops
    }

    #[test]
    fn a_result_is_not_forwarded_past_a_reader_of_the_old_value() {
        let ops = add_then_store(64, 64, 64, 64, RAX, true);
        let (alias, _, forward) = operand_forms(&ops, &last_reads(&ops), &BTreeSet::new());
        assert_eq!(alias, BTreeMap::from([(1, 0), (2, 3)]));
        assert!(forward.is_empty()); // the flags op reads RAX's old value after the add
    }

    #[test]
    fn a_result_with_dead_flags_is_computed_in_place() {
        let ops = add_then_store(64, 64, 64, 64, RAX, true);
        let last = BTreeMap::from([(1, 2), (2, 2), (3, 4)]); // the dead flags op reads nothing
        let (alias, _, forward) = operand_forms(&ops, &last, &BTreeSet::from([3]));
        assert_eq!(alias, BTreeMap::from([(1, 0), (2, 3)]));
        assert_eq!(forward, BTreeMap::from([(3, 0)]));
    }

    #[test]
    fn a_32_bit_register_with_a_64_bit_reader_is_copied() {
        let ops = add_then_store(64, 32, 64, 64, RBX, false);
        let (alias, _, _) = operand_forms(&ops, &last_reads(&ops), &BTreeSet::new());
        assert!(!alias.contains_key(&1)); // the add would read RAX's upper half
    }

    #[test]
    fn a_32_bit_register_read_by_32_bit_ops_is_read_in_place() {
        let ops = add_then_store(32, 32, 32, 32, RAX, false);
        let (alias, _, forward) = operand_forms(&ops, &last_reads(&ops), &BTreeSet::new());
        assert_eq!(alias, BTreeMap::from([(1, 0), (2, 3)]));
        assert_eq!(forward, BTreeMap::from([(3, 0)]));
    }
}
