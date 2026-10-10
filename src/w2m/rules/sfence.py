"""SFENCE (Intel SDM Vol 2, SFENCE)."""

from w2m.rules import _int

ICLASSES = ('SFENCE',)
SHARED = ("_int",)
rule = _int.sfence
