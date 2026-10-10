"""PANDN (Intel SDM Vol 2, PANDN; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PANDN",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
