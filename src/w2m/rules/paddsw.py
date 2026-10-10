"""PADDSW (Intel SDM Vol 2, PADDSW; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PADDSW",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
