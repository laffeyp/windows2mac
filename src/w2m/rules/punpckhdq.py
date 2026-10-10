"""PUNPCKHDQ (Intel SDM Vol 2, PUNPCKHDQ; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PUNPCKHDQ",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
