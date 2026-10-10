# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""ADD (Intel SDM Vol 2, ADD)."""

from w2m.rules import _int

ICLASSES = ('ADD',)
SHARED = ("_int",)
rule = _int.alu
