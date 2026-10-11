# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""ANDN, SHLX, SHRX, RORX, MULX (Intel SDM Vol 2; BMI1 and BMI2, which the x86-64-v3 profile reports; Sprint 049)."""

from w2m.rules import Ctx, _int

ICLASSES = ("ANDN", "SHLX", "SHRX", "RORX", "MULX")
SHARED = ("_int",)


def rule(ctx: Ctx) -> None:
    {"ANDN": _int.andn, "SHLX": _int.shift_x, "SHRX": _int.shift_x, "RORX": _int.rorx, "MULX": _int.mulx}[ctx.ic](ctx)
