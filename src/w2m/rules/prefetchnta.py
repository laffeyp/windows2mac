"""PREFETCHNTA (Intel SDM Vol 2, PREFETCHh): a hint; no access, no fault."""

from w2m.rules import _int

ICLASSES = ('PREFETCHNTA',)
SHARED = ("_int",)
rule = _int.nop
