"""CALL_NEAR (Intel SDM Vol 2, CALL)."""

from w2m.rules import _int

ICLASSES = ('CALL_NEAR',)
SHARED = ("_int",)
rule = _int.jmp_call
