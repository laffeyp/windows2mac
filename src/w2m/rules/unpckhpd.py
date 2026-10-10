"""UNPCKHPD (Intel SDM Vol 2, UNPCKHPD; Sprint 047)."""

from w2m.rules import _sse

ICLASSES = ("UNPCKHPD",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
