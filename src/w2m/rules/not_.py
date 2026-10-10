"""NOT (Intel SDM Vol 2, NOT)."""

from w2m.rules import _int

ICLASSES = ('NOT',)
SHARED = ("_int",)
rule = _int.not_
