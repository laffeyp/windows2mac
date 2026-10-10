"""PSRAD (Intel SDM Vol 2, PSRAD; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PSRAD",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
