"""SHUFPS (Intel SDM Vol 2, SHUFPS; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("SHUFPS",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
