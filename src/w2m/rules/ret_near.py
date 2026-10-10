# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""RET_NEAR (Intel SDM Vol 2, RET)."""

from w2m.rules import _int

ICLASSES = ('RET_NEAR',)
SHARED = ("_int",)
rule = _int.ret_near
