"""CPUID (Intel SDM Vol 2, CPUID)."""

from w2m.rules import _int

ICLASSES = ('CPUID',)
SHARED = ("_int",)
rule = _int.cpuid
