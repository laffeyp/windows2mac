# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""CPUID (Intel SDM Vol 2, CPUID)."""

from w2m.rules import _int

ICLASSES = ('CPUID',)
SHARED = ("_int",)
rule = _int.cpuid
