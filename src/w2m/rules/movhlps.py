"""MOVHLPS (Intel SDM Vol 2, MOVHLPS; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("MOVHLPS",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
