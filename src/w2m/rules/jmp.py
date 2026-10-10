# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""JMP (Intel SDM Vol 2, JMP)."""

from w2m.rules import _int

ICLASSES = ('JMP',)
SHARED = ("_int",)
rule = _int.jmp_call
