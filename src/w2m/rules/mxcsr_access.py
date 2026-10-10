# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""STMXCSR, LDMXCSR (Intel SDM Vol 2)."""

from w2m.rules import _int

ICLASSES = ('STMXCSR', 'LDMXCSR')
SHARED = ("_int",)
rule = _int.mxcsr
