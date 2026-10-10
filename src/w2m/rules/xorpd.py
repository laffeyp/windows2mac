"""XORPD (Intel SDM Vol 2, XORPD): one legacy-SSE operation, computed by x86fp in the model and by
the shared SSE lowering in the VM."""

from w2m.rules import _sse

ICLASSES = ("XORPD",)
SHARED = ("_sse",)
rule = _sse.rule
lower = _sse.lower
