# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""BT, BTS, BTR, BTC (Intel SDM Vol 2, BT, BTS, BTR, BTC; BTC Sprint 047)."""

from w2m.rules import _int

ICLASSES = ('BT', 'BTS', 'BTR', 'BTC')
SHARED = ("_int",)
rule = _int.bit_test
