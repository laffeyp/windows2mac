# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""CWDE (Intel SDM Vol 2, CBW/CWDE/CDQE; Sprint 046)."""

from w2m.rules import _int

ICLASSES = ("CWDE",)
SHARED = ("_int",)
rule = _int.cwde
