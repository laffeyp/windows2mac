# Sprint 330 — The kick

```yaml
---
id: 330
status: active
phase: 6
pass_kind: functional
---
```

## scope

The run-loop design's card 4c3b3, "The kick" (the Python host's Sprint 119): SuspendThread of a thread another vCPU
holds stops that vCPU (`VcpuHandle::stop`) and waits, the word and the guard let go, until the thread's `vcpu` word
clears; the vCPU stopped leaves through the host, its `SLICE_UNTIL` 0 and the C's switch declining the suspended
thread. Today `threads::suspend` (`threads.rs`) raises the count alone, and the thread runs on to its vCPU's next
switch. Measured before this card, the threads family probe (`store/family_runs/known-2026-10-29.tsv`'s `threads`
row, through `w2m-vm` with its arguments) on one vCPU and on two: the spinner, on vCPU 1 while the main thread loops
on SwitchToThread, kept counting after SuspendThread, `suspend.frozen`'s `count_unchanged` 0 where one vCPU reads 1,
and GetThreadContext read it with `rip_in_image` 0 where one vCPU reads 1; every other row the same.

- **The work:** `suspend` of a thread whose run entry's `vcpu` word names another vCPU stops that vCPU through its
  handle and waits, holding neither the word nor the lock, until the word clears; the stopped vCPU, at its cancel
  with the run not stopped, leaves through the host to a switch, which puts the suspended thread down; the run's
  vCPU handles reachable from a host request (`RunStop`'s, by owner). A cancel that is not the run's stop is no
  longer read as the time limit. Card 4c3b3's run of guests on two vCPUs also takes Deferred (Sprint 329):
  `model_steps`, `mutual`, `mutual_context`, `qpc2` and `readied` join `through_w2m_vm`, each held to
  `store/batch_run/first-stop-2026-10-28.tsv`.
- **Not here:** every other vCPU stopped for a locked model step or a split (4d); the Machine's own thread (4f).

## prerequisites

- 329 (closed)

## context_files

- `docs/design-run-loop-on-several-vcpus-2026-10-31.md` "A vCPU with no thread", the kick (196-198);
  `crates/w2m_vm/src/threads.rs` `suspend`, `switch`, `E_VCPU`, `E_SUSPEND`; `crates/w2m_vm/src/run.rs` `RunStop`,
  `vcpu_loop`'s cancel; `probes/threads/threads.c` 164-194; BLACKBOARD Deferred (Sprint 329)

## signal contract

### Emits

- None; when a suspended thread stops.

### Invariants

- Every run on one vCPU as held; on one vCPU no thread is held elsewhere, and `suspend` is as today.

## artifact contract

### Files modified

- `crates/w2m_vm/src/threads.rs`, `crates/w2m_vm/src/run.rs`, `crates/w2m_vm/tests/run_loop.rs`; the design's card 4
  note, with the work

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** a test running the threads family probe through `w2m-vm` on two vCPUs, with the arguments its
  `known` row gives: `suspend.frozen`'s `count_unchanged` 1 and `context`'s `rip_in_image` 1, held to those values,
  and every row equal to the one-vCPU run's; the five guests on two vCPUs ending with their held stop and output;
  every run on one vCPU as held.
- **Planted faults:** (1) the stop of the holding vCPU taken out of `suspend`: the test reads `count_unchanged` 0, as
  measured before this card; (2) the wait for the `vcpu` word taken out: the context read before the vCPU has put the
  thread down, `rip_in_image` 0. Each judge's reach read before this card's commit (WORKING_AGREEMENT, Sprint 284):
  the two rows were run on two vCPUs without the kick and read 0, where one vCPU reads 1.

## done criteria

A thread suspended while another vCPU runs it stops before SuspendThread returns, as on one vCPU.

## self-review checklist

- [x] Scope names one verifiable property: a suspended thread on another vCPU stops.
- [x] context_files cite the design, the suspend, the run's stop and the probe's rows.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values held to known values and the one-vCPU run, two plants, each judge's reach run.
- [x] One concept: the kick.

## result

