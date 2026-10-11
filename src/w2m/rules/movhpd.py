# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""MOVHPD (Intel SDM Vol 2, MOVHPD; Sprint 050, set 050)."""

from w2m.rules import _sse

ICLASSES = ("MOVHPD",)
SHARED = ("_sse",)
rule = _sse.memory_only_rule
lower = _sse.lower
