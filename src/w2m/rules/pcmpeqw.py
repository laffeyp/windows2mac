# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""PCMPEQW (Intel SDM Vol 2, PCMPEQW; Sprint 049)."""

from w2m.rules import _sse

ICLASSES = ("PCMPEQW",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
