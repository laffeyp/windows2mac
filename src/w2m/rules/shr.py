"""SHR (Intel SDM Vol 2, SAL/SAR/SHL/SHR)."""

from w2m.rules import _int

ICLASSES = ('SHR',)
SHARED = ("_int",)
rule = _int.shr
