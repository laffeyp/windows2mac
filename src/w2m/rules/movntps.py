# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""MOVNTPS (Intel SDM Vol 2, MOVNTPS; Sprint 044)."""

from w2m.rules import _sse

ICLASSES = ("MOVNTPS",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
