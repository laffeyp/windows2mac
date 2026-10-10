# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""SBB (Intel SDM Vol 2, SBB)."""

from w2m.rules import _int

ICLASSES = ('SBB',)
SHARED = ("_int",)
rule = _int.sbb
