# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""PREFETCHT2 (Intel SDM Vol 2, PREFETCHh; Sprint 048): a hint; no access, no fault."""

from w2m.rules import _int

ICLASSES = ('PREFETCHT2',)
SHARED = ("_int",)
rule = _int.nop
