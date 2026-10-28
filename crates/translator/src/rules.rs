// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The lifting rules, one per x86 instruction class (Sprint 151), carried from `src/w2m/rules` (one module per class,
//! Sprint 041) and its shared `_int.py`. Link 5 ports them in groups (`docs/design-host-compile-path-2026-10-24.md`);
//! group 1 is the integer and control-flow core: `alu` (ADD SUB AND OR XOR CMP TEST), `move` (MOV MOVZX MOVSXD LEA),
//! `jcc`, `jmp_call` (JMP CALL_NEAR), `ret_near`, `push`, `pop` and `nop` (NOP and the PREFETCH classes). Group 2
//! (Sprint 153) is the rest of `_int.py`'s rules lowered in A64 and `bmi.py`'s: 70 classes more. Each rule cites the
//! `_int.py` lines it carries.

use crate::lift::{
    Ctx, LiftError, Loc, Op, Operand, RAX, RDX, RSP, alu_op, cc, operand, pop as pop_value, push as push_value, val,
};

/// A rule: the operations of one instruction appended to `ctx.b`.
pub type Rule = fn(&mut Ctx<'_>) -> Result<(), LiftError>;

/// lift.py 94: REP STOS and REP MOVS, by iclass.
pub const REP_STRING: [&str; 8] = [
    "REP_STOSB",
    "REP_STOSW",
    "REP_STOSD",
    "REP_STOSQ",
    "REP_MOVSB",
    "REP_MOVSW",
    "REP_MOVSD",
    "REP_MOVSQ",
];

/// lift.py 143-144: integer classes whose operands are all implicit (Sprint 050).
pub const IMPLICIT_050: [&str; 16] = [
    "STC", "CLC", "CMC", "SAHF", "PUSHFQ", "POPFQ", "LEAVE", "PAUSE", "LODSB", "SCASB", "SCASD", "STOSB", "MOVSB",
    "MOVSD", "CMPSB", "CMPSD",
];

/// The registry (rules/__init__.py `registry`, 60-74): iclass -> rule, for the classes carried so far.
pub fn rule(iclass: &str) -> Option<Rule> {
    Some(match iclass {
        "ADD" | "SUB" | "AND" | "OR" | "XOR" | "CMP" | "TEST" => alu,
        "MOV" | "MOVZX" | "MOVSXD" | "LEA" => mov,
        "JMP" | "CALL_NEAR" => jmp_call,
        "RET_NEAR" => ret_near,
        "PUSH" => push,
        "POP" => pop,
        "NOP" | "PREFETCHNTA" | "PREFETCHT0" | "PREFETCHT1" | "PREFETCHT2" | "PREFETCHW" => nop,
        ic if ic.starts_with('J') && cc(&ic[1..]).is_some() => jcc,
        // group 2 (Sprint 153)
        "INC" | "DEC" => inc_dec,
        "NEG" => neg,
        "NOT" => not,
        "CDQE" => cdqe,
        "CQO" => cqo,
        "CWDE" => cwde,
        "CDQ" => cdq,
        "MOVSX" => movsx,
        "BSWAP" => bswap,
        "XCHG" => xchg,
        "SBB" => sbb,
        "ADC" => adc,
        "BSF" | "BSR" => bit_scan,
        "BT" | "BTS" | "BTR" | "BTC" => bit_test,
        "SHR" => shr,
        "SHL" | "SAR" | "ROL" | "ROR" => shift,
        "MUL" => mul,
        "IMUL" => imul,
        "XADD" => xadd,
        "CMPXCHG" => cmpxchg,
        "DIV" => div,
        "IDIV" => idiv,
        "POPCNT" => popcnt,
        "MOVBE" => movbe,
        "LEAVE" => leave,
        "ANDN" => andn,
        "SHLX" | "SHRX" => shift_x,
        "RORX" => rorx,
        "MULX" => mulx,
        ic if ic.starts_with("SET") && cc(&ic[3..]).is_some() => setcc,
        ic if ic.starts_with("CMOV") && cc(&ic[4..]).is_some() => cmov,
        // group 3 (Sprint 159)
        "ADD_LOCK" | "AND_LOCK" | "OR_LOCK" | "XOR_LOCK" | "SUB_LOCK" | "INC_LOCK" | "DEC_LOCK" | "NEG_LOCK"
        | "XADD_LOCK" | "CMPXCHG_LOCK" => locked,
        "CMPXCHG16B" | "CMPXCHG16B_LOCK" => cmpxchg16b,
        "CLD" | "STD" => df,
        "CPUID" => cpuid,
        "SFENCE" | "LFENCE" | "MFENCE" => fence,
        "PAUSE" => pause,
        "STMXCSR" | "LDMXCSR" | "VSTMXCSR" | "VLDMXCSR" => mxcsr,
        "FNSTCW" => fpu_control,
        "XGETBV" => xgetbv,
        ic if REP_STRING.contains(&ic) => rep_string,
        "SHLD" | "SHRD" => double_shift,
        "RCR" => rcr,
        "STC" | "CLC" | "CMC" | "SAHF" | "PUSHFQ" | "POPFQ" | "LODSB" | "SCASB" | "SCASD" | "STOSB" | "MOVSB"
        | "MOVSD" | "CMPSB" | "CMPSD" => implicit,
        "LOOP" | "LOOPE" | "LOOPNE" | "JRCXZ" => loop_,
        _ => return None,
    })
}

/// The classes the registry holds, for the census and the tests.
pub const GROUP_1: [&str; 38] = [
    "ADD",
    "SUB",
    "AND",
    "OR",
    "XOR",
    "CMP",
    "TEST",
    "MOV",
    "MOVZX",
    "MOVSXD",
    "LEA",
    "JO",
    "JNO",
    "JB",
    "JNB",
    "JZ",
    "JNZ",
    "JBE",
    "JNBE",
    "JS",
    "JNS",
    "JP",
    "JNP",
    "JL",
    "JNL",
    "JLE",
    "JNLE",
    "JMP",
    "CALL_NEAR",
    "RET_NEAR",
    "PUSH",
    "POP",
    "NOP",
    "PREFETCHNTA",
    "PREFETCHT0",
    "PREFETCHT1",
    "PREFETCHT2",
    "PREFETCHW",
];

fn loc_of(o: Operand, what: &str, ctx: &Ctx<'_>) -> Result<Loc, LiftError> {
    match o {
        Operand::Loc(l) => Ok(l),
        _ => Err(LiftError(format!(
            "{} at 0x{:x}: {what} is not a location",
            ctx.ic, ctx.address
        ))),
    }
}

/// `nop` (_int.py 16-17).
fn nop(_ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    Ok(())
}

/// `ret_near` (_int.py 32-46): RET, and RET imm16: in 64-bit mode RSP <- RSP + 8 + imm16, the count zero-extended
/// (SDM RET).
fn ret_near(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    if ctx.ops.is_empty() {
        let t = pop_value(ctx.b);
        ctx.b.emit(Op::Br { cond: "always", t });
        return Ok(());
    }
    let (count, _) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let count = val(ctx.b, count);
    let b = &mut *ctx.b;
    let (sp, target, eight, popped, nsp) = (b.t(), b.t(), b.t(), b.t(), b.t());
    b.emit(Op::Ld {
        t: sp,
        loc: Loc::R {
            idx: RSP,
            bits: 64,
            hi: false,
        },
    });
    b.emit(Op::Ld {
        t: target,
        loc: Loc::M { addr: sp, bytes: 8 },
    });
    b.emit(Op::Imm { t: eight, value: 8 });
    b.emit(Op::Alu {
        t: popped,
        op: "add",
        a: sp,
        b: eight,
        bits: 64,
    });
    b.emit(Op::Alu {
        t: nsp,
        op: "add",
        a: popped,
        b: count,
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
    b.emit(Op::Br {
        cond: "always",
        t: target,
    });
    Ok(())
}

/// `move` (_int.py 49-61): MOV, MOVZX, MOVSXD, LEA.
fn mov(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, dbits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let (s, sbits) = operand(ctx.b, &ctx.ops[1], ctx.nxt, Some(dbits))?;
    let mut v = val(ctx.b, s);
    if ctx.ic == "MOVSXD" && sbits == 32 && dbits == 64 {
        let w = ctx.b.t();
        ctx.b.emit(Op::Alu {
            t: w,
            op: "sext32",
            a: v,
            b: v,
            bits: 64,
        });
        v = w;
    } else if ctx.ic == "MOVSXD" && sbits != dbits {
        return Err(LiftError(format!("MOVSXD {sbits}->{dbits}")));
    }
    let dst = loc_of(d, "the destination", ctx)?;
    ctx.b.emit(Op::St { loc: dst, t: v });
    Ok(())
}

/// `alu` (_int.py 64-73): ADD SUB AND OR XOR CMP TEST.
fn alu(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let (s, _) = operand(ctx.b, &ctx.ops[1], ctx.nxt, Some(bits))?;
    let op = alu_op(&ctx.ic).expect("the registry names only ALU classes here");
    let a = val(ctx.b, d);
    let c = val(ctx.b, s);
    let r = ctx.b.t();
    ctx.b.emit(Op::Alu {
        t: r,
        op,
        a,
        b: c,
        bits,
    });
    if ctx.ic != "CMP" && ctx.ic != "TEST" {
        let dst = loc_of(d, "the destination", ctx)?;
        ctx.b.emit(Op::St { loc: dst, t: r }); // store before flags: the commit rule
    }
    let kind = if matches!(op, "and" | "or" | "xor") {
        "logic"
    } else {
        op
    };
    ctx.b.emit(Op::Flags { kind, a, b: c, r, bits });
    Ok(())
}

/// `jcc` (_int.py 417-419).
fn jcc(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (o, _) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let Operand::Imm(t) = o else {
        return Err(LiftError(format!(
            "{} at 0x{:x}: a target that is not immediate",
            ctx.ic, ctx.address
        )));
    };
    let cond = cc(&ctx.ic[1..]).expect("the registry names only Jcc classes here");
    ctx.b.emit(Op::Br { cond, t });
    Ok(())
}

/// `jmp_call` (_int.py 422-430).
fn jmp_call(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (o, _) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let target = val(ctx.b, o);
    if ctx.ic == "CALL_NEAR" {
        let ret = ctx.b.t();
        ctx.b.emit(Op::Imm { t: ret, value: ctx.nxt });
        push_value(ctx.b, ret, 8);
    }
    ctx.b.emit(Op::Br {
        cond: "always",
        t: target,
    });
    Ok(())
}

/// `push` (_int.py 433-441): operand size 64 by default, 16 with 66 (SDM PUSH: "decrements the stack pointer by 2"
/// for a 16-bit operand).
fn push(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let size = if ctx.f.opsize == 16 { 2 } else { 8 };
    let (o, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, Some(8 * size))?;
    if !matches!(bits, 8 | 16 | 32 | 64) || (matches!(o, Operand::Loc(_)) && bits != 8 * size) {
        return Err(LiftError(format!("PUSH {bits}-bit")));
    }
    let v = val(ctx.b, o);
    push_value(ctx.b, v, size);
    Ok(())
}

/// `pop` (_int.py 444-469).
fn pop(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (o, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let dst = loc_of(o, "the destination", ctx)?;
    if bits != 16 && bits != 64 {
        return Err(LiftError(format!("POP {bits}-bit")));
    }
    let size = bits / 8; // 66 POP: RSP += 2
    if let Loc::M { addr, .. } = dst {
        // SDM POP: "If the ESP register is used as a base register for addressing a destination operand in memory,
        // the POP instruction computes the effective address of the operand after it increments the ESP register."
        // The address was computed from the old RSP; the new one is `size` higher.
        for op in ctx.b.ops.iter_mut() {
            if let Op::Addr {
                t,
                base: Some(RSP),
                disp,
                ..
            } = op
                && *t == addr
            {
                *disp += i64::from(size);
            }
        }
    }
    let b = &mut *ctx.b;
    let (sp, step, nsp, v) = (b.t(), b.t(), b.t(), b.t());
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
        loc: Loc::M { addr: sp, bytes: size },
    });
    b.emit(Op::Imm {
        t: step,
        value: u64::from(size),
    });
    b.emit(Op::Alu {
        t: nsp,
        op: "add",
        a: sp,
        b: step,
        bits: 64,
    });
    if matches!(dst, Loc::M { .. }) {
        b.emit(Op::St { loc: dst, t: v }); // the store before RSP moves: the commit rule
        b.emit(Op::St {
            loc: Loc::R {
                idx: RSP,
                bits: 64,
                hi: false,
            },
            t: nsp,
        });
    } else {
        b.emit(Op::St {
            loc: Loc::R {
                idx: RSP,
                bits: 64,
                hi: false,
            },
            t: nsp,
        }); // POP RSP: the popped value wins
        b.emit(Op::St { loc: dst, t: v });
    }
    Ok(())
}

// ---- group 2 (Sprint 153): the rest of `_int.py`'s rules lowered in A64, and `bmi.py` ------------------------------

fn r(idx: u8, bits: u32) -> Loc {
    Loc::R { idx, bits, hi: false }
}

/// The location `o` names, or the operand error the Python's tuple index would have raised.
fn loc(o: Operand, ctx: &Ctx<'_>) -> Result<Loc, LiftError> {
    loc_of(o, "the operand", ctx)
}

/// `inc_dec` (_int.py 76-83).
fn inc_dec(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let a = val(ctx.b, d);
    let (one, res) = (ctx.b.t(), ctx.b.t());
    let inc = ctx.ic == "INC";
    ctx.b.emit(Op::Imm { t: one, value: 1 });
    ctx.b.emit(Op::Alu {
        t: res,
        op: if inc { "add" } else { "sub" },
        a,
        b: one,
        bits,
    });
    ctx.b.emit(Op::St {
        loc: loc(d, ctx)?,
        t: res,
    });
    ctx.b.emit(Op::Flags {
        kind: if inc { "inc" } else { "dec" },
        a,
        b: one,
        r: res,
        bits,
    });
    Ok(())
}

/// `neg` (_int.py 86-95): 0 - dest, with SUB's flags for it.
fn neg(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let zero = ctx.b.t();
    let a = val(ctx.b, d);
    let res = ctx.b.t();
    ctx.b.emit(Op::Imm { t: zero, value: 0 });
    ctx.b.emit(Op::Alu {
        t: res,
        op: "sub",
        a: zero,
        b: a,
        bits,
    });
    ctx.b.emit(Op::St {
        loc: loc(d, ctx)?,
        t: res,
    });
    ctx.b.emit(Op::Flags {
        kind: "sub",
        a: zero,
        b: a,
        r: res,
        bits,
    });
    Ok(())
}

/// `not_` (_int.py 98-105): one's complement; no flag changes.
fn not(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let a = val(ctx.b, d);
    let (ones, res) = (ctx.b.t(), ctx.b.t());
    let all = if bits == 64 { u64::MAX } else { (1u64 << bits) - 1 };
    ctx.b.emit(Op::Imm { t: ones, value: all });
    ctx.b.emit(Op::Alu {
        t: res,
        op: "xor",
        a,
        b: ones,
        bits,
    });
    ctx.b.emit(Op::St {
        loc: loc(d, ctx)?,
        t: res,
    });
    Ok(())
}

/// `cdqe` (_int.py 108-114): RAX = sign-extended EAX.
fn cdqe(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let b = &mut *ctx.b;
    let (v, res) = (b.t(), b.t());
    b.emit(Op::Ld { t: v, loc: r(RAX, 32) });
    b.emit(Op::Alu {
        t: res,
        op: "sext32",
        a: v,
        b: v,
        bits: 64,
    });
    b.emit(Op::St {
        loc: r(RAX, 64),
        t: res,
    });
    Ok(())
}

/// `cqo` (_int.py 117-124): RDX = every bit RAX's sign.
fn cqo(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let b = &mut *ctx.b;
    let (v, n, res) = (b.t(), b.t(), b.t());
    b.emit(Op::Ld { t: v, loc: r(RAX, 64) });
    b.emit(Op::Imm { t: n, value: 63 });
    b.emit(Op::Alu {
        t: res,
        op: "sar",
        a: v,
        b: n,
        bits: 64,
    });
    b.emit(Op::St {
        loc: r(RDX, 64),
        t: res,
    });
    Ok(())
}

/// `cwde` (_int.py 127-133): EAX = sign-extended AX; the 32-bit write zero-extends RAX.
fn cwde(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let b = &mut *ctx.b;
    let (v, res) = (b.t(), b.t());
    b.emit(Op::Ld { t: v, loc: r(RAX, 16) });
    b.emit(Op::Alu {
        t: res,
        op: "sext16",
        a: v,
        b: v,
        bits: 32,
    });
    b.emit(Op::St {
        loc: r(RAX, 32),
        t: res,
    });
    Ok(())
}

/// `cdq` (_int.py 136-143): EDX = every bit EAX's sign; a 32-bit write zero-extends RDX.
fn cdq(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let b = &mut *ctx.b;
    let (v, n, res) = (b.t(), b.t(), b.t());
    b.emit(Op::Ld { t: v, loc: r(RAX, 32) });
    b.emit(Op::Imm { t: n, value: 31 });
    b.emit(Op::Alu {
        t: res,
        op: "sar",
        a: v,
        b: n,
        bits: 32,
    });
    b.emit(Op::St {
        loc: r(RDX, 32),
        t: res,
    });
    Ok(())
}

/// `movsx` (_int.py 146-155): MOVSX r, r/m8 or r/m16, sign-extended to the destination.
fn movsx(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, dbits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let (s, sbits) = operand(ctx.b, &ctx.ops[1], ctx.nxt, None)?;
    if sbits != 8 && sbits != 16 {
        return Err(LiftError(format!("MOVSX from {sbits} bits")));
    }
    let v = val(ctx.b, s);
    let res = ctx.b.t();
    ctx.b.emit(Op::Alu {
        t: res,
        op: if sbits == 8 { "sext8" } else { "sext16" },
        a: v,
        b: v,
        bits: dbits,
    });
    ctx.b.emit(Op::St {
        loc: loc(d, ctx)?,
        t: res,
    });
    Ok(())
}

/// `bswap` (_int.py 158-166): BSWAP r32/r64 (the 16-bit form is undefined and not lifted).
fn bswap(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    if bits != 32 && bits != 64 {
        return Err(LiftError(format!("BSWAP r{bits}: undefined (SDM)")));
    }
    let v = val(ctx.b, d);
    let res = ctx.b.t();
    ctx.b.emit(Op::Alu {
        t: res,
        op: "bswap",
        a: v,
        b: v,
        bits,
    });
    ctx.b.emit(Op::St {
        loc: loc(d, ctx)?,
        t: res,
    });
    Ok(())
}

/// `xchg` (_int.py 169-178): the two operands swap; the memory store comes first (commit rule).
fn xchg(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (ao, _) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let (bo, _) = operand(ctx.b, &ctx.ops[1], ctx.nxt, None)?;
    let a = val(ctx.b, ao);
    let c = val(ctx.b, bo);
    let (a_loc, b_loc) = (loc(ao, ctx)?, loc(bo, ctx)?);
    if matches!(a_loc, Loc::M { .. }) {
        ctx.b.emit(Op::St { loc: a_loc, t: c });
        ctx.b.emit(Op::St { loc: b_loc, t: a });
    } else {
        ctx.b.emit(Op::St { loc: b_loc, t: a });
        ctx.b.emit(Op::St { loc: a_loc, t: c });
    }
    Ok(())
}

/// `sbb` and `adc` (_int.py 181-200): dest -/+ src -/+ CF, flags as a subtraction with borrow or addition with carry.
fn carry_op(ctx: &mut Ctx<'_>, op: &'static str) -> Result<(), LiftError> {
    let (d, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let (s, _) = operand(ctx.b, &ctx.ops[1], ctx.nxt, Some(bits))?;
    let a = val(ctx.b, d);
    let c = val(ctx.b, s);
    let res = ctx.b.t();
    ctx.b.emit(Op::Alu {
        t: res,
        op,
        a,
        b: c,
        bits,
    });
    ctx.b.emit(Op::St {
        loc: loc(d, ctx)?,
        t: res,
    });
    ctx.b.emit(Op::Flags {
        kind: op,
        a,
        b: c,
        r: res,
        bits,
    });
    Ok(())
}

fn sbb(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    carry_op(ctx, "sbb")
}

fn adc(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    carry_op(ctx, "adc")
}

/// `bit_scan` (_int.py 203-213): BSF, BSR; ZF set and the destination not written when the source is 0.
fn bit_scan(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let (s, _) = operand(ctx.b, &ctx.ops[1], ctx.nxt, Some(bits))?;
    let v = val(ctx.b, s);
    let res = ctx.b.t();
    ctx.b.emit(Op::Alu {
        t: res,
        op: if ctx.ic == "BSF" { "bsf" } else { "bsr" },
        a: v,
        b: v,
        bits,
    });
    ctx.b.emit(Op::Cst {
        c: v,
        loc: loc(d, ctx)?,
        t: res,
    });
    ctx.b.emit(Op::Flags {
        kind: "bsf",
        a: v,
        b: v,
        r: res,
        bits,
    });
    Ok(())
}

/// `bit_test` (_int.py 216-263): BT, BTS, BTR, BTC; a register offset with a memory base addresses the bit string.
fn bit_test(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let (o, obits) = operand(ctx.b, &ctx.ops[1], ctx.nxt, None)?;
    let off = val(ctx.b, o);
    let log = u64::from(bits.ilog2()); // 16 -> 4, 32 -> 5, 64 -> 6
    let mut dst = loc(d, ctx)?;
    if let (Loc::M { addr: base_addr, bytes }, Operand::Loc(_)) = (dst, o) {
        let mut wide = off;
        if obits != 64 {
            wide = ctx.b.t();
            ctx.b.emit(Op::Alu {
                t: wide,
                op: if obits == 32 { "sext32" } else { "sext16" },
                a: off,
                b: off,
                bits: 64,
            });
        }
        let b = &mut *ctx.b;
        let (shift, words, step, scaled, addr) = (b.t(), b.t(), b.t(), b.t(), b.t());
        b.emit(Op::Imm { t: shift, value: log });
        b.emit(Op::Alu {
            t: words,
            op: "sar",
            a: wide,
            b: shift,
            bits: 64,
        });
        b.emit(Op::Imm {
            t: step,
            value: log - 3,
        });
        b.emit(Op::Alu {
            t: scaled,
            op: "shl",
            a: words,
            b: step,
            bits: 64,
        });
        b.emit(Op::Alu {
            t: addr,
            op: "add",
            a: base_addr,
            b: scaled,
            bits: 64,
        });
        let abits = b
            .ops
            .iter()
            .find_map(|op| match op {
                Op::Addr { t, abits, .. } if *t == base_addr => Some(*abits),
                _ => None,
            })
            .expect("the memory operand's address op");
        let mut addr = addr;
        if abits == 32 {
            // a 0x67 prefix: the address wraps at 32 bits
            let (low, wrapped) = (b.t(), b.t());
            b.emit(Op::Imm {
                t: low,
                value: 0xFFFF_FFFF,
            });
            b.emit(Op::Alu {
                t: wrapped,
                op: "and",
                a: addr,
                b: low,
                bits: 64,
            });
            addr = wrapped;
        }
        dst = Loc::M { addr, bytes };
    }
    let b = &mut *ctx.b;
    let (mask, bit, shifted, one, cf) = (b.t(), b.t(), b.t(), b.t(), b.t());
    b.emit(Op::Imm {
        t: mask,
        value: u64::from(bits - 1),
    });
    b.emit(Op::Alu {
        t: bit,
        op: "and",
        a: off,
        b: mask,
        bits: 64,
    });
    let value = val(b, Operand::Loc(dst));
    b.emit(Op::Alu {
        t: shifted,
        op: "shr",
        a: value,
        b: bit,
        bits,
    });
    b.emit(Op::Imm { t: one, value: 1 });
    b.emit(Op::Alu {
        t: cf,
        op: "and",
        a: shifted,
        b: one,
        bits,
    });
    if matches!(ctx.ic.as_str(), "BTS" | "BTR" | "BTC") {
        let (sel, result) = (b.t(), b.t());
        b.emit(Op::Alu {
            t: sel,
            op: "shl",
            a: one,
            b: bit,
            bits,
        });
        if ctx.ic == "BTR" {
            let (ones, notsel) = (b.t(), b.t());
            let all = if bits == 64 { u64::MAX } else { (1u64 << bits) - 1 };
            b.emit(Op::Imm { t: ones, value: all });
            b.emit(Op::Alu {
                t: notsel,
                op: "xor",
                a: sel,
                b: ones,
                bits,
            });
            b.emit(Op::Alu {
                t: result,
                op: "and",
                a: value,
                b: notsel,
                bits,
            });
        } else {
            let op = if ctx.ic == "BTC" { "xor" } else { "or" };
            b.emit(Op::Alu {
                t: result,
                op,
                a: value,
                b: sel,
                bits,
            });
        }
        b.emit(Op::St { loc: dst, t: result });
    }
    b.emit(Op::Flags {
        kind: "bt",
        a: cf,
        b: cf,
        r: cf,
        bits,
    });
    Ok(())
}

/// `shr` (_int.py 376-384).
fn shr(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let (s, _) = operand(ctx.b, &ctx.ops[1], ctx.nxt, None)?;
    let a = val(ctx.b, d);
    let res = ctx.b.t();
    let cnt = val(ctx.b, s);
    ctx.b.emit(Op::Alu {
        t: res,
        op: "shr",
        a,
        b: cnt,
        bits,
    });
    ctx.b.emit(Op::St {
        loc: loc(d, ctx)?,
        t: res,
    });
    ctx.b.emit(Op::Flags {
        kind: "shr",
        a,
        b: cnt,
        r: res,
        bits,
    });
    Ok(())
}

/// `_count` (_int.py 472-479): a shift's count, CL or an immediate; the D0/D1 forms carry an implicit 1.
fn count(ctx: &mut Ctx<'_>) -> Result<Operand, LiftError> {
    if ctx.ops.len() > 1 {
        return Ok(operand(ctx.b, &ctx.ops[1], ctx.nxt, None)?.0);
    }
    let one = ctx.b.t();
    ctx.b.emit(Op::Imm { t: one, value: 1 });
    Ok(Operand::Imm(one))
}

/// `shift` (_int.py 482-493): SHL (and SAL), SAR, ROL, ROR, the result stored before the flags.
fn shift(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let op = match ctx.ic.as_str() {
        "SHL" => "shl",
        "SAR" => "sar",
        "ROL" => "rol",
        _ => "ror",
    };
    let (d, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let a = val(ctx.b, d);
    let c = count(ctx)?;
    let cnt = val(ctx.b, c);
    let res = ctx.b.t();
    ctx.b.emit(Op::Alu {
        t: res,
        op,
        a,
        b: cnt,
        bits,
    });
    ctx.b.emit(Op::St {
        loc: loc(d, ctx)?,
        t: res,
    });
    ctx.b.emit(Op::Flags {
        kind: op,
        a,
        b: cnt,
        r: res,
        bits,
    });
    Ok(())
}

/// `mul` (_int.py 387-409): AX = AL * r/m8, DX:AX, EDX:EAX, RDX:RAX = the accumulator * r/m.
fn mul(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (s, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    if bits != 64 {
        // the exact product in 64 bits, split
        let a = ctx.b.t();
        let c = val(ctx.b, s);
        let b = &mut *ctx.b;
        let (p, k, hi) = (b.t(), b.t(), b.t());
        b.emit(Op::Ld {
            t: a,
            loc: r(RAX, bits),
        });
        b.emit(Op::Alu {
            t: p,
            op: "mul_lo",
            a,
            b: c,
            bits: 64,
        });
        b.emit(Op::Imm {
            t: k,
            value: u64::from(bits),
        });
        b.emit(Op::Alu {
            t: hi,
            op: "shr",
            a: p,
            b: k,
            bits: 64,
        });
        b.emit(Op::Flags {
            kind: "mul",
            a,
            b: c,
            r: hi,
            bits,
        });
        if bits == 8 {
            b.emit(Op::St { loc: r(RAX, 16), t: p });
        } else {
            b.emit(Op::St {
                loc: r(RAX, bits),
                t: p,
            });
            b.emit(Op::St {
                loc: r(RDX, bits),
                t: hi,
            });
        }
        return Ok(());
    }
    let a = ctx.b.t();
    let c = val(ctx.b, s);
    let b = &mut *ctx.b;
    let (lo, hi) = (b.t(), b.t());
    b.emit(Op::Ld { t: a, loc: r(RAX, 64) });
    b.emit(Op::Alu {
        t: lo,
        op: "mul_lo",
        a,
        b: c,
        bits: 64,
    });
    b.emit(Op::Alu {
        t: hi,
        op: "mul_hi",
        a,
        b: c,
        bits: 64,
    });
    b.emit(Op::Flags {
        kind: "mul",
        a,
        b: c,
        r: hi,
        bits: 64,
    });
    b.emit(Op::St { loc: r(RAX, 64), t: lo });
    b.emit(Op::St { loc: r(RDX, 64), t: hi });
    Ok(())
}

/// `imul` (_int.py 496-526): one operand (the accumulator * r/m), two (r = r * r/m) or three (r = r/m * imm); CF and
/// OF on a truncated signed product.
fn imul(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    if ctx.ops.len() == 1 {
        let (s, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
        let a = ctx.b.t();
        let c = val(ctx.b, s);
        let b = &mut *ctx.b;
        b.emit(Op::Ld {
            t: a,
            loc: r(RAX, bits),
        });
        let (lo, hi) = (b.t(), b.t());
        b.emit(Op::Alu {
            t: lo,
            op: "smul_lo",
            a,
            b: c,
            bits,
        });
        b.emit(Op::Alu {
            t: hi,
            op: "smul_hi",
            a,
            b: c,
            bits,
        });
        if bits == 8 {
            // AX = AL * r/m8
            b.emit(Op::St { loc: r(RAX, 8), t: lo });
            b.emit(Op::St {
                loc: Loc::R {
                    idx: RAX,
                    bits: 8,
                    hi: true,
                },
                t: hi,
            });
        } else {
            b.emit(Op::St {
                loc: r(RAX, bits),
                t: lo,
            });
            b.emit(Op::St {
                loc: r(RDX, bits),
                t: hi,
            });
        }
        b.emit(Op::Flags {
            kind: "imul",
            a,
            b: c,
            r: lo,
            bits,
        });
        return Ok(());
    }
    let (d, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let (a, c) = if ctx.ops.len() == 2 {
        let (s, _) = operand(ctx.b, &ctx.ops[1], ctx.nxt, Some(bits))?;
        let a = val(ctx.b, d);
        (a, val(ctx.b, s))
    } else {
        let (s, _) = operand(ctx.b, &ctx.ops[1], ctx.nxt, Some(bits))?;
        let (i, _) = operand(ctx.b, &ctx.ops[2], ctx.nxt, Some(bits))?;
        let a = val(ctx.b, s);
        (a, val(ctx.b, i))
    };
    let res = ctx.b.t();
    ctx.b.emit(Op::Alu {
        t: res,
        op: "smul_lo",
        a,
        b: c,
        bits,
    });
    ctx.b.emit(Op::St {
        loc: loc(d, ctx)?,
        t: res,
    });
    ctx.b.emit(Op::Flags {
        kind: "imul",
        a,
        b: c,
        r: res,
        bits,
    });
    Ok(())
}

/// `setcc` (_int.py 412-414).
fn setcc(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, _) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let cond = cc(&ctx.ic[3..]).expect("the registry names only SETcc classes here");
    ctx.b.emit(Op::Set {
        cond,
        loc: loc(d, ctx)?,
    });
    Ok(())
}

/// `cmov` (_int.py 529-534): the source is read whether or not the condition holds.
fn cmov(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let (s, _) = operand(ctx.b, &ctx.ops[1], ctx.nxt, Some(bits))?;
    let cond = cc(&ctx.ic[4..]).expect("the registry names only CMOVcc classes here");
    let t = val(ctx.b, s);
    ctx.b.emit(Op::Cmov {
        cond,
        loc: loc(d, ctx)?,
        t,
    });
    Ok(())
}

/// `xadd` (_int.py 537-551): TEMP = SRC + DEST; SRC = DEST; DEST = TEMP; a memory destination stored first.
fn xadd(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let (s, _) = operand(ctx.b, &ctx.ops[1], ctx.nxt, None)?;
    let a = val(ctx.b, d);
    let c = val(ctx.b, s);
    let res = ctx.b.t();
    let (dst, src) = (loc(d, ctx)?, loc(s, ctx)?);
    ctx.b.emit(Op::Alu {
        t: res,
        op: "add",
        a,
        b: c,
        bits,
    });
    if matches!(dst, Loc::M { .. }) {
        ctx.b.emit(Op::St { loc: dst, t: res });
        ctx.b.emit(Op::St { loc: src, t: a });
    } else {
        ctx.b.emit(Op::St { loc: src, t: a });
        ctx.b.emit(Op::St { loc: dst, t: res });
    }
    ctx.b.emit(Op::Flags {
        kind: "add",
        a,
        b: c,
        r: res,
        bits,
    });
    Ok(())
}

/// `cmpxchg` (_int.py 554-575): compare the accumulator with DEST; equal: DEST = SRC; else the accumulator = DEST. A
/// memory DEST is always written; a register DEST only when equal.
fn cmpxchg(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let (s, _) = operand(ctx.b, &ctx.ops[1], ctx.nxt, None)?;
    let acc = r(RAX, bits);
    let a = ctx.b.t();
    let dv = val(ctx.b, d);
    let sv = val(ctx.b, s);
    let dst = loc(d, ctx)?;
    let b = &mut *ctx.b;
    b.emit(Op::Ld { t: a, loc: acc });
    let res = b.t();
    b.emit(Op::Alu {
        t: res,
        op: "sub",
        a,
        b: dv,
        bits,
    }); // nonzero exactly when they differ
    if matches!(dst, Loc::M { .. }) {
        let new = b.t();
        b.emit(Op::Sel {
            t: new,
            c: res,
            x: dv,
            y: sv,
        }); // differ: DEST written back; equal: SRC
        b.emit(Op::St { loc: dst, t: new });
    } else {
        b.emit(Op::Cstz {
            c: res,
            loc: dst,
            t: sv,
        }); // equal: DEST = SRC
    }
    b.emit(Op::Cst {
        c: res,
        loc: acc,
        t: dv,
    }); // differ: the accumulator gets DEST
    b.emit(Op::Flags {
        kind: "sub",
        a,
        b: dv,
        r: res,
        bits,
    });
    Ok(())
}

/// `div` and `idiv` (_int.py 362-373).
fn divide(ctx: &mut Ctx<'_>, signed: bool) -> Result<(), LiftError> {
    let (s, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let t = val(ctx.b, s);
    ctx.b.emit(Op::Div { bits, t, signed });
    Ok(())
}

fn div(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    divide(ctx, false)
}

fn idiv(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    divide(ctx, true)
}

/// `popcnt` (_int.py 632-642).
fn popcnt(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let (s, _) = operand(ctx.b, &ctx.ops[1], ctx.nxt, Some(bits))?;
    let a = val(ctx.b, s);
    let res = ctx.b.t();
    ctx.b.emit(Op::Alu {
        t: res,
        op: "popcnt",
        a,
        b: a,
        bits,
    });
    ctx.b.emit(Op::St {
        loc: loc(d, ctx)?,
        t: res,
    });
    ctx.b.emit(Op::Flags {
        kind: "popcnt",
        a,
        b: a,
        r: res,
        bits,
    });
    Ok(())
}

/// `leave` (_int.py 654-666): RSP = RBP, then RBP = the popped quadword; the load first. 16-bit LEAVE is not lifted.
fn leave(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    if ctx.f.opsize == 16 {
        return Err(LiftError(format!("LEAVE with 16-bit operands at 0x{:x}", ctx.address)));
    }
    let b = &mut *ctx.b;
    let (rbp, v, eight, nsp) = (b.t(), b.t(), b.t(), b.t());
    b.emit(Op::Ld { t: rbp, loc: r(5, 64) });
    b.emit(Op::Ld {
        t: v,
        loc: Loc::M { addr: rbp, bytes: 8 },
    });
    b.emit(Op::Imm { t: eight, value: 8 });
    b.emit(Op::Alu {
        t: nsp,
        op: "add",
        a: rbp,
        b: eight,
        bits: 64,
    });
    b.emit(Op::St {
        loc: r(RSP, 64),
        t: nsp,
    });
    b.emit(Op::St { loc: r(5, 64), t: v });
    Ok(())
}

/// `movbe` (_int.py 669-686): the bytes reversed on the way; a register in the memory operand's place is #UD.
fn movbe(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let memory = usize::from(ctx.f.opcode == 0xF0);
    if !ctx.ops[memory].starts_with("M:") {
        ctx.b.emit(Op::Ud);
        return Ok(());
    }
    let (d, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let (s, _) = operand(ctx.b, &ctx.ops[1], ctx.nxt, Some(bits))?;
    let v = val(ctx.b, s);
    let res = ctx.b.t();
    if bits == 16 {
        let eight = ctx.b.t();
        ctx.b.emit(Op::Imm { t: eight, value: 8 });
        ctx.b.emit(Op::Alu {
            t: res,
            op: "rol",
            a: v,
            b: eight,
            bits: 16,
        });
    } else {
        ctx.b.emit(Op::Alu {
            t: res,
            op: "bswap",
            a: v,
            b: v,
            bits,
        });
    }
    ctx.b.emit(Op::St {
        loc: loc(d, ctx)?,
        t: res,
    });
    Ok(())
}

/// `andn` (_int.py 281-292): ANDN r, vvvv, r/m: r = ~vvvv & r/m.
fn andn(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let (vs, _) = operand(ctx.b, &ctx.ops[1], ctx.nxt, None)?;
    let (s, _) = operand(ctx.b, &ctx.ops[2], ctx.nxt, Some(bits))?;
    let a = val(ctx.b, vs);
    let c = val(ctx.b, s);
    let b = &mut *ctx.b;
    let (ones, inv, res) = (b.t(), b.t(), b.t());
    b.emit(Op::Imm {
        t: ones,
        value: u64::MAX,
    });
    b.emit(Op::Alu {
        t: inv,
        op: "xor",
        a,
        b: ones,
        bits,
    });
    b.emit(Op::Alu {
        t: res,
        op: "and",
        a: inv,
        b: c,
        bits,
    });
    let dst = loc(d, ctx)?;
    ctx.b.emit(Op::St { loc: dst, t: res });
    ctx.b.emit(Op::Flags {
        kind: "andn",
        a: inv,
        b: c,
        r: res,
        bits,
    });
    Ok(())
}

/// `shift_x` (_int.py 295-303): SHLX, SHRX r, r/m, vvvv; no flag changes.
fn shift_x(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let (s, _) = operand(ctx.b, &ctx.ops[1], ctx.nxt, Some(bits))?;
    let (cn, _) = operand(ctx.b, &ctx.ops[2], ctx.nxt, None)?;
    let a = val(ctx.b, s);
    let c = val(ctx.b, cn);
    let res = ctx.b.t();
    let op = if ctx.ic == "SHLX" { "shl" } else { "shr" };
    ctx.b.emit(Op::Alu {
        t: res,
        op,
        a,
        b: c,
        bits,
    });
    ctx.b.emit(Op::St {
        loc: loc(d, ctx)?,
        t: res,
    });
    Ok(())
}

/// `rorx` (_int.py 306-314): RORX r, r/m, imm8; no flag changes.
fn rorx(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let (s, _) = operand(ctx.b, &ctx.ops[1], ctx.nxt, Some(bits))?;
    let (i, _) = operand(ctx.b, &ctx.ops[2], ctx.nxt, Some(bits))?;
    let a = val(ctx.b, s);
    let c = val(ctx.b, i);
    let res = ctx.b.t();
    ctx.b.emit(Op::Alu {
        t: res,
        op: "ror",
        a,
        b: c,
        bits,
    });
    ctx.b.emit(Op::St {
        loc: loc(d, ctx)?,
        t: res,
    });
    Ok(())
}

/// `mulx` (_int.py 317-336): RDX (EDX) * r/m unsigned; the low half to vvvv first, then the high half.
fn mulx(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (hi_o, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let (lo_o, _) = operand(ctx.b, &ctx.ops[1], ctx.nxt, None)?;
    let (s, _) = operand(ctx.b, &ctx.ops[2], ctx.nxt, Some(bits))?;
    let rdx = ctx.b.t();
    let c = val(ctx.b, s);
    let (hi_dst, lo_dst) = (loc(hi_o, ctx)?, loc(lo_o, ctx)?);
    let b = &mut *ctx.b;
    b.emit(Op::Ld {
        t: rdx,
        loc: r(2, bits),
    });
    let (lo, hi) = (b.t(), b.t());
    if bits == 64 {
        b.emit(Op::Alu {
            t: lo,
            op: "mul_lo",
            a: rdx,
            b: c,
            bits: 64,
        });
        b.emit(Op::Alu {
            t: hi,
            op: "mul_hi",
            a: rdx,
            b: c,
            bits: 64,
        });
    } else {
        // 32 x 32 bits: the 64-bit product, split
        let k = b.t();
        b.emit(Op::Alu {
            t: lo,
            op: "mul_lo",
            a: rdx,
            b: c,
            bits: 64,
        });
        b.emit(Op::Imm { t: k, value: 32 });
        b.emit(Op::Alu {
            t: hi,
            op: "shr",
            a: lo,
            b: k,
            bits: 64,
        });
    }
    b.emit(Op::St { loc: lo_dst, t: lo });
    b.emit(Op::St { loc: hi_dst, t: hi });
    Ok(())
}

// group 3 (Sprint 159): the locked forms, CMPXCHG16B, the system ops and the model's steps

/// `locked` (_int.py 590-596): a LOCK-prefixed form (iclass X_LOCK), the base rule's operations under the base iclass;
/// the lowering makes the locked forms atomic. The instruction keeps its own iclass (`lift` takes it from the decode).
fn locked(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let base = ctx.ic.trim_end_matches("_LOCK").to_string();
    let rule = rule(&base).ok_or_else(|| LiftError(format!("no lifting rule for {base}")))?;
    ctx.ic = base;
    rule(ctx)
}

/// `cmpxchg16b` (_int.py 578-587): RDX:RAX against the 16 bytes at m128; one op, atomic in the VM with or without LOCK.
/// CMPXCHG8B (no REX.W) is not lifted.
fn cmpxchg16b(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    match (d, bits) {
        (Operand::Loc(Loc::M { addr, .. }), 128) => {
            ctx.b.emit(Op::Cx16 { addr });
            Ok(())
        }
        _ => Err(LiftError(format!("CMPXCHG8B at 0x{:x}", ctx.address))),
    }
}

/// `df` (_int.py 24-25): CLD, STD.
fn df(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let set = u32::from(ctx.ic == "STD");
    ctx.b.emit(Op::Df { set });
    Ok(())
}

/// `cpuid` (_int.py 28-29): EAX, EBX, ECX, EDX from the guest profile.
fn cpuid(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    ctx.b.emit(Op::Cpuid);
    Ok(())
}

/// `sfence` (_int.py 346-348): SFENCE, LFENCE, MFENCE, the ordering each gives, as one fence kind.
fn fence(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let kind = match ctx.ic.as_str() {
        "SFENCE" => "st",
        "LFENCE" => "ld",
        _ => "full",
    };
    ctx.b.emit(Op::Fence { kind });
    Ok(())
}

/// `pause` (_int.py 20-21): no architectural effect; the VM counts it.
fn pause(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    ctx.b.emit(Op::Pause);
    Ok(())
}

/// `mxcsr` (_int.py 275-278): STMXCSR, LDMXCSR m32, and their VEX forms VSTMXCSR, VLDMXCSR.
fn mxcsr(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, _) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let loc = loc(d, ctx)?;
    let store = ctx.ic.ends_with("STMXCSR");
    ctx.b.emit(Op::Mxcsr { store, loc });
    Ok(())
}

/// `fpu_control` (_int.py 339-343): FNSTCW m2byte, the x87 control word to memory.
fn fpu_control(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, _) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let loc = loc(d, ctx)?;
    ctx.b.emit(Op::Fcw { loc });
    Ok(())
}

/// `xgetbv` (_int.py 356-359): EDX:EAX = XCR[ECX], XCR0 = 7; any ECX but 0 is #GP. The model runs it.
fn xgetbv(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    ctx.b.emit(Op::Xgetbv);
    Ok(())
}

/// `rep_string` (_int.py 266-272): REP STOS and REP MOVS with 64-bit addresses.
fn rep_string(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    if ctx.f.prefixes.contains(&0x67) {
        return Err(LiftError(format!(
            "{} with 32-bit addresses at 0x{:x}",
            ctx.ic, ctx.address
        )));
    }
    // lift.py 94: REP_{STOS,MOVS}{B,W,D,Q} -> (stos|movs, 1|2|4|8)
    let kind = if ctx.ic.starts_with("REP_STOS") { "stos" } else { "movs" };
    let size = match ctx.ic.as_bytes()[ctx.ic.len() - 1] {
        b'B' => 1,
        b'W' => 2,
        b'D' => 4,
        _ => 8,
    };
    ctx.b.emit(Op::Rep { kind, size });
    Ok(())
}

/// `_model` (_int.py 599-601): one instruction the model computes whole; the VM runs it as a model step.
fn model(ctx: &mut Ctx<'_>, dst: Option<Loc>, srcs: Vec<crate::lift::Temp>, bits: u32) {
    let ic = ctx.ic.clone();
    ctx.b.emit(Op::X86 { ic, dst, srcs, bits });
}

/// `double_shift` (_int.py 604-610): SHLD, SHRD r/m, r, imm8 or CL.
fn double_shift(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let (s, _) = operand(ctx.b, &ctx.ops[1], ctx.nxt, Some(bits))?;
    let (c, _) = operand(ctx.b, &ctx.ops[2], ctx.nxt, None)?;
    let srcs = vec![val(ctx.b, d), val(ctx.b, s), val(ctx.b, c)];
    let dst = loc(d, ctx)?;
    model(ctx, Some(dst), srcs, bits);
    Ok(())
}

/// `rcr` (_int.py 613-618): RCR r/m, 1, CL or imm8, rotate right through CF.
fn rcr(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let (d, bits) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let c = count(ctx)?;
    let srcs = vec![val(ctx.b, d), val(ctx.b, c)];
    let dst = loc(d, ctx)?;
    model(ctx, Some(dst), srcs, bits);
    Ok(())
}

/// `implicit` (_int.py 621-629): STC, CLC, CMC, SAHF, PUSHFQ, POPFQ, and the single string forms, every operand
/// implicit. A 0x67 prefix on a string form or a 66 prefix on PUSHF and POPF is not lifted.
fn implicit(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    let ic = ctx.ic.as_str();
    let strings = !matches!(ic, "STC" | "CLC" | "CMC" | "SAHF" | "PUSHFQ" | "POPFQ");
    let p = &ctx.f.prefixes;
    if (p.contains(&0x67) && strings) || (p.contains(&0x66) && matches!(ic, "PUSHFQ" | "POPFQ")) {
        return Err(LiftError(format!(
            "{} with a 66 or 67 prefix at 0x{:x}",
            ctx.ic, ctx.address
        )));
    }
    model(ctx, None, vec![], 64);
    Ok(())
}

/// `loop` (_int.py 645-651): LOOP, LOOPE, LOOPNE, JRCXZ rel8, on RCX; ECX (a 0x67 prefix) is not lifted.
fn loop_(ctx: &mut Ctx<'_>) -> Result<(), LiftError> {
    if ctx.f.prefixes.contains(&0x67) {
        return Err(LiftError(format!("{} on ECX at 0x{:x}", ctx.ic, ctx.address)));
    }
    let (target, _) = operand(ctx.b, &ctx.ops[0], ctx.nxt, None)?;
    let srcs = vec![val(ctx.b, target)];
    model(ctx, None, srcs, 64);
    Ok(())
}
