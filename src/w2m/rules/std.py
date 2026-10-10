# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""STD (Intel SDM Vol 2, STD)."""

from w2m.rules import _int

ICLASSES = ('STD',)
SHARED = ("_int",)
rule = _int.df
