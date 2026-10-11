# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""ORPD (Intel SDM Vol 2, ORPD; Sprint 049)."""

from w2m.rules import _sse

ICLASSES = ("ORPD",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
