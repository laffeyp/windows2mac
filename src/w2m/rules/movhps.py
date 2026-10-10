"""MOVHPS (Intel SDM Vol 2, MOVHPS; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("MOVHPS",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
