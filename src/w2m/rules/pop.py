# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""POP (Intel SDM Vol 2, POP)."""

from w2m.rules import _int

ICLASSES = ('POP',)
SHARED = ("_int",)
rule = _int.pop
