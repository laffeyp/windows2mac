# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""UNPCKLPD (Intel SDM Vol 2, UNPCKLPD; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("UNPCKLPD",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
