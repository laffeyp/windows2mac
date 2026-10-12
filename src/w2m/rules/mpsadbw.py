# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""MPSADBW (Intel SDM Vol 2, MPSADBW: \"66 0F 3A 42 /r ib MPSADBW xmm1, xmm2/m128, imm8\"; Sprint 065)."""

from w2m.rules import _sse

ICLASSES = ("MPSADBW",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
