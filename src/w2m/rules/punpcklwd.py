"""PUNPCKLWD (Intel SDM Vol 2, PUNPCKLWD; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PUNPCKLWD",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
