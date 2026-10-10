# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""MOV (Intel SDM Vol 2, MOV)."""

from w2m.rules import _int

ICLASSES = ('MOV',)
SHARED = ("_int",)
rule = _int.move
