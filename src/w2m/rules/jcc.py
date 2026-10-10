"""JO ... JNLE (16 iclasses) (Intel SDM Vol 2, Jcc)."""

from w2m.rules import _int

ICLASSES = ('JO', 'JNO', 'JB', 'JNB', 'JZ', 'JNZ', 'JBE', 'JNBE', 'JS', 'JNS', 'JP', 'JNP', 'JL', 'JNL', 'JLE', 'JNLE')
SHARED = ("_int",)
rule = _int.jcc
