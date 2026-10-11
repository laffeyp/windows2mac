# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""PEXTRW (Intel SDM Vol 2, PEXTRW; Sprint 050, set 050)."""

from w2m.rules import _sse

ICLASSES = ("PEXTRW",)
SHARED = ("_sse",)
rule = _sse.register_only_rule
lower = _sse.lower
