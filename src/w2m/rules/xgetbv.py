# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""XGETBV (Intel SDM Vol 2, XGETBV): XCR0 = 7 with the guest profile's OSXSAVE set (Sprint 049)."""

from w2m.rules import _int

ICLASSES = ('XGETBV',)
SHARED = ("_int",)
rule = _int.xgetbv
