# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""UCOMISD (Intel SDM Vol 2, UCOMISD): one legacy-SSE operation, computed by x86fp in the model and by
the shared SSE lowering in the VM."""

from w2m.rules import _sse

ICLASSES = ("UCOMISD",)
SHARED = ("_sse",)
rule = _sse.rule
lower = _sse.lower
