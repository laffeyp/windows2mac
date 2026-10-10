# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""PSLLQ (Intel SDM Vol 2, PSLLQ; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PSLLQ",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
