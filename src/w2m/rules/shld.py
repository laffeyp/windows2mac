# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""SHLD (Intel SDM Vol 2, SHLD); Sprint 050, set 050."""

from w2m.rules import _int

ICLASSES = ('SHLD',)
SHARED = ("_int",)
rule = _int.double_shift
