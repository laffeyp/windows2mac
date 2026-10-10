# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""PADDQ (Intel SDM Vol 2, PADDQ; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PADDQ",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
