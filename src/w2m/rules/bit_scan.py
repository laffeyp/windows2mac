"""BSF, BSR (Intel SDM Vol 2, BSF, BSR)."""

from w2m.rules import _int

ICLASSES = ('BSF', 'BSR')
SHARED = ("_int",)
rule = _int.bit_scan
