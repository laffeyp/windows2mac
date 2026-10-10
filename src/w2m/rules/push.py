# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""PUSH (Intel SDM Vol 2, PUSH)."""

from w2m.rules import _int

ICLASSES = ('PUSH',)
SHARED = ("_int",)
rule = _int.push
