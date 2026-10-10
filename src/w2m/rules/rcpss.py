"""RCPSS (Intel SDM Vol 2, RCPSS; Sprint 046): the reference CPU's tables (src/w2m/approximate.py)."""

from w2m.rules import _sse

ICLASSES = ("RCPSS",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
