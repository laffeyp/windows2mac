"""MOVSXD (Intel SDM Vol 2, MOVSX/MOVSXD)."""

from w2m.rules import _int

ICLASSES = ('MOVSXD',)
SHARED = ("_int",)
rule = _int.move
