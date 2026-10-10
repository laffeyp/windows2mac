"""PADDD (Intel SDM Vol 2, PADDD; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PADDD",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
