# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""MUL (Intel SDM Vol 2, MUL)."""

from w2m.rules import _int

ICLASSES = ('MUL',)
SHARED = ("_int",)
rule = _int.mul
