"""PAND (Intel SDM Vol 2, PAND; Sprint 044)."""

from w2m.rules import _sse

ICLASSES = ("PAND",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
