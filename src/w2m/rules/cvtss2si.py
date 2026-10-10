"""CVTSS2SI (Intel SDM Vol 2, CVTSS2SI): one legacy-SSE operation, computed by x86fp in the model and by
the shared SSE lowering in the VM."""

from w2m.rules import _sse

ICLASSES = ("CVTSS2SI",)
SHARED = ("_sse",)
rule = _sse.rule
lower = _sse.lower
