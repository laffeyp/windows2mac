# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""MOVBE (Intel SDM Vol 2, MOVBE); Sprint 050, set 050."""

from w2m.rules import _int

ICLASSES = ('MOVBE',)
SHARED = ("_int",)
rule = _int.movbe
