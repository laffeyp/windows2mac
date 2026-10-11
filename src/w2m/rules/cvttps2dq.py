# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""CVTTPS2DQ (Intel SDM Vol 2, CVTTPS2DQ; Sprint 050, set 050)."""

from w2m.rules import _sse

ICLASSES = ("CVTTPS2DQ",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
