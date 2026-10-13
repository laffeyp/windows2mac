# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""LDDQU (Intel SDM Vol 2, LDDQU: \"F2 0F F0 /r LDDQU xmm1, mem\"; Sprint 065): 16 bytes loaded, no alignment."""

from w2m.rules import _sse

ICLASSES = ("LDDQU",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
