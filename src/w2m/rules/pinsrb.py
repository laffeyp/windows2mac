# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""PINSRB (Intel SDM Vol 2, PINSRB; Sprint 050, set 050)."""

from w2m.rules import _sse

ICLASSES = ("PINSRB",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
