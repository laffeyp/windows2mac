"""BT, BTS, BTR (Intel SDM Vol 2, BT, BTS, BTR)."""

from w2m.rules import _int

ICLASSES = ('BT', 'BTS', 'BTR')
SHARED = ("_int",)
rule = _int.bit_test
