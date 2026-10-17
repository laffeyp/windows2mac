# Sprint 106 — one lock around the guest runtime

```yaml
---
id: 106
status: active
phase: 5
pass_kind: functional
---
```

## scope

The first sprint of "The runtime under several vCPUs" (docs/design-several-vcpus.md; Decision 2026-10-17, one lock
first). A spinlock word on the run table's header page (RUN_TABLE + 0x80), taken with SWPA and released with STLR
at every entry from guest code into the runtime's C:

- each service bridge, around its call;
- the exception entry, around w2m_exception;
- the switch stub, from before its first store to the run table to its last load;
- the dispatcher's miss stub, from before its save to the compiler's shared page to after its restore.

The lock is never held while guest code runs: runtime.h's invariant ("No runtime frame is live while guest code
runs": a service that calls guest code sets the next address and returns), so a guest callback's own service calls
cannot deadlock on it. The helpers a block calls directly (CPUID, PCMPISTRI, RCPPS's approximation, x87_fast.c) touch
only the vCPU's own pages and take no lock. The run table's RUNNING mark, a current thread per vCPU and the host
serving every vCPU's channel are Sprint 107's.

## prerequisites

- 105.

## context_files

- docs/design-several-vcpus.md; src/w2m/vm_backend.py (`_bridge`, `_switch_stub`, `_write_exception_entry`, the
  dispatcher); tools/guest_runtime/runtime.h (the no-live-frame invariant), heap.c; tests/test_litmus.py's harness.

## signal contract

### Emits

- None new.

### Invariants

- Vocabulary unchanged; with one vCPU every run behaves as before.

## artifact contract

- The lock words (encodings checked against clang's assembler) and their four places; `tests/test_runtime_lock.py`
  (hvf); `scripts/gates.sh` exit 0; CI `success` on the closing commit.

## observation contract

- **Judge:** two x86 threads on two vCPUs, each 20,000 times: HeapAlloc 48 bytes, write its own mark at both ends,
  spin, check both marks, HeapFree; no mark is ever another thread's, and no run faults.
- **Plant:** the lock's words left out: the same program must show a foreign mark, a fault, or a corrupt free list
  within its iterations; if it shows none, the record says the plant was blind.
- **No change at one vCPU:** 68 of 68 families; the batch run conforms (168 of 168 lines, 0 divergences).

## done criteria

Guest code on several vCPUs calls the runtime one vCPU at a time; CI green.

## observed
