"""PCMPGTD (Intel SDM Vol 2, PCMPGTD; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PCMPGTD",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
