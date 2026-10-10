"""PUNPCKHBW (Intel SDM Vol 2, PUNPCKHBW; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PUNPCKHBW",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
