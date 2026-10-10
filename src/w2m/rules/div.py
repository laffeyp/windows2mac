# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""DIV (Intel SDM Vol 2, DIV)."""

from w2m.rules import _int

ICLASSES = ('DIV',)
SHARED = ("_int",)
rule = _int.div
