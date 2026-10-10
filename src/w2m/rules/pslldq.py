"""PSLLDQ (Intel SDM Vol 2, PSLLDQ; Sprint 044)."""

from w2m.rules import _sse

ICLASSES = ("PSLLDQ",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
