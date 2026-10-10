# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""CQO (Intel SDM Vol 2, CWD/CDQ/CQO; Sprint 046)."""

from w2m.rules import _int

ICLASSES = ("CQO",)
SHARED = ("_int",)
rule = _int.cqo
