# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""PAUSE (Intel SDM Vol 2, PAUSE): a spin-loop hint, no architectural effect; Sprint 050, set 050."""

from w2m.rules import _int

ICLASSES = ('PAUSE',)
SHARED = ("_int",)
rule = _int.pause
