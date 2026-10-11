# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""POPFQ (Intel SDM Vol 2, POPF/POPFD/POPFQ); Sprint 050, set 050."""

from w2m.rules import _int

ICLASSES = ('POPFQ',)
SHARED = ("_int",)
rule = _int.implicit
