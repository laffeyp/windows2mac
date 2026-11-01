# Sprint 321 — A slice ends for a thread no vCPU will take

```yaml
---
id: 321
status: active
phase: 6
pass_kind: functional
---
```

## scope

The run-loop design's "A vCPU with no thread" (Sprint 318), its slice rule: `threads::slice_until` ends the running
thread's slice at its next stop whenever another unsuspended thread is READY. On several vCPUs that is wrong twice: a
thread another vCPU runs is READY, so every slice would end at once; and a thread readied while a vCPU waits is that
vCPU's to take, so the readying vCPU need not stop. The Python host's `_slice_until` (its Sprint 121; the C runtime's
`readied()`, `threads.c` 118-127) ends the slice only for ready threads beyond the waiting vCPUs.

- **The work:** `slice_until` counting the ready, unsuspended threads this vCPU may load (`free_for`), other than its
  own, and ending the slice only when they are more than the run table's `waiting`; one vCPU, `waiting` 0 and no
  thread held elsewhere, as before.
- **Not here:** the run loop's threads (4c3b2).

## prerequisites

- 320 (closed)

## context_files

- the run-loop design's "A vCPU with no thread"; `threads.rs` (`slice_until`, `free_for`, `RT_WAITING`);
  `tools/guest_runtime/threads.c` 118-127; `docs/design-several-vcpus-2026-10-17.md` 196-202 (Sprint 121)

## signal contract

### Emits

- None; the slice's end.

### Invariants

- Every run as held; one vCPU.

## artifact contract

### Files modified

- `crates/w2m_vm/src/threads.rs`; a unit test of the slice's end

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** every run as held; a unit test on owner 1 of three threads: the second held by owner 2 and
  READY, the slice not ended; one ready free thread with `waiting` 1, not ended; with `waiting` 0, ended.
- **Planted faults:** (1) the held-elsewhere rule dropped: the first case ends the slice, and the test names it; (2)
  `waiting` not subtracted: the second case ends it.

## done criteria

On several vCPUs a running thread's slice ends only for a ready thread no running or waiting vCPU will take.

## self-review checklist

- [x] Scope names one verifiable property: the slice ends only for an untaken ready thread.
- [x] context_files cite the design, the slice rule and the C runtime's half.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each changing what its judge reads.
- [x] One concept: one slice rule.

## result

- **The rule** (`threads.rs` `slice_until`, 1305): the ready, unsuspended threads other than the current one counted
  only when this vCPU may load them (`free_for`, its owner read at 1333), and the slice ended only when they are more
  than the run table's `waiting`; one vCPU, no thread held elsewhere and `waiting` 0, as before. The doc comment no
  longer says "for one vCPU".
- **The unit test** `threads::tests::a_slice_ends_only_for_a_ready_thread_no_vcpu_will_take` (1545): on owner 1, the
  second thread ready and held by owner 2, `!0`; free with `waiting` 1, `!0`; free with `waiting` 0, 0.
- **Plant 1,** restored by `cp` and checked by `cmp`: `free_for` dropped from the count; the test failed, "the slice
  ended for a thread another vCPU holds", left 0, right 18446744073709551615.
- **Plant 2,** restored likewise: `waiting` not subtracted (`ready > 0`); the test failed, "the slice ended for a
  thread the waiting vCPU will take".
- **A breach, recorded in the tail entry:** the doc comment's first rewrite went in by a `sed` whose replacement held an
  escaped newline, which wrote an empty line and dropped half a sentence; the Edit tool repaired it, the diff against
  HEAD read after.
- **The gates:** GATES-LINE

