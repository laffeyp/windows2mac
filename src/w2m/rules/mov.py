"""MOV (Intel SDM Vol 2, MOV)."""

from w2m.rules import _int

ICLASSES = ('MOV',)
SHARED = ("_int",)
rule = _int.move
