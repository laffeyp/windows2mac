"""ADD_LOCK: ADD with the LOCK prefix (Intel SDM Vol 2, ADD; Vol 3A 9.1.2.2 locked
atomic operations): the base rule's operations; atomic in the VM's lowering."""

from w2m.rules import _int

ICLASSES = ("ADD_LOCK",)
SHARED = ("_int",)
rule = _int.locked(_int.alu)
