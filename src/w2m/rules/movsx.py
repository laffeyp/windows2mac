"""MOVSX (Intel SDM Vol 2, MOVSX/MOVSXD)."""

from w2m.rules import _int

ICLASSES = ('MOVSX',)
SHARED = ("_int",)
rule = _int.movsx
