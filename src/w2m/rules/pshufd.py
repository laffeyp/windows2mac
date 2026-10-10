"""PSHUFD (Intel SDM Vol 2, PSHUFD; Sprint 044)."""

from w2m.rules import _sse

ICLASSES = ("PSHUFD",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
