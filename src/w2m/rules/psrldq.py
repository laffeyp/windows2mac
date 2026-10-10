# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""PSRLDQ (Intel SDM Vol 2, PSRLDQ; Sprint 044)."""

from w2m.rules import _sse

ICLASSES = ("PSRLDQ",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
