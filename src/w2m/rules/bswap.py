"""BSWAP (Intel SDM Vol 2, BSWAP)."""

from w2m.rules import _int

ICLASSES = ('BSWAP',)
SHARED = ("_int",)
rule = _int.bswap
