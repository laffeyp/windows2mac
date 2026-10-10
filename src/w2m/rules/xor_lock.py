"""XOR_LOCK: XOR with the LOCK prefix (Intel SDM Vol 2, XOR; Vol 3A 9.1.2.2 locked
atomic operations): the base rule's operations; atomic in the VM's lowering."""

from w2m.rules import _int

ICLASSES = ("XOR_LOCK",)
SHARED = ("_int",)
rule = _int.locked(_int.alu)
