# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""POPCNT (Intel SDM Vol 2, POPCNT); Sprint 050, set 050."""

from w2m.rules import _int

ICLASSES = ('POPCNT',)
SHARED = ("_int",)
rule = _int.popcnt
