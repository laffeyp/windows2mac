# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""SHR (Intel SDM Vol 2, SAL/SAR/SHL/SHR)."""

from w2m.rules import _int

ICLASSES = ('SHR',)
SHARED = ("_int",)
rule = _int.shr
