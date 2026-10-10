"""XCHG (Intel SDM Vol 2, XCHG)."""

from w2m.rules import _int

ICLASSES = ('XCHG',)
SHARED = ("_int",)
rule = _int.xchg
