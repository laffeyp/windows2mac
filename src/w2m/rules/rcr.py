# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""RCR (Intel SDM Vol 2, RCL/RCR/ROL/ROR); Sprint 050, set 050."""

from w2m.rules import _int

ICLASSES = ('RCR',)
SHARED = ("_int",)
rule = _int.rcr
