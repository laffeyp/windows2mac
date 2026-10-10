# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""IDIV (Intel SDM Vol 2, IDIV; Sprint 046)."""

from w2m.rules import _int

ICLASSES = ("IDIV",)
SHARED = ("_int",)
rule = _int.idiv
