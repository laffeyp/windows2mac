"""LEA (Intel SDM Vol 2, LEA)."""

from w2m.rules import _int

ICLASSES = ('LEA',)
SHARED = ("_int",)
rule = _int.move
