# Sprint 116 — the runtime lock, one object

```yaml
---
id: 116
status: active
phase: 5
pass_kind: architecture
---
```

## scope

Step 1, the chain's third sprint (docs/design-several-vcpus.md, "The host for several vCPUs, designed twice", design
A). One concept: the guest runtime lock's host side is one object, `RuntimeLock` (vm_backend.py), with one rule for
when the host frees it and one for when it takes it.

Today the host frees the lock at four call sites (`_release_runtime_lock`: a run's start, on_exception's resume,
on_unimplemented's resume, a backward cancel inside the runtime's C) with a 64-bit store over a 32-bit word, and
`tools/runtime_lock/runtime_lock.c` (Sprint 107) is built and called nowhere. Guest code holds the lock only inside a
call into the runtime's C: every `lock_runtime` site (the bridge, the dispatcher's compile, the switch stub, the
exception entry) frees it before guest code or a hand-over (`HVC_SCHEDULE` follows `unlock_runtime`).

- **Free, one rule:** after `serve_exit`, a lock this vCPU holds goes back unless the outcome resumes it where it
  stopped (`Resume` at the exit's pc): any other resume leaves the frame that took it, which never returns. This is
  the three resume sites' rule, and it also covers a backward cancel at a bridge's words between `lock_runtime` and
  its call, which the old test (the pc inside the runtime image) missed. `reset()` frees it at a run's start.
- **Take, when host threads share it:** `enter(vcpu)` takes it with the vCPU's value (index + 1) through the C helper
  (built as build/libruntime_lock.dylib, called through ctypes, which releases the GIL) unless the vCPU holds it
  already; `leave` gives back what `enter` took. The lock is `shared` only when several host threads serve vCPUs at
  once (chain step 5); until then `enter` takes nothing. Not "whenever vcpu_count > 1": the litmus and runtime-lock
  tests drive vCPUs 1-4 from inside vCPU 0's on_dispatch, and a lock held across that hook would deadlock them.
- The word is written and read as one 32-bit access (`lock_runtime`'s SWPA and `unlock_runtime`'s STLR are W-sized).

No behaviour changes on one vCPU.

## prerequisites

- 115.

## context_files

- src/w2m/vm_backend.py (`lock_runtime`, `unlock_runtime`, `_release_runtime_lock`, `steps`, `serve_exit`, the
  switch stub, the exception entry); tools/runtime_lock/runtime_lock.c; docs/design-several-vcpus.md;
  tests/test_runtime_lock.py.

## signal contract

### Emits

- On the batch run's record, as on Sprint 115's (frontier-643a6283510c-1792295964556678000): SESSION_INIT
  (vocab_version 0.9); EXECUTION_STARTED (timeout_seconds 10800, kill_grace_seconds 60); MEASUREMENT_RECORDED 107
  times; EXECUTION_COMPLETED (exit_status killed); TRACE_INGESTED (thread_count 28); COMPARISON_COMPLETED
  (first_divergence none); SESSION_COMPLETED (outcome completed).
- Per class run (458): SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED, then CANDIDATE_ACCEPTED (456) or, for
  FCOS and FSIN, REDUCTION_COMPLETED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED (8 each), SESSION_COMPLETED. Per
  family run (68): SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED, SESSION_COMPLETED.

### Invariants

- Vocabulary 0.9 unchanged.
- `grep -n '_release_runtime_lock\|RUNTIME_LOCK) // 8' src/w2m/vm_backend.py` finds nothing: the word is the lock
  object's alone.

## artifact contract

- `RuntimeLock` in vm_backend.py; `tests/test_runtime_lock.py`'s new test; `scripts/gates.sh` exit 0; CI `success`.

## observation contract

- **Judge:** the batch run conforms (168 lines, 0 divergences, the same eight exit kinds); 456 of 458 classes (the
  class runs' faults are the on_exception resumes: Sprint 106's 8 h 59 min hang came from a lock left held there),
  68 of 68 families; the litmus and runtime-lock tests on vCPUs 1-4. New: two host threads, each with a `Vcpu` of
  its own (indexes 1 and 2), take a shared lock through `enter` and `leave` 2,000 times each around a read, a yield
  and a write of one shared word; the word ends at 4,000.
- **Plant:** the same test with `enter` taking nothing loses increments (the word ends below 4,000).

## done criteria

The host frees and takes the runtime lock through one object by one rule each; behaviour unchanged on one vCPU; CI
green.
