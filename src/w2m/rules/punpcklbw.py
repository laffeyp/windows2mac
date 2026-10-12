# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""PUNPCKLBW (Intel SDM Vol 2, PUNPCKLBW; Sprint 044)."""

from w2m.rules import _sse

ICLASSES = ("PUNPCKLBW",)
SHARED = ("_sse", "_mmx")
rule = _sse.int_rule
lower = _sse.lower
