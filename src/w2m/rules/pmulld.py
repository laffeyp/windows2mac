# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""PMULLD (Intel SDM Vol 2, PMULLD; Sprint 050, set 050)."""

from w2m.rules import _sse

ICLASSES = ("PMULLD",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
