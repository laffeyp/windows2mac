# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""PSRAD (Intel SDM Vol 2, PSRAD; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PSRAD",)
SHARED = ("_sse", "_mmx")
rule = _sse.int_rule
lower = _sse.lower
