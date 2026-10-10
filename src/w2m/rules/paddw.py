"""PADDW (Intel SDM Vol 2, PADDW; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PADDW",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
