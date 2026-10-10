"""PMOVMSKB (Intel SDM Vol 2, PMOVMSKB; Sprint 044)."""

from w2m.rules import _sse

ICLASSES = ("PMOVMSKB",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
