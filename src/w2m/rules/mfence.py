# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""MFENCE (Intel SDM Vol 2, MFENCE; Sprint 047)."""

from w2m.rules import _int

ICLASSES = ('MFENCE',)
SHARED = ("_int",)
rule = _int.sfence
