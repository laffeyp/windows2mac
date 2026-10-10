# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""CMPXCHG (Intel SDM Vol 2, CMPXCHG)."""

from w2m.rules import _int

ICLASSES = ('CMPXCHG',)
SHARED = ("_int",)
rule = _int.cmpxchg
