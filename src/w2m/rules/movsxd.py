# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""MOVSXD (Intel SDM Vol 2, MOVSX/MOVSXD)."""

from w2m.rules import _int

ICLASSES = ('MOVSXD',)
SHARED = ("_int",)
rule = _int.move
