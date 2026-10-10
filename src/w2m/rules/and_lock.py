# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
"""AND_LOCK: AND with the LOCK prefix (Intel SDM Vol 2, AND; Vol 3A 9.1.2.2 locked
atomic operations): the base rule's operations; atomic in the VM's lowering."""

from w2m.rules import _int

ICLASSES = ("AND_LOCK",)
SHARED = ("_int",)
rule = _int.locked(_int.alu)
