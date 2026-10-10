"""PUSH (Intel SDM Vol 2, PUSH)."""

from w2m.rules import _int

ICLASSES = ('PUSH',)
SHARED = ("_int",)
rule = _int.push
