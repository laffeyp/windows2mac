"""MFENCE (Intel SDM Vol 2, MFENCE; Sprint 047)."""

from w2m.rules import _int

ICLASSES = ('MFENCE',)
SHARED = ("_int",)
rule = _int.sfence
