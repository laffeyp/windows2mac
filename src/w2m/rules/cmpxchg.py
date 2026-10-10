"""CMPXCHG (Intel SDM Vol 2, CMPXCHG)."""

from w2m.rules import _int

ICLASSES = ('CMPXCHG',)
SHARED = ("_int",)
rule = _int.cmpxchg
