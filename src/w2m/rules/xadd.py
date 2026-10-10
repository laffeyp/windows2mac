# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""XADD (Intel SDM Vol 2, XADD)."""

from w2m.rules import _int

ICLASSES = ('XADD',)
SHARED = ("_int",)
rule = _int.xadd
