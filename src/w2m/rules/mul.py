"""MUL (Intel SDM Vol 2, MUL)."""

from w2m.rules import _int

ICLASSES = ('MUL',)
SHARED = ("_int",)
rule = _int.mul
