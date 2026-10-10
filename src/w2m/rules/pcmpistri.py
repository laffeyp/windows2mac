# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""PCMPISTRI (Intel SDM Vol 2B, PCMPISTRI; Vol 2A §4.1; Sprint 047): src/w2m/string_compare.py, run in the model."""

from w2m.rules import _sse

ICLASSES = ("PCMPISTRI", "PCMPISTRI64")
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
