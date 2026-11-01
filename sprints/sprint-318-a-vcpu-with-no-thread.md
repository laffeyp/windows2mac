# Sprint 318 — A vCPU with no thread

```yaml
---
id: 318
status: active
phase: 6
pass_kind: functional
---
```

## scope

The run-loop design's card 4c3b (`docs/design-run-loop-on-several-vcpus-2026-10-31.md`, card 4) runs a second vCPU's
thread, and its "The threads" section gives a parked vCPU one line: it "waits on the `sched` condition ... a thread
readied on another vCPU wakes it". No host code parks, waits or wakes: `THREAD_WAKE` is answered 0 and nothing more
(`host_request.rs` 665), and `threads::switch` calls a state with no ready thread and no timed waiter a deadlock, which
on two vCPUs it is not while the other vCPU runs a thread. The Python host solved it at Sprints 118 to 121
(`docs/design-several-vcpus-2026-10-17.md` 160-200: the park, `_wait_for_change` with the earliest deadline as its
timeout, the notify, `RT_WAITING` and `W2M_THREAD_WAKE`, `_slice_until` past the waiting vCPUs, the deadlock only when
every vCPU waits a whole second, and vCPU 0 driving the Machine), and the C runtime keeps its half (`threads.c` 51-52,
115-126). Building before the form is decided would repeat 309's first draft.

- **The work, a design amendment** (the run-loop design, "The threads"): each mechanism of Sprints 118 to 121, where
  it sits in the Rust run loop and what the C runtime already does for it, read from `threads.c` and `threads.rs`; the
  park and its wait (the lock and the word let go, the timeout the earliest deadline), the wake (`THREAD_WAKE` and every
  host service that readies a thread), the load onto a vCPU with no thread, the deadlock rule across vCPUs, and the
  Machine's driver; and card 4c3b split into the cards they make. A BLACKBOARD Decision. No code.
- **Not here:** any build.

## prerequisites

- 317 (closed)

## context_files

- the run-loop design's "The threads" and card 4; `docs/design-several-vcpus-2026-10-17.md` 160-200;
  `tools/guest_runtime/threads.c` (the run table, `readied`, `held_elsewhere`, the in-VM switch); `threads.rs`
  (`switch`, `timed_waiter`, `slice_until`); `host_request.rs` (`THREAD_WAKE`); `turns.rs` (the Machine)

## signal contract

### Emits

- None; a design.

### Invariants

- No code changes.

## artifact contract

### Files modified

- `docs/design-run-loop-on-several-vcpus-2026-10-31.md`; BLACKBOARD's Decision at the close

### Command exit codes

- `scripts/gates.sh` returns 0; `design_check` passes

## observation contract

- **Expected values:** each of Sprints 118 to 121's mechanisms with its Rust place and the C runtime's part, by file
  and line; the cards card 4c3b becomes; `design_check` passing; the record check over the Decision.
- **Planted faults:** (1) the Decision's revisit clause struck: `signal_report --check` names the Decision.

## done criteria

How a vCPU with no thread waits, wakes and takes one, and when a run of several vCPUs is deadlocked, is decided from
the Python host's built form and the code as it is, before any second thread runs.

## self-review checklist

- [x] Scope names one verifiable property: the no-thread vCPU's form, decided from the built prior form.
- [x] context_files cite the design, the prior design, the C runtime and the host.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and a plant changing what its judge reads.
- [x] One concept: one design decision.

## result

