# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""CLD (Intel SDM Vol 2, CLD)."""

from w2m.rules import _int

ICLASSES = ('CLD',)
SHARED = ("_int",)
rule = _int.df
