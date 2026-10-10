# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""DEC (Intel SDM Vol 2, DEC)."""

from w2m.rules import _int

ICLASSES = ('DEC',)
SHARED = ("_int",)
rule = _int.inc_dec
