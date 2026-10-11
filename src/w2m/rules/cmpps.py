# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""CMPPS (Intel SDM Vol 2, CMPPS; Sprint 050, set 050)."""

from w2m.rules import _sse

ICLASSES = ("CMPPS",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
