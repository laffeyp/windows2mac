# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""PCMPEQD (Intel SDM Vol 2, PCMPEQD; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PCMPEQD",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
