# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""CMP (Intel SDM Vol 2, CMP)."""

from w2m.rules import _int

ICLASSES = ('CMP',)
SHARED = ("_int",)
rule = _int.alu
