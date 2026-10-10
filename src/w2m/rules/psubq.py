"""PSUBQ (Intel SDM Vol 2, PSUBQ; Sprint 044)."""

from w2m.rules import _sse

ICLASSES = ("PSUBQ",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
