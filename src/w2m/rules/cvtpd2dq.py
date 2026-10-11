# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""CVTPD2DQ (Intel SDM Vol 2, CVTPD2DQ; Sprint 049)."""

from w2m.rules import _sse

ICLASSES = ("CVTPD2DQ",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
