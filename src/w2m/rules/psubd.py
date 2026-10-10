"""PSUBD (Intel SDM Vol 2, PSUBD; Sprint 044)."""

from w2m.rules import _sse

ICLASSES = ("PSUBD",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
