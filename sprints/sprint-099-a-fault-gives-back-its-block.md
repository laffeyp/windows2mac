# Sprint 099 — a fault gives back the instructions it did not run

```yaml
---
id: 099
status: active
phase: 5
pass_kind: functional
---
```

## scope

The second of the five entries deferred that should not have been (user decision 2026-10-16). A block's entry counts
every instruction in it (SUBS x23). A guest fault leaves the block at the faulting instruction through the runtime's
exception path (vm_backend.py `_write_exception_entry`, then `w2m_exception`), and nothing gave back the faulting
instruction or those after it: the count, which is the instruction clock's time and the step limit, ran ahead by them
at every fault. The code-write exit (hvc 13, 13,673 a batch run) is not affected: it returns by ERET to the same
store in the same block, whose entry is not run again (read in the code, 2026-10-16).

This card was written after the change and its test, in the same session, before any record or commit: the order is
noted here rather than hidden.

**Known first.**

- A fault is reported before its instruction executes; the saved return address points to the faulting instruction
  (Intel SDM Vol 3A 6.5, Exception Classifications). So the faulting instruction is not counted, as an instruction
  counter of retired instructions does not count it.
- The host's tail table (`tail_hosts`, `tail_left`) holds each compiled instruction's host address and the count after
  it, for host and VM compiles alike (`_vm_compiled` notes the VM's at every exit, first).

## prerequisites

- 098.

## context_files

- src/w2m/vm_backend.py (`_write_exception_entry`, the exit loop's count from x23, `_unrun_after`);
  tools/guest_runtime/runtime.c (`w2m_exception`); tests/test_instruction_count.py.

## signal contract

### Emits

- None new; the batch run prints the number of faults given back.

### Invariants

- Vocabulary unchanged. The SIMD exception the host raises (`_raise_in_guest`) enters after the new exit and is
  unchanged: its model step already gave back its block's tail.

## artifact contract

- The exception entry's hardware-fault path exits with hvc 14 (`HVC_FAULT_GIVE_BACK`, x2 = ELR); the host adds
  `_unrun_after(ELR) + 1` to x23 when ELR is in the code cache. Tests; `scripts/gates.sh` exit 0; CI `success`.

## observation contract

- **Judge:** the instruction-count tests end each program with a faulting store, three NOPs and a JMP in one block;
  each count equals the instructions before the store exactly; a new test counts 2 for two ADDs before the fault.
- **Plant:** the give-back removed: every such count must be five high.
- Every family accepts; the batch run conforms (168 of 168 lines, 0 divergences).

## done criteria

The instruction count at a guest fault is the instructions retired before it; every judge holds; CI green.

## observed
