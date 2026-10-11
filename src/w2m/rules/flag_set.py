# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""STC, CLC, CMC (Intel SDM Vol 2, each page); Sprint 050, set 050."""

from w2m.rules import _int

ICLASSES = ('STC', 'CLC', 'CMC')
SHARED = ("_int",)
rule = _int.implicit
