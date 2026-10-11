# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""CMPSD_XMM (Intel SDM Vol 2, CMPSD_XMM; Sprint 049)."""

from w2m.rules import _sse

ICLASSES = ("CMPSD_XMM",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
