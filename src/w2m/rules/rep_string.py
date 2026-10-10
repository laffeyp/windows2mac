"""REP STOS and REP MOVS (Intel SDM Vol 2, STOS, MOVS, REP; Vol 1 7.3.9)."""

from w2m.lift import REP_STRING
from w2m.rules import _int

ICLASSES = tuple(REP_STRING)
SHARED = ("_int",)
rule = _int.rep_string
