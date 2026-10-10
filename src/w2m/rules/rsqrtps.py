"""RSQRTPS (Intel SDM Vol 2, RSQRTPS; Sprint 046): the reference CPU's tables (src/w2m/approximate.py)."""

from w2m.rules import _sse

ICLASSES = ("RSQRTPS",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
