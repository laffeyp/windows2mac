"""MOVMSKPS (Intel SDM Vol 2, MOVMSKPS; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("MOVMSKPS",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
