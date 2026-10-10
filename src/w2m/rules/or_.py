# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""OR (Intel SDM Vol 2, OR)."""

from w2m.rules import _int

ICLASSES = ('OR',)
SHARED = ("_int",)
rule = _int.alu
