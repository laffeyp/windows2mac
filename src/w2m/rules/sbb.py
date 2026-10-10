"""SBB (Intel SDM Vol 2, SBB)."""

from w2m.rules import _int

ICLASSES = ('SBB',)
SHARED = ("_int",)
rule = _int.sbb
