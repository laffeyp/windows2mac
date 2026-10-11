# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""CVTDQ2PS (Intel SDM Vol 2, CVTDQ2PS; Sprint 050, set 050)."""

from w2m.rules import _sse

ICLASSES = ("CVTDQ2PS",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
