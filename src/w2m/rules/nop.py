"""NOP (Intel SDM Vol 2, NOP)."""

from w2m.rules import _int

ICLASSES = ('NOP',)
SHARED = ("_int",)
rule = _int.nop
