# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""SFENCE (Intel SDM Vol 2, SFENCE)."""

from w2m.rules import _int

ICLASSES = ('SFENCE',)
SHARED = ("_int",)
rule = _int.sfence
