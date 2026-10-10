# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""NEG (Intel SDM Vol 2, NEG)."""

from w2m.rules import _int

ICLASSES = ("NEG",)
SHARED = ("_int",)
rule = _int.neg
