# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""XOR (Intel SDM Vol 2, XOR)."""

from w2m.rules import _int

ICLASSES = ('XOR',)
SHARED = ("_int",)
rule = _int.alu
