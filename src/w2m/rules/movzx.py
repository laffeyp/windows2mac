"""MOVZX (Intel SDM Vol 2, MOVZX)."""

from w2m.rules import _int

ICLASSES = ('MOVZX',)
SHARED = ("_int",)
rule = _int.move
