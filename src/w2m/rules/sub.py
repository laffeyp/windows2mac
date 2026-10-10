# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""SUB (Intel SDM Vol 2, SUB)."""

from w2m.rules import _int

ICLASSES = ('SUB',)
SHARED = ("_int",)
rule = _int.alu
