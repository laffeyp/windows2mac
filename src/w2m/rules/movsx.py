# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""MOVSX (Intel SDM Vol 2, MOVSX/MOVSXD)."""

from w2m.rules import _int

ICLASSES = ('MOVSX',)
SHARED = ("_int",)
rule = _int.movsx
