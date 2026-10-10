"""IMUL (Intel SDM Vol 2, IMUL)."""

from w2m.rules import _int

ICLASSES = ('IMUL',)
SHARED = ("_int",)
rule = _int.imul
