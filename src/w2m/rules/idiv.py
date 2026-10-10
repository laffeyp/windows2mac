"""IDIV (Intel SDM Vol 2, IDIV; Sprint 046)."""

from w2m.rules import _int

ICLASSES = ("IDIV",)
SHARED = ("_int",)
rule = _int.idiv
