"""PSRLW (Intel SDM Vol 2, PSRLW; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PSRLW",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
