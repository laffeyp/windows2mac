# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""MOVMSKPS (Intel SDM Vol 2, MOVMSKPS; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("MOVMSKPS",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
