# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""XGETBV (Intel SDM Vol 2, XGETBV): #UD with the guest profile's OSXSAVE clear."""

from w2m.rules import _int

ICLASSES = ('XGETBV',)
SHARED = ("_int",)
rule = _int.undefined_by_profile
