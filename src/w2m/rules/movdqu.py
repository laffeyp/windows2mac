"""MOVDQU (Intel SDM Vol 2, MOVDQU; Sprint 044)."""

from w2m.rules import _sse

ICLASSES = ("MOVDQU",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
