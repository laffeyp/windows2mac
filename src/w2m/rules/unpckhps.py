"""UNPCKHPS (Intel SDM Vol 2, UNPCKHPS; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("UNPCKHPS",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
