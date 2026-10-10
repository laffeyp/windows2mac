# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""MOVZX (Intel SDM Vol 2, MOVZX)."""

from w2m.rules import _int

ICLASSES = ('MOVZX',)
SHARED = ("_int",)
rule = _int.move
