# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""PREFETCHW (Intel SDM Vol 2, PREFETCHW; Sprint 048): a hint; no access, no fault."""

from w2m.rules import _int

ICLASSES = ('PREFETCHW',)
SHARED = ("_int",)
rule = _int.nop
