# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""LODSB, STOSB, MOVSB, MOVSD, SCASB, SCASD, CMPSB, CMPSD without REP (Intel SDM Vol 2, each page); Sprint 050, set 050."""

from w2m.rules import _int

ICLASSES = ('LODSB', 'STOSB', 'MOVSB', 'MOVSD', 'SCASB', 'SCASD', 'CMPSB', 'CMPSD')
SHARED = ("_int",)
rule = _int.implicit
