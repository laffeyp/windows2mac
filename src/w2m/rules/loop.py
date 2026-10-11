# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""LOOP, LOOPE, LOOPNE (Intel SDM Vol 2, LOOP/LOOPcc) and JRCXZ (Jcc); Sprint 050, set 050."""

from w2m.rules import _int

ICLASSES = ('LOOP', 'LOOPE', 'LOOPNE', 'JRCXZ')
SHARED = ("_int",)
rule = _int.loop
