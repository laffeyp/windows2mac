# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""CDQE (Intel SDM Vol 2, CBW/CWDE/CDQE)."""

from w2m.rules import _int

ICLASSES = ('CDQE',)
SHARED = ("_int",)
rule = _int.cdqe
