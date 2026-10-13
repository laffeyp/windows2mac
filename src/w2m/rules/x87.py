# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The x87 classes the game reaches (Intel SDM Vol 2, each class's page; Sprint 065): mono's JIT code (FLD, FSTP,
FMUL, FSQRT, FSIN, FCOS, FADD, FWAIT, FISTTP) and UnityPlayer.dll (FIADD, FISTP, FIDIVR, FLDCW, FNCLEX, FNSTSW).

Each instruction lifts to one operation the model executes whole (src/w2m/x87.py): ("x87", iclass, opcode, ModRM.reg,
ST(i) or None, the memory operand or None). The VM runs it as a model step (hvc #11): the register stack lives in
the thread's state block (specs/guest_state.json: x87_status_word with TOP, x87_tag_abridged, st0-st7), the control
word at the monitor page's FCW_OFF, as FNSTCW has kept it since Sprint 049.
"""

from w2m.lift import _operand
from w2m.rules import Ctx

ICLASSES = ("FLD", "FSTP", "FMUL", "FSQRT", "FSIN", "FCOS", "FADD", "FWAIT", "FISTTP", "FIADD", "FISTP", "FIDIVR",
            "FLDCW", "FNCLEX", "FNSTSW", "FDIVR", "FIMUL")
SHARED = ()


def rule(ctx: Ctx) -> None:
    f = ctx.f
    assert f is not None
    reg = (f.modrm >> 3 & 7) if f.modrm is not None else 0
    st, mem = None, None
    for o in ctx.ops:
        if o.startswith("ST:"):
            st = int(o[3:])
        else:
            _, mem, _ = _operand(ctx.b, o, ctx.nxt)
    ctx.b.emit("x87", ctx.ic, f.opcode, reg, st, mem)
