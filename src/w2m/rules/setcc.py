"""SETO ... SETNLE (16 iclasses) (Intel SDM Vol 2, SETcc)."""

from w2m.rules import _int

ICLASSES = ('SETO', 'SETNO', 'SETB', 'SETNB', 'SETZ', 'SETNZ', 'SETBE', 'SETNBE', 'SETS', 'SETNS', 'SETP', 'SETNP', 'SETL', 'SETNL', 'SETLE', 'SETNLE')
SHARED = ("_int",)
rule = _int.setcc
