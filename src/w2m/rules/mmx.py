# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""The classes that are MMX only (Sprint 065): EMMS, MOVDQ2Q and PSHUFW. Every MMX-register form, of these classes or of a
class with SSE forms (PADDW, MOVQ, PUNPCKLBW ...), lifts through lift.lift_mmx to one ("mmx", ...) operation the model
runs whole (src/w2m/mmx.py); the VM runs it in A64 (src/w2m/rules/_mmx.py) or as a model step (hvc #11)."""

from w2m.lift import lift_mmx
from w2m.rules import Ctx

ICLASSES = ("EMMS", "MOVDQ2Q", "PSHUFW")
SHARED = ()


def rule(ctx: Ctx) -> None:
    lift_mmx(ctx.b, ctx.ic, ctx.ops, ctx.nxt)
