# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""MOVDQA (Intel SDM Vol 2, MOVDQA; Sprint 044)."""

from w2m.rules import _sse

ICLASSES = ("MOVDQA",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
