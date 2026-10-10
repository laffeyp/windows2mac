# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""LFENCE (Intel SDM Vol 2, LFENCE; Sprint 047)."""

from w2m.rules import _int

ICLASSES = ('LFENCE',)
SHARED = ("_int",)
rule = _int.sfence
