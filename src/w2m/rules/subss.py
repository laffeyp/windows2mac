"""SUBSS (Intel SDM Vol 2, SUBSS): one legacy-SSE operation, computed by x86fp in the model and by
the shared SSE lowering in the VM."""

from w2m.rules import _sse

ICLASSES = ("SUBSS",)
SHARED = ("_sse",)
rule = _sse.rule
lower = _sse.lower
