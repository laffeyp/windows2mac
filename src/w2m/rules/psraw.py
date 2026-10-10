"""PSRAW (Intel SDM Vol 2, PSRAW; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PSRAW",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
