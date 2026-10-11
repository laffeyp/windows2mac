# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""MOVNTDQ (Intel SDM Vol 2, MOVNTDQ; Sprint 050, set 050)."""

from w2m.rules import _sse

ICLASSES = ("MOVNTDQ",)
SHARED = ("_sse",)
rule = _sse.memory_only_rule
lower = _sse.lower
