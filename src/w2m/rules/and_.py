"""AND (Intel SDM Vol 2, AND)."""

from w2m.rules import _int

ICLASSES = ('AND',)
SHARED = ("_int",)
rule = _int.alu
