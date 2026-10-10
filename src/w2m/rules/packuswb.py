"""PACKUSWB (Intel SDM Vol 2, PACKUSWB; Sprint 046)."""

from w2m.rules import _sse

ICLASSES = ("PACKUSWB",)
SHARED = ("_sse",)
rule = _sse.int_rule
lower = _sse.lower
