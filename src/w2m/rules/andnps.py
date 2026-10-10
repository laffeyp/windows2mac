"""ANDNPS (Intel SDM Vol 2, ANDNPS; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("ANDNPS",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
