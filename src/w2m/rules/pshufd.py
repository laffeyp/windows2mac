# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""PSHUFD (Intel SDM Vol 2, PSHUFD; Sprint 044)."""

from w2m.rules import _sse

ICLASSES = ("PSHUFD",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
