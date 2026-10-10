"""CMPXCHG16B (Intel SDM Vol 2, CMPXCHG8B/CMPXCHG16B; Sprint 046): one atomic op in the VM either way."""

from w2m.rules import _int

ICLASSES = ("CMPXCHG16B",)
SHARED = ("_int",)
rule = _int.cmpxchg16b
