"""PMAXUB (Intel SDM Vol 2, PMAXUB; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PMAXUB",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
