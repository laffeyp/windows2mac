"""CDQE (Intel SDM Vol 2, CBW/CWDE/CDQE)."""

from w2m.rules import _int

ICLASSES = ('CDQE',)
SHARED = ("_int",)
rule = _int.cdqe
