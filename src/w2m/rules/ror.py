"""ROR (Intel SDM Vol 2, RCL/RCR/ROL/ROR)."""

from w2m.rules import _int

ICLASSES = ('ROR',)
SHARED = ("_int",)
rule = _int.shift
