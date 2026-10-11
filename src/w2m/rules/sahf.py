# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""SAHF (Intel SDM Vol 2, SAHF); Sprint 050, set 050."""

from w2m.rules import _int

ICLASSES = ('SAHF',)
SHARED = ("_int",)
rule = _int.implicit
