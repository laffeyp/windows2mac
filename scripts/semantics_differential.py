# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""semantics_differential.py — the lifter judged by a real x86 CPU, one instruction at a time
(Sprint 037; floating-point state and faults, Sprint 039).

  --cases [--out probes/semantics/cases.bin]   write the case file probes/semantics/semantics.c
                                                runs on the windows-2025 runner (format 2)
  --judge <run dir> [--record]                 compare the runner's result files with
                                                runtime.Runtime on the same cases

Cases: every encgen encoding (spec 0.7.1, seeded) that w2m decodes and lifts, except
control-flow rows, RIP-relative and absolute memory forms (their addresses are not placeable);
plus fixed control-flow encodings whose both paths are observable: Jcc rel8 +14 (all 16
conditions), JMP rel8 +14, CALL rel32 +14, RET to a planted address. Each runs from STATES random
states (SSE_STATES for legacy-SSE classes): 16 GPRs from width and sign boundaries and uniform
bits; CF, PF, AF, ZF, SF, OF, DF random; MXCSR with random rounding, DAZ, FTZ and sticky flags,
every exception masked in most states; XMM lanes from floating-point boundaries and uniform bits;
random data and stack windows. A memory operand's base register is set so the effective address
falls in the data window (index 0); RSP lies in the stack window. Each encoding with a memory
operand also gets one fault case: the access straddles the end of the data page at 0x3000_1000,
whose next page is unmapped, so the CPU faults and nothing of the access may be committed.

The reference is the CPU's result file; the model is runtime.Runtime stepping lift.py's
operations once from the same state at the same addresses. Compared: the path (RIP), all 16
GPRs, every flag the model defines after the step (a flag the model marks undefined is not
compared: the SDM leaves it undefined), DF, MXCSR, XMM0-15, the three windows; for a fault, the
fault code, the faulting RIP and the whole state at the fault.

Format 1 (Sprint 037's case file, now store/probe-runs/semantics-037-cases.bin) is read by
read_cases_format1 and judged by judge_format1.
"""

from __future__ import annotations

import argparse
import functools
import hashlib
import json
import random
import struct
import sys
import time
from collections import Counter
from collections.abc import Iterator
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from w2m import x86fp as fp  # noqa: E402
from w2m.encgen import SPEC, cases as encgen_cases  # noqa: E402
from w2m.kernel32 import M64  # noqa: E402
from w2m.lift import SSE, LiftError, Lifted, lift  # noqa: E402
from w2m.loader import GuestFault, Loaded, Memory  # noqa: E402
from w2m.runtime import Runtime  # noqa: E402
from w2m.x86dec import DecodeError, decode  # noqa: E402
from w2m.x86dec_tables import SPEC_VERSION  # noqa: E402
from w2m.x86ops import operands  # noqa: E402

CODE_VA, DATA_VA, STACK_VA = 0x2000_0000, 0x3000_0000, 0x4000_0000
DATA_OFF, STACK_OFF, WINDOW, CODE_BYTES = 0x40, 0x87C0, 64, 32
EDGE_OFF, EDGE = 0xFE0, 32                              # the last 32 bytes of the data page
PAGE_END = DATA_VA + 0x1000
STACK_SIZE = 0x10000
RET_TARGET = CODE_VA + 0x100
STATES = 4
SSE_STATES = 8
CONTROL_STATES = 16
FLAG_BITS = {"C": 0, "P": 2, "A": 4, "Z": 6, "S": 7, "O": 11}
DF_BIT = 10
CASE_SIZE = 8 + CODE_BYTES + 8 * 16 + 8 + 8 + 16 * 16 + WINDOW + WINDOW + EDGE
RESULT_SIZE = 8 * 16 + 8 * 5 + 16 * 16 + WINDOW + WINDOW + EDGE
CASE_SIZE_1 = 8 + CODE_BYTES + 8 * 16 + 8 + WINDOW + WINDOW
RESULT_SIZE_1 = 8 * 16 + 8 + 8 + 8 + WINDOW + WINDOW
EDGE_VALUES = (0, 1, 0x7F, 0x80, 0xFF, 0x7FFF, 0x8000, 0xFFFF, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF,
               0x7FFF_FFFF_FFFF_FFFF, 0x8000_0000_0000_0000, M64)
# Windows reports a page fault, and #GP on a misaligned SSE operand, as an access violation
# (ntstatus.h); #XM as the STATUS_FLOAT_* code x86fp.xm_status picks
ACCESS_VIOLATION = 0xC000_0005
F32_EDGES = (0, 0x8000_0000, 1, 0x007F_FFFF, 0x0080_0000, 0x0080_0001, 0x7F7F_FFFF, 0xFF7F_FFFF,
             0x7F80_0000, 0xFF80_0000, 0x7FC0_0000, 0xFFC0_0000, 0x7F80_0001, 0x7FA0_0000, 0xFFC1_2345,
             0x3F80_0000, 0xBF80_0000, 0x3F00_0000, 0x3FC0_0000, 0x4020_0000, 0xBFC0_0000, 0x3F7F_FFFE,
             0x4F00_0000, 0xCF00_0000, 0x4EFF_FFFF, 0xCF00_0001, 0x5F00_0000, 0xDF00_0000, 0x5EFF_FFFF,
             0x3EAA_AAAB, 0x0C00_0000, 0x7200_0000)
F64_EDGES = (0, 0x8000_0000_0000_0000, 1, 0x000F_FFFF_FFFF_FFFF, 0x0010_0000_0000_0000,
             0x0010_0000_0000_0001, 0x7FEF_FFFF_FFFF_FFFF, 0x7FF0_0000_0000_0000, 0xFFF0_0000_0000_0000,
             0x7FF8_0000_0000_0000, 0xFFF8_0000_0000_0000, 0x7FF0_0000_0000_0001, 0x7FF4_0000_0000_0000,
             0x3FF0_0000_0000_0000, 0xBFF0_0000_0000_0000, 0x3FE0_0000_0000_0000, 0x3FF8_0000_0000_0000,
             0x4004_0000_0000_0000, 0x3FEF_FFFF_FFFF_FFFF, 0x41E0_0000_0000_0000, 0xC1E0_0000_0000_0000,
             0x41DF_FFFF_FFC0_0000, 0x41DF_FFFF_FFE0_0000, 0xC1E0_0000_0020_0000, 0x43E0_0000_0000_0000,
             0xC3E0_0000_0000_0000, 0x43DF_FFFF_FFFF_FFFF, 0x3FD5_5555_5555_5555, 0x47EF_FFFF_E000_0000,
             0x47EF_FFFF_F000_0000, 0x36A0_0000_0000_0000, 0x3690_0000_0000_0000, 0x3810_0000_0000_0000)


@dataclass
class Case:
    code: bytes
    regs: list[int]
    rflags: int
    data: bytes
    stack: bytes
    label: str                    # iclass, for grouping
    mxcsr: int = fp.MXCSR_DEFAULT
    xmm: list[int] = field(default_factory=lambda: [0] * 16)
    edge: bytes = bytes(EDGE)
    kind: int = 0                 # 0 ordinary, 1 fault placement
    ymm_hi: list[int] = field(default_factory=lambda: [0] * 16)   # YMM bits 255:128 (format 3, Sprint 049)


def _value(rng: random.Random) -> int:
    r = rng.random()
    if r < 0.4:
        v = rng.choice(EDGE_VALUES)
    elif r < 0.7:
        v = rng.getrandbits(rng.choice((8, 16, 32)))
    else:
        v = rng.getrandbits(64)
    return v ^ (rng.getrandbits(64) & 0xFFFF_FFFF_0000_0000 if rng.random() < 0.2 else 0)


def _fp_lane(rng: random.Random, f: fp.Fmt) -> int:
    r = rng.random()
    if r < 0.5:
        return rng.choice(F32_EDGES if f is fp.F32 else F64_EDGES)
    if r < 0.85:                                  # an ordinary number near 1: rounding is exercised
        e = f.bias + rng.randrange(-30, 31)
        return rng.getrandbits(1) << (f.bits - 1) | e << f.mbits | rng.getrandbits(f.mbits)
    return rng.getrandbits(f.bits)


def _xmm(rng: random.Random) -> int:
    f = fp.F32 if rng.getrandbits(1) else fp.F64
    return fp.join([_fp_lane(rng, f) for _ in range(128 // f.bits)], f.bits)


def _mxcsr(rng: random.Random) -> int:
    v = rng.getrandbits(2) << 13
    v |= fp.DAZ if rng.random() < 0.25 else 0
    v |= fp.FTZ if rng.random() < 0.25 else 0
    v |= rng.getrandbits(6) if rng.random() < 0.5 else 0
    v |= (0x3F if rng.random() < 0.8 else rng.getrandbits(6)) << fp.MASK_SHIFT
    return v


def _state(rng: random.Random, sse: bool, ymm: bool = False) -> Case:
    regs = [_value(rng) for _ in range(16)]
    regs[4] = STACK_VA + STACK_OFF + 0x20 + 8 * rng.randrange(4)      # push and pop stay inside the window
    rflags = 0x2 | sum(1 << b for b in FLAG_BITS.values() if rng.getrandbits(1))
    rflags |= rng.getrandbits(1) << DF_BIT
    if sse:
        data = b"".join(_xmm(rng).to_bytes(16, "little") for _ in range(WINDOW // 16))
    else:
        data = rng.randbytes(WINDOW)
    c = Case(b"", regs, rflags, data, rng.randbytes(WINDOW), "", _mxcsr(rng),
             [_xmm(rng) for _ in range(16)], rng.randbytes(EDGE))
    if ymm:                                                    # format-3 sets only: older sets draw nothing more
        c.ymm_hi = [_xmm(rng) for _ in range(16)]
    return c


def _place(ins: Lifted, regs: list[int], rng: random.Random, target_of: Any = None, size: int = 0) -> bool:
    """Point each memory operand into the data window (or at `target_of(size)`). False for forms
    whose address is not placeable (RIP-relative and absolute). `size` (format-3 sets, Sprint 049): an
    operand wider than 24 bytes starts where all of it lies in the window (offsets 0, 8, ... 64 - size)."""
    for op in ins.ops:
        if op[0] != "addr":
            continue
        _, _, base, index, scale, disp, abits, _seg = op
        if base is None:
            return False
        mask = M64 if abits == 64 else 0xFFFF_FFFF
        slots = (WINDOW - size) // 8 + 1 if size > 24 else 6
        target = target_of() if target_of else DATA_VA + DATA_OFF + 8 * rng.randrange(slots)
        if index is not None and index != base:
            regs[index] = 0
        if index == base:                                            # [r + r*scale]: solve r*(1+scale)
            lo = target - 40 if target_of else DATA_VA + DATA_OFF
            hit = next((t for t in range(lo, lo + 41) if ((t - disp) & mask) % (1 + scale) == 0
                        and (not target_of or t == target)), None)
            if hit is None:
                return False
            regs[base] = ((hit - disp) & mask) // (1 + scale)
            continue
        value = (target - disp) & mask
        if abits == 32:
            value |= rng.getrandbits(32) << 32                    # the CPU truncates; so must the model
        regs[base] = value
    return True


def _dispatchable(c: Case) -> bool:
    """False when the case moves RSP out of the stack mapping and the model says it faults:
    Windows dispatches an exception on the faulting thread's stack, so the probe process would
    die (run 38010542937: [rsp+rbp*2+disp] placed at the page edge)."""
    if STACK_VA + 0x1000 <= c.regs[4] < STACK_VA + STACK_SIZE:
        return True
    m = model(c)
    return isinstance(m, str) or not m.fault


def _memory_size(raw: bytes) -> int | None:
    d = decode(raw)
    if d.fields is None or not d.fields.ops:
        return None
    sizes = [int(o.split(":")[6]) for o in operands(d.fields) if o.startswith("M:")]
    return sizes[0] if sizes else None


def _strings(classes: frozenset[str], rng: random.Random) -> Iterator[Case]:
    """REP STOS / REP MOVS states that stay inside the compared windows: RDI in the data window
    (RSI in the stack window for MOVS) with a count that fits, DF clear or set, a zero count; and,
    one state in four, RDI near the data page's end with a count that runs past it, so the stores
    fault part way (SDM Vol 1 7.3.9: the registers show the faulting iteration)."""
    for label in sorted(STRING_ENCODINGS):
        if label not in classes:
            continue
        code, size = bytes.fromhex(STRING_ENCODINGS[label]), {"B": 1, "W": 2, "D": 4, "Q": 8}[label[-1]]
        for i in range(STRING_STATES):
            s = _state(rng, False)
            df = rng.getrandbits(1)
            s.rflags = (s.rflags & ~(1 << DF_BIT)) | df << DF_BIT
            if i % 4 == 3:                                     # runs off the data page: faults part way
                done = rng.randrange(1, 4)
                s.regs[7] = PAGE_END - done * size if not df else PAGE_END - size
                s.regs[1] = done + rng.randrange(1, 9) if not df else rng.randrange(1, 4)
                s.kind = 1 if not df else 0
            else:
                count = 0 if i % 8 == 0 else rng.randrange(1, WINDOW // size + 1)
                span = count * size
                lo = DATA_VA + DATA_OFF
                first = lo + rng.randrange(0, WINDOW - span + 1)
                s.regs[7] = first if not df else first + max(span - size, 0)
                s.regs[1] = count
            src_lo = STACK_VA + STACK_OFF
            s.regs[6] = src_lo + (rng.randrange(0, WINDOW - s.regs[1] * size + 1) if s.regs[1] * size <= WINDOW else 0)
            if df and s.regs[1]:
                s.regs[6] += max(s.regs[1] * size - size, 0)
            s.code, s.label = code, label
            yield s


CX16_ENCODINGS = {"CMPXCHG16B": "480fc70e", "CMPXCHG16B_LOCK": "f0480fc70e"}    # cmpxchg16b [rsi]
CX16_STATES = 24


def _cmpxchg16b(classes: frozenset[str], rng: random.Random) -> Iterator[Case]:
    """CMPXCHG16B states the random generator almost never makes: RSI on a 16-byte boundary in the data window,
    and in half of them the 16 bytes there equal RDX:RAX, so the exchange succeeds; one in four is 8 bytes
    off a boundary (#GP)."""
    for label in sorted(CX16_ENCODINGS):
        if label not in classes:
            continue
        for i in range(CX16_STATES):
            s = _state(rng, False)
            off = 16 * rng.randrange(0, WINDOW // 16) if i % 4 != 3 else 16 * rng.randrange(0, WINDOW // 16 - 1) + 8
            s.regs[6] = DATA_VA + DATA_OFF + off
            if i % 2 == 0 and i % 4 != 3:
                value = (s.regs[2] << 64 | s.regs[0]).to_bytes(16, "little")
                s.data = s.data[:off] + value + s.data[off + 16:]
            s.code, s.label = bytes.fromhex(CX16_ENCODINGS[label]), label
            yield s


# pcmpistri xmm1, xmm2 / xmm1, [rsi] / REX.W xmm1, xmm2, each followed by imm8 (Sprint 047)
STRING_COMPARE_FORMS = (("PCMPISTRI", "660f3a63ca"), ("PCMPISTRI", "660f3a630e"), ("PCMPISTRI64", "66480f3a63ca"))
STRING_COMPARE_STATES = 4


def _text(rng: random.Random) -> bytes:
    """16 bytes a string compare finds structure in: a small alphabet (signed and unsigned extremes
    among them), and in most a zero byte (or word) that ends the implicit length."""
    alphabet = rng.choice((b"ab", b"abc\x80\xff", b"a\x00", bytes(range(256))))
    v = bytearray(rng.choice(alphabet) for _ in range(16))
    if rng.random() < 0.6:
        z = rng.randrange(16)
        v[z] = 0
        if rng.getrandbits(1):
            v[z ^ 1] = 0
    return bytes(v)


def _string_compares(classes: frozenset[str], rng: random.Random) -> Iterator[Case]:
    """PCMPISTRI states over all 128 control bytes (bit 7 is ignored): both operands from `_text`; the
    memory form reads at RSI in the data window, unaligned, and one state in eight runs off the data
    page's end (the read faults before ECX or the flags change)."""
    for label, prefix in STRING_COMPARE_FORMS:
        if label not in classes:
            continue
        memory = prefix.endswith("0e")
        for imm in range(128):
            for i in range(STRING_COMPARE_STATES):
                s = _state(rng, True)
                s.xmm[1] = int.from_bytes(_text(rng), "little")
                s.xmm[2] = int.from_bytes(_text(rng), "little")
                if memory:
                    if (imm * STRING_COMPARE_STATES + i) % 8 == 7:
                        s.regs[6], s.kind = PAGE_END - rng.randrange(1, 16), 1
                    else:
                        off = rng.randrange(0, WINDOW - 16 + 1)
                        s.regs[6] = DATA_VA + DATA_OFF + off
                        s.data = s.data[:off] + _text(rng) + s.data[off + 16:]
                s.code, s.label = bytes.fromhex(prefix) + bytes([imm]), label
                yield s


def _control_flow() -> Iterator[tuple[bytes, str]]:
    for cc in range(16):
        yield bytes([0x70 + cc, 0x0E]), f"Jcc_{cc:x}"
    yield bytes([0xEB, 0x0E]), "JMP"
    yield bytes([0xE8, 0x0E, 0, 0, 0]), "CALL_NEAR"
    yield bytes([0xC3]), "RET_NEAR"


# The instruction classes Sprint 039's runs executed (cases.sha256 3094485413…). The generator keeps
# exactly these, so a lifter that learns a new class (CPUID, Sprint 040) does not change a case file
# Windows already ran; new classes get cases in a new run.
CLASSES_039 = frozenset((
    "ADD", "ADDPD", "ADDPS", "ADDSD", "ADDSS", "AND", "ANDPD", "ANDPS", "CALL_NEAR", "CLD", "CMP",
    "COMISD", "COMISS", "CVTSD2SI", "CVTSD2SS", "CVTSI2SD", "CVTSI2SS", "CVTSS2SD", "CVTSS2SI",
    "CVTTSD2SI", "CVTTSS2SI", "DEC", "DIVPD", "DIVPS", "DIVSD", "DIVSS", "INC", "JMP", "LEA", "MAXPD",
    "MAXPS", "MAXSD", "MAXSS", "MINPD", "MINPS", "MINSD", "MINSS", "MOV", "MOVAPD", "MOVAPS", "MOVD",
    "MOVQ", "MOVSD_XMM", "MOVSS", "MOVSXD", "MOVUPD", "MOVUPS", "MOVZX", "MUL", "MULPD", "MULPS",
    "MULSD", "MULSS", "NOP", "OR", "POP", "PUSH", "RET_NEAR", "SETB", "SETBE", "SETL", "SETLE",
    "SETNB", "SETNBE", "SETNL", "SETNLE", "SETNO", "SETNP", "SETNS", "SETNZ", "SETO", "SETP", "SETS",
    "SETZ", "SHR", "SQRTPD", "SQRTPS", "SQRTSD", "SQRTSS", "STD", "SUB", "SUBPD", "SUBPS", "SUBSD",
    "SUBSS", "TEST", "UCOMISD", "UCOMISS", "XOR", "XORPD", "XORPS",
))


# Sprint 041's block, chosen by frequency in the game's code (Sprint 036): IMUL, CMOVcc, the shifts and
# rotates, XADD, CMPXCHG, and the LOCK forms of the lockable classes w2m lifts. The generator never emits
# a LOCK prefix, so each memory-destination case of a lockable class also appears with F0 in front.
CMOV_CC = ("O", "NO", "B", "NB", "Z", "NZ", "BE", "NBE", "S", "NS", "P", "NP", "L", "NL", "LE", "NLE")
LOCKABLE_041 = ("ADD", "SUB", "AND", "OR", "XOR", "INC", "DEC", "XADD", "CMPXCHG")
CLASSES_041 = frozenset(("IMUL", "SHL", "SAR", "ROL", "ROR", "XADD", "CMPXCHG", *(f"CMOV{c}" for c in CMOV_CC),
                         *(f"{c}_LOCK" for c in LOCKABLE_041)))
# Sprint 042: NEG, the one class the CreateFileW probe's code (both runner binaries) has no rule for
LOCKABLE_042 = ("NEG",)
CLASSES_042 = frozenset(("NEG", "NEG_LOCK"))
# Sprint 044: the integer classes the launcher's reachable code uses without a rule (census 2026-10-10)
CLASSES_044 = frozenset(("NOT", "CDQE", "CDQ", "MOVSX", "BSWAP", "XCHG", "SBB", "BT", "BTS", "BTR", "DIV"))
# Sprint 044 (second block): REP STOS and REP MOVS, from fixed encodings with controlled states
# (_strings): random registers would send RDI anywhere with a count near 2^64
CLASSES_044B = frozenset(f"REP_{k}{s}" for k in ("STOS", "MOVS") for s in "BWDQ")
STRING_ENCODINGS = {"REP_STOSB": "f3aa", "REP_STOSW": "66f3ab", "REP_STOSD": "f3ab", "REP_STOSQ": "f348ab",
                    "REP_MOVSB": "f3a4", "REP_MOVSW": "66f3a5", "REP_MOVSD": "f3a5", "REP_MOVSQ": "f348a5"}
STRING_STATES = 24
# Sprint 044 (third block): ADC (the thread-storage probe's code), BSF, BSR
CLASSES_044C = frozenset(("ADC", "BSF", "BSR"))
# Sprint 044 (fourth block): the SSE2 integer block, MXCSR access, SFENCE, PREFETCHNTA (XGETBV is judged
# by the guest profile: the runners' CPUs set OSXSAVE)
CLASSES_044D = frozenset(("MOVDQA", "MOVDQU", "MOVNTPS", "PAND", "POR", "ORPS", "PSUBD", "PSUBQ", "PSRLQ", "PSRLDQ",
                          "PSLLDQ", "PUNPCKLBW", "PCMPEQB", "PSHUFD", "PMOVMSKB", "CVTDQ2PD", "STMXCSR", "LDMXCSR",
                          "SFENCE", "PREFETCHNTA"))
# Sprint 044 (fifth block): RET imm16 (C2 iw), where the launcher stops after its code pages (frontier
# 2026-10-10, 17,914 instructions). RET_NEAR's C3 form is set 039's; this set holds only C2, at immediates
# that tell a zero-extended count from a sign-extended one (8000, FFFF) and odd counts from aligned ones.
CLASSES_044E = frozenset(("RET_NEAR",))
RET_IMMEDIATES = (0, 1, 2, 3, 8, 0x10, 0x18, 0x20, 0x28, 0x100, 0x7FF8, 0x7FFF, 0x8000, 0x8008, 0xFFF8, 0xFFFF)
# Sprint 046: the second SSE block, the classes UnityPlayer.dll reaches most without a rule that raise no
# floating-point exception (census 2026-10-10; SHUFPS stops UnityPlayer's start-up at 152,691 instructions)
CLASSES_046 = frozenset(("SHUFPS", "PXOR", "ANDNPS", "UNPCKLPS", "UNPCKHPS", "MOVHLPS", "MOVLHPS", "PUNPCKLDQ", "PADDD",
                         "PSRLD", "PSLLD", "PSRAD", "MOVMSKPS"))
# Sprint 046 (second block): CQO, where UnityPlayer's start-up stops after its system queries (173,624
# instructions with GetNativeSystemInfo stubbed), CWDE beside it, and IDIV
CLASSES_046B = frozenset(("CQO", "CWDE", "IDIV"))
# Sprint 046 (third block): CMPXCHG16B, plain and LOCK, where UnityPlayer's DllMain stops after its
# semaphore (scratch run, 281,745 instructions); the profile advertises CMPXCHG16B
CLASSES_046C = frozenset(("CMPXCHG16B", "CMPXCHG16B_LOCK"))
LOCKABLE_046C = ("CMPXCHG16B",)
# Sprint 046 (fourth block): the SSE2 integer classes UnityPlayer.dll reaches most (census 2026-10-10); PUNPCKLQDQ
# stops the static C runtime's DLL start-up in w2m
CLASSES_046D = frozenset(('PUNPCKLQDQ', 'PADDQ', 'PCMPGTD', 'PCMPEQD', 'PANDN', 'PSUBUSB', 'PMAXUB', 'UNPCKLPD', 'PACKUSWB', 'PADDSB', 'PADDSW', 'PADDW', 'PMULLW', 'PSUBSB', 'PSUBW', 'PUNPCKHBW', 'PUNPCKHDQ', 'PUNPCKHWD', 'PUNPCKLWD', 'PSLLQ', 'PSRLW', 'PSRAW', 'PSHUFLW', 'PSHUFHW', 'PINSRW', 'MOVHPS'))
# Sprint 046 (fifth block): RCPPS, RSQRTPS (UnityPlayer's DllMain stops at RSQRTPS, 292,735 instructions) and
# their scalar forms; judged on Intel only, whose tables w2m reproduces (AMD's differ)
CLASSES_046E = frozenset(("RCPPS", "RSQRTPS", "RCPSS", "RSQRTSS"))
# Sprint 047: the classes UnityMain's start-up stops on (scripts/frontier_lookahead.py): PCMPISTRI (also constructed
# over every control byte), UNPCKHPD, MUL's 8-, 16- and 32-bit forms, BTC, LFENCE and MFENCE
CLASSES_047 = frozenset(("PCMPISTRI", "PCMPISTRI64", "UNPCKHPD", "MUL", "BTC", "LFENCE", "MFENCE"))
# Sprint 048: the prefetch hints the frontier meets (PREFETCHW at 0x18143e96c) and their siblings
CLASSES_048 = frozenset(("PREFETCHW", "PREFETCHT0", "PREFETCHT1", "PREFETCHT2"))
# Sprint 049: the guest profile is x86-64-v3; the VEX classes mono-2.0-bdwgc.dll reaches (its census, 2026-10-11),
# the legacy forms they rest on, and FNSTCW, where mono's start-up stopped (3,020,967 instructions). Format 3:
# the case and result records carry YMM0-15 bits 255:128
CLASSES_049 = frozenset(("ORPD", "PCMPEQW", "CVTTPD2DQ", "CVTPD2DQ", "CMPSD_XMM", "FNSTCW", "VZEROUPPER",
                         "VMULSD", "VADDSD", "VSUBSD", "VDIVSD", "VMOVSD", "VMOVQ", "VMOVD", "VMOVAPD", "VMOVDQA",
                         "VMOVDQU", "VCOMISD", "VUCOMISD", "VPAND", "VPOR", "VPXOR", "VXORPS", "VXORPD", "VORPD",
                         "VANDPD", "VANDPS", "VPCMPEQB", "VPCMPEQW", "VPSUBQ", "VPSUBD", "VPADDQ", "VCVTDQ2PD",
                         "VPSRLQ", "VPSLLQ", "VPSRLDQ", "VPMOVMSKB", "VCVTTPD2DQ", "VCVTPD2DQ", "VCMPSD", "VMOVLHPS",
                         "VHADDPD", "VFMADD213SD", "VFMADD231SD", "VFMADD132SD", "VFNMADD231SD", "VFNMADD132SD",
                         "VFNMADD213SD", "VFMSUB213SD", "VFMSUB132SD", "VFMADD132PD"))
# Sprint 049 (049b): with AVX reported, UnityPlayer's VEX and BMI classes (its census, 2026-10-11), where the frontier
# stopped at VMOVUPS (384,526 instructions); AES, PCLMULQDQ, FMA4 and XOP are not reported and not lifted
CLASSES_049B = frozenset((
    "VMOVUPS", "VMOVAPS", "VPADDD", "VPSRLD", "VPSLLD", "VPSRAD", "VPSLLDQ", "VPANDN", "VPUNPCKLDQ", "VMULPS",
    "VADDPS", "VSUBPS", "VMAXPS", "VMINPS", "VPSHUFD", "VUNPCKHPS", "VUNPCKLPS", "VPCMPEQD", "VPCMPGTD", "VMULSS",
    "VADDSS", "VSUBSS", "VDIVSS", "VCOMISS", "VUCOMISS", "VMOVSS", "VCVTSD2SS", "VCVTSS2SD", "VRCPPS", "VRSQRTPS",
    "VANDNPS", "VORPS", "VMULPD", "VADDPD", "VSHUFPS", "VUNPCKLPD", "VCVTSS2SI", "VCVTSI2SS", "VCVTTSD2SI", "VPADDB",
    "VPMULUDQ", "VPUNPCKHQDQ", "VPCMPEQQ", "VPSHUFB", "VPALIGNR", "VPINSRD", "VPINSRQ", "VPEXTRQ", "VANDNPD",
    "VCVTDQ2PS", "VCVTTPS2DQ", "VCMPPS", "VROUNDSD", "VPBLENDD", "VCVTPS2PD", "VPMOVSXDQ", "VPERMILPS", "VPERMQ",
    "VPERMD", "VPBROADCASTD", "VPBROADCASTQ", "VBROADCASTSS", "VBROADCASTF128", "VINSERTF128", "VINSERTI128",
    "VEXTRACTF128", "VEXTRACTI128", "VZEROALL", "VFMADD213SS", "VFMADD231SS", "VFMSUB213SS", "VFMADD231PS",
    "VFNMADD231PS", "VLDMXCSR", "VSTMXCSR", "ANDN", "SHLX", "SHRX", "RORX", "MULX"))
FORMAT_3 = frozenset(("049", "049b"))              # sets whose records carry the YMM upper halves
# set -> (seed, class set, classes given LOCK-prefixed memory variants, fixed control-flow encodings)
CASE_SETS: dict[str, tuple[int, frozenset[str], tuple[str, ...], bool]] = {
    "039": (2026_10_09_39, CLASSES_039, (), True),
    "041": (2026_10_10_41, CLASSES_041, LOCKABLE_041, False),
    "042": (2026_10_11_42, CLASSES_042, LOCKABLE_042, False),
    "044": (2026_10_10_44, CLASSES_044, (), False),
    "044b": (2026_10_10_441, CLASSES_044B, (), False),
    "044c": (2026_10_10_442, CLASSES_044C, (), False),
    "044d": (2026_10_10_443, CLASSES_044D, (), False),
    "044e": (2026_10_10_444, CLASSES_044E, (), False),
    "046": (2026_10_10_46, CLASSES_046, (), False),
    "046b": (2026_10_10_461, CLASSES_046B, (), False),
    "046c": (2026_10_10_462, CLASSES_046C, LOCKABLE_046C, False),
    "046d": (2026_10_10_463, CLASSES_046D, (), False),
    "046e": (2026_10_10_464, CLASSES_046E, (), False),
    "047": (2026_10_10_47, CLASSES_047, (), False),
    "048": (2026_10_10_48, CLASSES_048, (), False),
    "049": (2026_10_11_49, CLASSES_049, (), False),
    "049b": (2026_10_11_491, CLASSES_049B, (), False)}


def _locked_variants(lockable: tuple[str, ...]) -> Iterator[Any]:
    """F0-prefixed copies of the generator's memory-destination cases of the `lockable` classes."""
    for c in encgen_cases():
        try:
            d = decode(c.raw)
        except DecodeError:
            continue
        if d.iclass in lockable and d.fields is not None and d.fields.ops and operands(d.fields)[0][0] == "M":
            yield type(c)(raw=b"\xf0" + c.raw[:14], row=c.row, note=c.note + " +lock")


def build_cases(seed: int = 2026_10_09_39, classes: frozenset[str] = CLASSES_039,
                locked: tuple[str, ...] = (), control_flow: bool = True) -> tuple[list[Case], Counter[str]]:
    rng = random.Random(seed)
    ymm = classes in (CLASSES_049, CLASSES_049B)    # format 3 (FORMAT_3): states carry YMM upper halves
    out: list[Case] = []
    skipped: Counter[str] = Counter()
    source = list(encgen_cases()) + list(_locked_variants(locked))
    for c in source:
        try:
            d = decode(c.raw)
            ins = lift(c.raw[:d.length] + b"\x90" * 15, CODE_VA)
        except (DecodeError, LiftError, IndexError) as e:
            skipped[f"not lifted: {type(e).__name__}"] += 1
            continue
        if d.iclass not in classes:
            skipped["not in this case file's class set"] += 1
            continue
        if d.fields is not None and d.fields.ops and (0x64 in d.fields.prefixes or 0x65 in d.fields.prefixes) and \
                any(o[0] in "MA" for o in operands(d.fields)):
            # an FS/GS override with a memory or address operand: the lifter refused these before Sprint 044
            # and lifts them since (GS adds the TEB; LEA ignores the segment); never placeable here (the
            # runner's GS base is its TEB). The same rule as the old refusal keeps the frozen case files
            # what the runners ran (KIT_DIARY #23)
            skipped["segment-relative operand (not placeable)"] += 1
            continue
        if classes is CLASSES_039 and d.iclass == "MUL" and \
                any(op[0] == "st" and op[1][0] == "r" and op[1][2] != 64 for op in ins.ops):
            # MUL r/m8, r/m16, r/m32: refused before Sprint 047 and lifted since; set 039 keeps the old refusal so its
            # frozen file stays what the runners ran (KIT_DIARY #23), and set 047 holds them
            skipped["MUL narrower than 64 bits (set 047)"] += 1
            continue
        if any(op[0] == "br" for op in ins.ops):
            skipped["control flow (fixed encodings instead)"] += 1
            continue
        raw = c.raw[:d.length]
        sse = d.iclass in SSE or (ymm and (d.iclass.startswith("V") or d.iclass == "FNSTCW"))
        if ymm and d.iclass in ("VZEROALL", "VZEROUPPER"):
            sse = True
        placeable = True
        wide = (_memory_size(c.raw[:d.length]) or 0) if ymm else 0
        for _ in range(SSE_STATES if sse else STATES):
            s = _state(rng, sse, ymm)
            if not _place(ins, s.regs, rng, size=wide):
                skipped["address not placeable (RIP-relative or absolute)"] += 1
                placeable = False
                break
            s.code, s.label = raw, d.iclass
            if not _dispatchable(s):
                skipped["fault with RSP outside the stack (not dispatchable)"] += 1
                continue
            out.append(s)
        size = _memory_size(raw)
        if placeable and size is not None:
            s = _state(rng, sse, ymm)
            target = PAGE_END - rng.randrange(1, size) if size > 1 else PAGE_END
            if _place(ins, s.regs, rng, lambda target=target: target):
                s.code, s.label, s.kind = raw, d.iclass, 1
                if _dispatchable(s):
                    out.append(s)
                else:
                    skipped["fault with RSP outside the stack (not dispatchable)"] += 1
            else:
                skipped["fault placement not solvable ([r + r*scale])"] += 1
    out.extend(_strings(classes, rng))                         # only sets with REP classes (044b on)
    out.extend(_cmpxchg16b(classes, rng))                     # only sets with CMPXCHG16B (046c on)
    out.extend(_string_compares(classes, rng))                # only sets with PCMPISTRI (047 on)
    if "RET_NEAR" in classes and not control_flow:              # 044e: RET imm16 (039 holds C3)
        for imm in RET_IMMEDIATES:
            for _ in range(CONTROL_STATES):
                s = _state(rng, False)
                off = s.regs[4] - (STACK_VA + STACK_OFF)
                s.stack = s.stack[:off] + RET_TARGET.to_bytes(8, "little") + s.stack[off + 8:]
                s.code, s.label = bytes([0xC2]) + imm.to_bytes(2, "little"), "RET_NEAR"
                out.append(s)
    for code, label in _control_flow() if control_flow else []:
        for _ in range(CONTROL_STATES):
            s = _state(rng, False)
            if label == "RET_NEAR":
                off = s.regs[4] - (STACK_VA + STACK_OFF)
                s.stack = s.stack[:off] + RET_TARGET.to_bytes(8, "little") + s.stack[off + 8:]
            s.code, s.label = code, label
            out.append(s)
    return out, skipped


_CASE_INPUTS = ("scripts/semantics_differential.py", "src/w2m/encgen.py", "specs/x86_decode.json", "src/w2m/lift.py",
                "src/w2m/x86dec.py", "src/w2m/x86dec_tables.py", "src/w2m/x86ops.py")


@functools.cache
def case_file(case_set: str) -> Path:
    """The committed case file the runners ran for `case_set` (set 039's sits in probes/semantics)."""
    return ROOT / "probes" / ("semantics" if case_set == "039" else f"semantics_{case_set}") / "cases.bin"


def cached_cases(case_set: str) -> list[Case]:
    """build_cases for `case_set`, cached on disk under build/case-cache/ by the sha256 of everything the
    build reads (this script, the encoding generator, the decode spec, the decoder, the operand parser,
    the lifter and its rules) and in the process. A full build of the six sets took 17 s, and the gate
    built them in several tests (Sprint 044)."""
    import pickle
    h = hashlib.sha256(repr(CASE_SETS[case_set]).encode())
    for rel in (*_CASE_INPUTS, *sorted(str(p.relative_to(ROOT)) for p in (ROOT / "src" / "w2m" / "rules").glob("*.py"))):
        h.update(rel.encode() + b"\0" + (ROOT / rel).read_bytes())
    path = ROOT / "build" / "case-cache" / f"{case_set}-{h.hexdigest()[:24]}.pickle"
    if path.exists():
        cases: list[Case] = pickle.loads(path.read_bytes())
        return cases
    seed, classes, locked, control = CASE_SETS[case_set]
    cases = build_cases(seed, classes, locked, control)[0]
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(pickle.dumps(cases))
    return cases


def write_cases(cases: list[Case], path: Path, fmt: int = 2) -> None:
    with path.open("wb") as f:
        f.write(b"W2MS" + struct.pack("<II", fmt, len(cases)))
        for c in cases:
            f.write(bytes([len(c.code), c.kind]) + bytes(6) + c.code.ljust(CODE_BYTES, b"\0"))
            f.write(struct.pack("<16QQQ", *c.regs, c.rflags, c.mxcsr))
            f.write(b"".join(x.to_bytes(16, "little") for x in c.xmm))
            if fmt == 3:
                f.write(b"".join(x.to_bytes(16, "little") for x in c.ymm_hi))
            f.write(c.data + c.stack + c.edge)


def case_format(path: Path) -> int:
    """2 or 3, from a case file's header (format 3: the records carry YMM upper halves; Sprint 049)."""
    raw = path.read_bytes()[:8]
    assert raw[:4] == b"W2MS"
    return int(struct.unpack_from("<I", raw, 4)[0])


def read_cases(path: Path) -> list[Case]:
    raw = path.read_bytes()
    fmt = struct.unpack_from("<I", raw, 4)[0]
    assert raw[:4] == b"W2MS" and fmt in (2, 3)
    count = struct.unpack_from("<I", raw, 8)[0]
    size = CASE_SIZE + (256 if fmt == 3 else 0)
    out = []
    for n in range(count):
        p = 12 + n * size
        code = raw[p + 8:p + 8 + raw[p]]
        q = p + 8 + CODE_BYTES
        regs = list(struct.unpack_from("<16Q", raw, q))
        rflags, mxcsr = struct.unpack_from("<QQ", raw, q + 128)
        q += 144
        xmm = [int.from_bytes(raw[q + 16 * i:q + 16 * i + 16], "little") for i in range(16)]
        q += 256
        ymm_hi = [0] * 16
        if fmt == 3:
            ymm_hi = [int.from_bytes(raw[q + 16 * i:q + 16 * i + 16], "little") for i in range(16)]
            q += 256
        out.append(Case(code, regs, rflags, raw[q:q + WINDOW], raw[q + WINDOW:q + 2 * WINDOW], "", mxcsr, xmm,
                        raw[q + 2 * WINDOW:q + 2 * WINDOW + EDGE], raw[p + 1], ymm_hi))
    return out


def read_cases_format1(path: Path) -> list[Case]:
    raw = path.read_bytes()
    assert raw[:4] == b"W2MS" and struct.unpack_from("<I", raw, 4)[0] == 1
    count = struct.unpack_from("<I", raw, 8)[0]
    out = []
    for n in range(count):
        p = 12 + n * CASE_SIZE_1
        code = raw[p + 8:p + 8 + raw[p]]
        regs = list(struct.unpack_from("<16Q", raw, p + 8 + CODE_BYTES))
        rflags = struct.unpack_from("<Q", raw, p + 8 + CODE_BYTES + 128)[0]
        q = p + 8 + CODE_BYTES + 136
        out.append(Case(code, regs, rflags, raw[q:q + WINDOW], raw[q + WINDOW:q + 2 * WINDOW], ""))
    return out


@dataclass
class Result:
    regs: list[int]
    rflags: int
    marker: int
    fault: int
    data: bytes
    stack: bytes
    fault_rip: int = 0
    mxcsr: int = fp.MXCSR_DEFAULT
    xmm: list[int] = field(default_factory=lambda: [0] * 16)
    edge: bytes = bytes(EDGE)
    ymm_hi: list[int] | None = None       # format 3: YMM bits 255:128 after the case


def read_results(path: Path, fmt: int = 2) -> list[Result]:
    raw = path.read_bytes()
    return [parse_result(raw[p:p + result_size(fmt)], fmt) for p in range(0, len(raw), result_size(fmt))]


def result_size(fmt: int) -> int:
    return RESULT_SIZE + (256 if fmt == 3 else 0)


def parse_result(rec: bytes, fmt: int = 2) -> Result:
    """One result record (format 2, or 3 with YMM upper halves after the XMM registers)."""
    regs = list(struct.unpack_from("<16Q", rec, 0))
    rflags, marker, fault, fault_rip, mxcsr = struct.unpack_from("<5Q", rec, 128)
    q = 168
    xmm = [int.from_bytes(rec[q + 16 * i:q + 16 * i + 16], "little") for i in range(16)]
    q += 256
    ymm_hi = None
    if fmt == 3:
        ymm_hi = [int.from_bytes(rec[q + 16 * i:q + 16 * i + 16], "little") for i in range(16)]
        q += 256
    return Result(regs, rflags, marker, fault, rec[q:q + WINDOW], rec[q + WINDOW:q + 2 * WINDOW],
                  fault_rip, mxcsr, xmm, rec[q + 2 * WINDOW:q + 2 * WINDOW + EDGE], ymm_hi)


def read_results_format1(path: Path) -> list[Result]:
    raw = path.read_bytes()
    out = []
    for p in range(0, len(raw), RESULT_SIZE_1):
        regs = list(struct.unpack_from("<16Q", raw, p))
        rflags, marker, fault = struct.unpack_from("<QQQ", raw, p + 128)
        out.append(Result(regs, rflags, marker, fault, raw[p + 152:p + 152 + WINDOW],
                          raw[p + 152 + WINDOW:p + 152 + 2 * WINDOW]))
    return out


@dataclass
class ModelRun:
    rt: Runtime
    fault: int                    # 0, or the Windows exception code the model's stop stands for
    why: str


def model(c: Case) -> ModelRun | str:
    """runtime.Runtime after one step of `c`; a string when the model stopped for a reason that
    is not a guest fault (an unlifted instruction, an undefined flag read)."""
    mem = Memory()
    code = mem.map(CODE_VA, 0x1000, "code")
    code.data[:len(c.code)] = c.code
    data = mem.map(DATA_VA, 0x1000, "data")
    data.data[DATA_OFF:DATA_OFF + WINDOW] = c.data
    data.data[EDGE_OFF:EDGE_OFF + EDGE] = c.edge
    stack = mem.map(STACK_VA, STACK_SIZE, "stack")
    stack.data[STACK_OFF:STACK_OFF + WINDOW] = c.stack
    rt = Runtime(Loaded(None, mem, CODE_VA, {}, STACK_VA + STACK_SIZE))   # type: ignore[arg-type]
    th = rt.th
    th.regs = list(c.regs)
    th.rip = CODE_VA
    th.flags = {f: bool(c.rflags >> b & 1) for f, b in FLAG_BITS.items()}
    th.df = bool(c.rflags >> DF_BIT & 1)
    th.mxcsr = c.mxcsr
    th.xmm = list(c.xmm)
    th.ymm_hi = list(c.ymm_hi)
    try:
        rt.step()
    except GuestFault as e:
        return ModelRun(rt, e.code, str(e))                        # ACCESS_VIOLATION unless DIV's #DE
    except fp.SimdFault as e:
        return ModelRun(rt, fp.xm_status(th.mxcsr), str(e))
    except Exception as e:                                       # the model's stop is a result
        return f"{type(e).__name__}: {e}"
    return ModelRun(rt, 0, "")


def undefined_memory(c: Case) -> str | None:
    """Why a case cannot be judged, or None. BT, BTS, BTR and BTC with a register offset address a bit
    string: the operand they touch may lie outside the windows a case defines (the data window, the
    page edge, the stack window), where the runner's page holds bytes the case never set (case set
    044: 3 such cases differed in CF). Those cases are set aside, counted, not judged."""
    from w2m.lift import REG
    d = decode(c.code)                                          # the decoded class, not the label: a committed
    if d.iclass not in ("BT", "BTS", "BTR", "BTC"):             # case file carries none (BTC: Sprint 047)
        return None
    o = operands(d.fields) if d.fields is not None and d.fields.ops else []
    if len(o) < 2 or o[0][0] != "M" or o[1][0] != "R":
        return None
    p = o[0].split(":")
    wrap = (1 << 32) - 1 if d.fields is not None and 0x67 in d.fields.prefixes else M64
    ea = int(p[5]) + (c.regs[REG[p[2]][0]] if p[2] != "INVALID" else 0)
    ea += c.regs[REG[p[3]][0]] * int(p[4]) if p[3] != "INVALID" else 0
    idx, obits, _ = REG[o[1].split(":")[1]]
    off = c.regs[idx] & ((1 << obits) - 1)
    off -= (1 << obits) if off >> (obits - 1) else 0
    size = int(p[6])
    addr = (ea + size * (off // (8 * size))) & wrap
    windows = ((DATA_VA + DATA_OFF, WINDOW), (DATA_VA + EDGE_OFF, EDGE), (STACK_VA + STACK_OFF, WINDOW))
    if any(lo <= addr and addr + size <= lo + n for lo, n in windows):
        return None
    if not any(lo <= addr < lo + 0x1000 for lo in (DATA_VA, STACK_VA)):
        return None                                   # unmapped on both: the fault is judged
    return "bit string outside the case's windows (not judged)"


def judge(c: Case, r: Result, label: str) -> list[tuple[str, str]]:
    """(class, detail) for every field where the model differs from the CPU (format 2)."""
    m = model(c)
    if isinstance(m, str):
        return [(f"{label} model stopped", m)]
    rt, th = m.rt, m.rt.th
    diffs = []
    if r.marker == 2 or m.fault:
        if r.marker != 2:
            return [(f"{label} model fault", f"model 0x{m.fault:x} ({m.why}); cpu completed")]
        if not m.fault:
            return [(f"{label} cpu fault", f"cpu exception 0x{r.fault:x}; model completed")]
        if m.fault != r.fault:
            diffs.append((f"{label} fault code", f"model 0x{m.fault:x} ({m.why}), cpu 0x{r.fault:x}"))
        if r.fault_rip != th.rip:
            diffs.append((f"{label} fault rip", f"model 0x{th.rip:x}, cpu 0x{r.fault_rip:x}"))
    else:
        paths = {CODE_VA + len(c.code): 0, CODE_VA + len(c.code) + 14: 1, RET_TARGET: 1}
        if paths.get(th.rip) != r.marker:
            diffs.append((f"{label} rip", f"model 0x{th.rip:x}, cpu path {r.marker}"))
    for i in range(16):
        if th.regs[i] != r.regs[i]:
            diffs.append((f"{label} r{i}", f"model 0x{th.regs[i]:x}, cpu 0x{r.regs[i]:x}"))
    for f, b in FLAG_BITS.items():
        mv = th.flags.get(f)
        if mv is not None and mv != bool(r.rflags >> b & 1):
            diffs.append((f"{label} {f}F", f"model {int(mv)}, cpu {r.rflags >> b & 1}"))
    if th.df != bool(r.rflags >> DF_BIT & 1):
        diffs.append((f"{label} DF", f"model {int(th.df)}, cpu {r.rflags >> DF_BIT & 1}"))
    if th.mxcsr != r.mxcsr:
        diffs.append((f"{label} MXCSR", f"model 0x{th.mxcsr:x}, cpu 0x{r.mxcsr:x} (in 0x{c.mxcsr:x})"))
    for i in range(16):
        if th.xmm[i] != r.xmm[i]:
            diffs.append((f"{label} xmm{i}", f"model {th.xmm[i]:032x}, cpu {r.xmm[i]:032x}"))
    if r.ymm_hi is not None:                                    # format 3: bits 255:128
        for i in range(16):
            if th.ymm_hi[i] != r.ymm_hi[i]:
                diffs.append((f"{label} ymm{i}.hi", f"model {th.ymm_hi[i]:032x}, cpu {r.ymm_hi[i]:032x}"))
    if rt.mem.read(DATA_VA + DATA_OFF, WINDOW) != r.data:
        diffs.append((f"{label} data", "data window differs"))
    if rt.mem.read(STACK_VA + STACK_OFF, WINDOW) != r.stack:
        diffs.append((f"{label} stack", "stack window differs"))
    if rt.mem.read(DATA_VA + EDGE_OFF, EDGE) != r.edge:
        diffs.append((f"{label} edge", "edge window differs"))
    return diffs


def judge_format1(c: Case, r: Result, label: str) -> list[tuple[str, str]]:
    """Sprint 037's comparison over its format-1 files, with AF now compared where defined."""
    if r.marker == 2:
        return [(f"{label} cpu fault", f"exception 0x{r.fault:x}")]
    m = model(c)
    if isinstance(m, str) or m.fault:
        return [(f"{label} model stopped", m if isinstance(m, str) else m.why)]
    rt, th = m.rt, m.rt.th
    diffs = []
    paths = {CODE_VA + len(c.code): 0, CODE_VA + len(c.code) + 14: 1, RET_TARGET: 1}
    if paths.get(th.rip) != r.marker:
        diffs.append((f"{label} rip", f"model 0x{th.rip:x}, cpu path {r.marker}"))
    for i in range(16):
        if th.regs[i] != r.regs[i]:
            diffs.append((f"{label} r{i}", f"model 0x{th.regs[i]:x}, cpu 0x{r.regs[i]:x}"))
    for f, b in FLAG_BITS.items():
        mv = th.flags.get(f)
        if mv is not None and mv != bool(r.rflags >> b & 1):
            diffs.append((f"{label} {f}F", f"model {int(mv)}, cpu {r.rflags >> b & 1}"))
    if rt.mem.read(DATA_VA + DATA_OFF, WINDOW) != r.data:
        diffs.append((f"{label} data", "data window differs"))
    if rt.mem.read(STACK_VA + STACK_OFF, WINDOW) != r.stack:
        diffs.append((f"{label} stack", "stack window differs"))
    return diffs


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cases", action="store_true")
    ap.add_argument("--set", default="039", choices=sorted(CASE_SETS))
    ap.add_argument("--out", type=Path)
    ap.add_argument("--judge", type=Path)
    ap.add_argument("--record", action="store_true")
    ap.add_argument("--runs", type=Path, default=ROOT / "runs")
    a = ap.parse_args(argv)
    if a.out is None:
        a.out = ROOT / "probes" / ("semantics" if a.set == "039" else f"semantics_{a.set}") / "cases.bin"
    if a.cases:
        seed, class_set, locked, control = CASE_SETS[a.set]
        cs, skipped = build_cases(seed, class_set, locked, control)
        write_cases(cs, a.out, 3 if a.set in FORMAT_3 else 2)
        labels = Counter(c.label for c in cs)
        print(f"{len(cs)} cases over {len(labels)} iclasses ({sum(c.kind for c in cs)} fault placements) -> "
              f"{a.out} ({a.out.stat().st_size} bytes); skipped {dict(skipped)}")
        return 0
    if a.judge:
        return judge_run(a)
    ap.error("--cases or --judge")
    return 2


def judge_run(a: argparse.Namespace) -> int:
    t0 = time.monotonic()
    seed, class_set, locked, control = CASE_SETS[a.set]
    built, _ = build_cases(seed, class_set, locked, control)
    stored = read_cases(a.out)
    assert [(c.code, c.regs, c.rflags, c.xmm, c.mxcsr) for c in built] == \
        [(c.code, c.regs, c.rflags, c.xmm, c.mxcsr) for c in stored], \
        "probes/semantics/cases.bin is not what --cases builds now"
    sha = "sha256:" + hashlib.sha256(a.out.read_bytes()).hexdigest()
    ran = a.judge / "cases.sha256"
    assert not ran.exists() or "sha256:" + ran.read_text().strip() == sha, \
        f"the runner ran a case file other than this one ({sha})"
    profile = (a.judge / "profile.txt").read_text().strip()
    cpu = (a.judge / "cpu.txt").read_text().strip() if (a.judge / "cpu.txt").exists() else "unknown CPU"
    result_files = sorted(a.judge.glob("result-*.bin"))
    fmt = case_format(a.out)
    runs = [read_results(p, fmt) for p in result_files]
    assert all(len(r) == len(built) for r in runs), "result count differs from case count"

    def key(x: Result) -> tuple[Any, ...]:
        return (tuple(x.regs), x.rflags, x.marker, x.fault, x.mxcsr, tuple(x.xmm), x.data, x.stack, x.edge,
                tuple(x.ymm_hi or ()))
    unstable = sum(1 for i in range(len(built)) if len({key(x[i]) for x in runs}) > 1)
    classes: Counter[str] = Counter()
    example: dict[str, str] = {}
    faults = Counter(f"0x{r.fault:x}" for r in runs[0] if r.marker == 2)
    set_aside: Counter[str] = Counter()
    for i, (c, r) in enumerate(zip(built, runs[0], strict=True)):
        why = undefined_memory(c)
        if why:
            set_aside[f"{c.label}: {why}"] += 1
            continue
        for k, detail in judge(c, r, c.label):
            classes[k] += 1
            example.setdefault(k, f"case {i}: {c.code.hex()} {detail}")
    print(f"cases {len(built)} over {len({c.label for c in built})} iclasses; runs {len(runs)} on {cpu} ({profile}); "
          f"cpu faults {dict(faults)}; cases whose CPU result differs between runs {unstable}; "
          f"classes {len(classes)}; set aside {dict(set_aside)}; {time.monotonic() - t0:.0f} s")
    for k, n in classes.most_common(60):
        print(f"  {n:6} {k}: {example[k]}")
    if a.record:
        print(json.dumps(record(a, f"{cpu}; {profile}", len(built), classes, example, [c.label for c in built]),
                         indent=1))
    return 0


def record(a: argparse.Namespace, profile: str, total: int, classes: Counter[str], example: dict[str, str],
           labels: list[str]) -> dict[str, Any]:
    """One causal differential run (Sprint 043): a unit per instruction class, its candidate the
    class's rule (src/w2m/rules, sha256); the runner's CPU is the reference. A class whose only
    differences are CPU faults on bytes the decode spec accepts routes to contract_revision."""
    from w2m import differential
    from w2m.rules import registry

    grouped = differential.by_unit(labels, classes, example)
    rules = registry()
    whole = "sha256:" + hashlib.sha256("".join(sorted(r.sha256 for r in rules.values())).encode()).hexdigest()

    def judge_unit(unit: str) -> differential.Verdict:
        lines = grouped[unit]
        rule = f"{sum(1 for x in labels if x == unit)} cases of {unit} in {a.out.name}, {SPEC}"
        if not lines:
            return differential.Verdict("conforms", rule_ref=rule)
        fault_only = all(" cpu fault:" in line for line in lines)
        return differential.Verdict(
            "discrepancy", lines, rule, "contract_revision" if fault_only else "implementation_repair",
            "the decode spec calls these bytes an instruction and this CPU faults on them: the contract lacks "
            "this CPU's rule" if fault_only else "the runner's CPU executed the instruction; the model's semantics disagree")

    units = sorted(set(labels))
    return differential.run(differential.Differential(
        name=f"semantics-{a.set}", profile=profile, purpose="conformance",
        contract_id=lambda unit: "x86.semantics", contract_version=SPEC_VERSION, relation="exact",
        implementation=f"the model's rules (src/w2m/rules), lifter spec {SPEC_VERSION}", implementation_sha256=whole,
        generator_ref="hand (agent-authored)", reference_ref=a.judge.name, target_execution_id=f"model-{a.judge.name}",
        units=units, compare=judge_unit, baseline={"cases": total, "case_set": a.set},
        unit_sha256=lambda unit: rules[unit].sha256 if unit in rules else whole), a.runs)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
