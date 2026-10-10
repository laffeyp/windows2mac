# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""PSUBSB (Intel SDM Vol 2, PSUBSB; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PSUBSB",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
