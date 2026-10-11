# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""FNSTCW (Intel SDM Vol 2A, FSTCW/FNSTCW; Sprint 049)."""

from w2m.rules import _int

ICLASSES = ("FNSTCW",)
SHARED = ("_int",)
rule = _int.fpu_control
