# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""TEST (Intel SDM Vol 2, TEST)."""

from w2m.rules import _int

ICLASSES = ('TEST',)
SHARED = ("_int",)
rule = _int.alu
