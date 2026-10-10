"""MOVLHPS (Intel SDM Vol 2, MOVLHPS; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("MOVLHPS",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
