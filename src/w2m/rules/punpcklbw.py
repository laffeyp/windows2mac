"""PUNPCKLBW (Intel SDM Vol 2, PUNPCKLBW; Sprint 044)."""

from w2m.rules import _sse

ICLASSES = ("PUNPCKLBW",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
