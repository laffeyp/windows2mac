"""PSHUFHW (Intel SDM Vol 2, PSHUFHW; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PSHUFHW",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
