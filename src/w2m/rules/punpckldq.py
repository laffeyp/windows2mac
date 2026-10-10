"""PUNPCKLDQ (Intel SDM Vol 2, PUNPCKLDQ; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PUNPCKLDQ",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
