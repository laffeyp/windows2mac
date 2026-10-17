# Sprint 106 — one lock around the guest runtime

```yaml
---
id: 106
status: done
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
- **The judge** (`tests/test_runtime_lock.py`, two x86 threads on vCPUs 1 and 2, 20,000 HeapAlloc/HeapFree each): no
  foreign mark and no fault, on every run since the dispatch-slot fix below.
- **The plant** (no lock words): three runs on 2026-10-17 showed 45 and 141, 16 and 12, 14 and 7 foreign marks; two of
  them also faulted where a mark was read as a free-list pointer, and an earlier run tore a record in the service ring
  (the host's KeyError). The plant is not blind.
- **A fault found on the way:** a vCPU branched to 0 (ESR 0x86000006, x25 = 0) in 2 of about 26 runs. The host had
  written a dispatch slot with `struct.pack_into`, which zero-fills its destination before it packs (CPython
  `_struct.c`, `s_pack_internal`), and the other vCPU read the slot between the two writes. Dispatch slots and every
  live page descriptor are now one aligned 64-bit store through `memoryview.cast("Q")`, after `OSMemoryBarrier`; 0
  faults in 60 runs after.
- **A hang found by the gates:** `tests/test_class_runs.py::test_acceptance_traces_back_to_the_rule` hung for 8 h 59
  min at 0.5% CPU. vm_oracle's `on_exception` takes a guest fault and resumes at the dispatcher, abandoning
  w2m_exception's frame, which held the lock; the next case's first service spun on it (vm_monitor at 99% CPU in
  `run_vcpu`). pytest-timeout's signal method raised its 30 s timeout inside `Channel.call`, and the host waited in
  `Channel.leave` until the vCPU's 600 s ran out (instrumented reproduction, 2026-10-17). pytest's `Failed` is a
  BaseException (`_pytest/outcomes.py`) and substrate's producer task catches `Exception` (`runtime.py`), so the
  task then died with no PRODUCER_FAILED and the run's event loop waited in `select` for good. Fixed: the host frees
  the lock wherever it resumes a vCPU away from a runtime frame (on_exception, on_unimplemented, the backward cancel
  rule) and at each run's start, and the two class-run tests passed in 9.27 s; pytest's `timeout_method = "thread"`
  now ends a timed-out run with every thread's stack. Open for substrate: a BaseException in a producer leaves its
  run waiting.
- **No change at one vCPU** (9c9c2197): 456 of 458 classes (FSIN and FCOS open, as before; 67 s); 68 of 68 families;
  the batch run conforms, the log's 168 lines at 81,727,641,571 instructions, 0 divergences, 24 values Windows varies
  (`frontier-643a6283510c-1792262915456560000`). A first batch run was cut at the script's default `--max-steps` of
  10^9 (`frontier-643a6283510c-1792262881818353000`, 76 divergences from the cut); the conforming run passed 10^12.
