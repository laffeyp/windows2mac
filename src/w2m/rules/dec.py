"""DEC (Intel SDM Vol 2, DEC)."""

from w2m.rules import _int

ICLASSES = ('DEC',)
SHARED = ("_int",)
rule = _int.inc_dec
