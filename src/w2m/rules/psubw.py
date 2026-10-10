# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""PSUBW (Intel SDM Vol 2, PSUBW; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PSUBW",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
