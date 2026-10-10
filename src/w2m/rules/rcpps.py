# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""RCPPS (Intel SDM Vol 2, RCPPS; Sprint 046): the reference CPU's tables (src/w2m/approximate.py)."""

from w2m.rules import _sse

ICLASSES = ("RCPPS",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
