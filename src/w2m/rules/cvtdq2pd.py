# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""CVTDQ2PD (Intel SDM Vol 2, CVTDQ2PD; Sprint 044)."""

from w2m.rules import _sse

ICLASSES = ("CVTDQ2PD",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
