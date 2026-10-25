// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The lifting rules, one per x86 instruction class (Sprint 151), carried from `src/w2m/rules` (one module per class,
//! Sprint 041) and its shared `_int.py`. Link 5 ports them in groups (`docs/design-host-compile-path-2026-10-24.md`);
//! group 1 is the integer and control-flow core: `alu` (ADD SUB AND OR XOR CMP TEST), `move` (MOV MOVZX MOVSXD LEA),
//! `jcc`, `jmp_call` (JMP CALL_NEAR), `ret_near`, `push`, `pop` and `nop` (NOP and the PREFETCH classes). Each rule
//! cites the `_int.py` lines it carries.

use crate::lift::{
    Ctx, LiftError, Loc, Op, Operand, RSP, alu_op, cc, operand, pop as pop_value, push as push_value, val,
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
