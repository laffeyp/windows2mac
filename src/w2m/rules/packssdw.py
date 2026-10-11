# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""PACKSSDW (Intel SDM Vol 2, PACKSSDW; Sprint 050, set 050)."""

from w2m.rules import _sse

ICLASSES = ("PACKSSDW",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
