"""PUNPCKLQDQ (Intel SDM Vol 2, PUNPCKLQDQ; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PUNPCKLQDQ",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
