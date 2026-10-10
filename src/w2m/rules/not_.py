# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""NOT (Intel SDM Vol 2, NOT)."""

from w2m.rules import _int

ICLASSES = ('NOT',)
SHARED = ("_int",)
rule = _int.not_
