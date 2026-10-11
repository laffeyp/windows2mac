# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""CVTPD2PS (Intel SDM Vol 2, CVTPD2PS; Sprint 050, set 050)."""

from w2m.rules import _sse

ICLASSES = ("CVTPD2PS",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
