# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""SHL (Intel SDM Vol 2, SAL/SAR/SHL/SHR)."""

from w2m.rules import _int

ICLASSES = ('SHL',)
SHARED = ("_int",)
rule = _int.shift
