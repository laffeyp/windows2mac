"""CMP (Intel SDM Vol 2, CMP)."""

from w2m.rules import _int

ICLASSES = ('CMP',)
SHARED = ("_int",)
rule = _int.alu
