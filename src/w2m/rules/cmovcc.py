# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""CMOVO ... CMOVNLE (16 iclasses) (Intel SDM Vol 2, CMOVcc)."""

from w2m.rules import _int

ICLASSES = ('CMOVO', 'CMOVNO', 'CMOVB', 'CMOVNB', 'CMOVZ', 'CMOVNZ', 'CMOVBE', 'CMOVNBE', 'CMOVS', 'CMOVNS', 'CMOVP', 'CMOVNP', 'CMOVL', 'CMOVNL', 'CMOVLE', 'CMOVNLE')
SHARED = ("_int",)
rule = _int.cmov
