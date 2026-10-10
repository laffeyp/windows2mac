"""CVTSS2SD (Intel SDM Vol 2, CVTSS2SD): one legacy-SSE operation, computed by x86fp in the model and by
the shared SSE lowering in the VM."""

from w2m.rules import _sse

ICLASSES = ("CVTSS2SD",)
SHARED = ("_sse",)
rule = _sse.rule
lower = _sse.lower
