# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""INC (Intel SDM Vol 2, INC)."""

from w2m.rules import _int

ICLASSES = ('INC',)
SHARED = ("_int",)
rule = _int.inc_dec
