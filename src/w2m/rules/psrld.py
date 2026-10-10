"""PSRLD (Intel SDM Vol 2, PSRLD; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PSRLD",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
