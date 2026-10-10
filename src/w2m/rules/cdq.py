# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""CDQ (Intel SDM Vol 2, CWD/CDQ/CQO)."""

from w2m.rules import _int

ICLASSES = ('CDQ',)
SHARED = ("_int",)
rule = _int.cdq
