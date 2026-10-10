"""POR (Intel SDM Vol 2, POR; Sprint 044)."""

from w2m.rules import _sse

ICLASSES = ("POR",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
