# Sprint 028 — witness the pending-waiters precondition

```yaml
---
id: 028
status: closed
phase: 3
pass_kind: functional
---
```

## scope

Replace the event probe's `Sleep(200)` assumption with an observation. Before `SetEvent`, main polls `NtQuerySystemInformation(SystemProcessInformation)` until both waiter threads show `ThreadState` 5 (Wait) with `WaitReason` 6 (UserRequest), up to 500 ms, then records one `GUEST_THREAD_STATE_OBSERVED` line per waiter (new guest tag, vocabulary 0.5) and calls `SetEvent`. Contract `windows.event.auto_reset.single_signal@0.3.0` adds a `preconditions` clause; `compare.py` returns `inconclusive` when a trace lacks the witness, before judging the outcome. Twenty new runner executions are judged under 0.3.0.

Why the witness is sufficient: between `InterlockedIncrement(&g_ready)` and `WaitForSingleObject`, the waiter calls nothing that can block. A thread observed in Wait/UserRequest after announcing itself is inside that `WaitForSingleObject`. It leaves only by timeout (2000 ms) or by the signal; kernel APCs to a non-alertable wait do not end it (they can reorder waiters, which the contract already allows).

## prerequisites

- 027

## context_files

- Microsoft Learn: `NtQuerySystemInformation` (SYSTEM_PROCESS_INFORMATION followed by SYSTEM_THREAD_INFORMATION with `ThreadState`, `WaitReason`; "may be altered or unavailable in future versions"; link at run time); .NET `System.Diagnostics.ThreadState` (Wait = 5) and `ThreadWaitReason` (UserRequest = 6; "only valid when the ThreadState is Wait"); Wait Chain Traversal (does not list event objects, so not usable here). All retrieved 2026-10-08.
- `probes/event_auto_reset/event_auto_reset.c`, `src/w2m/compare.py`, `src/w2m/contract.py`, `signals/0.4.json`

## signal contract

### Emits

- guest plane: `GUEST_THREAD_STATE_OBSERVED` (thread_id, thread_seq, hb_refs, subject_thread_id, thread_state, wait_reason, source) ×2 per execution, on main, before main's `SetEvent` line
- harness plane (judge): SESSION_INIT, EXECUTION_STARTED/COMPLETED, TRACE_INGESTED, COMPARISON_COMPLETED, SESSION_COMPLETED

### Invariants

- In every new trace, both observation lines precede `kernel32!SetEvent` in main's thread_seq order.
- A trace without the witness never yields `conforms` under 0.3.0.

## artifact contract

### Files created

- `signals/0.5.json`, `signals/0.5-rationale.md`, `contracts/windows.event.auto_reset.single_signal/0.3.0.json`

### Files modified

- `probes/event_auto_reset/event_auto_reset.c`, `src/w2m/compare.py`, `src/w2m/contract.py`, `signals/proposals.json`, `signals/current.json`, `src/w2m/events.py` (generated), `src/w2m/__init__.py`, tests

Hard rule 6 exception: one concept (a witnessed precondition) needs the vocabulary, the probe, the contract format and the comparator; recorded here.

### Command exit codes

- check_vocab, gen_events --check, pytest, ruff, mypy → 0 (unpiped); CI success

## observation contract

### Expected runtime signals

- runner run (repeat 20): every trace holds two `GUEST_THREAD_STATE_OBSERVED` lines with thread_state 5, wait_reason 6, before SetEvent; judge under 0.3.0: 20 of 20 conforms, or `inconclusive` naming an unwitnessed precondition.
- judge of the 12 earlier executions under 0.3.0: 12 of 12 inconclusive (no witness). Under 0.2.1: 12 of 12 conforms, unchanged.
- Which waiter is released: recorded as observed, not predicted.

## done criteria

The contract's precondition is observed in each counted run, and runs without the observation do not count.

## self-review checklist

- [x] Scope concrete. [x] Sources retrieved and named. [x] Emits uses a tag ratified in this sprint. [x] Observation declared before running. [ ] ≤2 files: exception recorded.

## observed

- Runner run 37854891607 (evt28-1..20): the probe compiled and ran with the declared layouts; every trace holds two `GUEST_THREAD_STATE_OBSERVED` lines, thread_state 5, wait_reason 6, on poll 1 of 500, at main thread_seq 1 and 2, before `kernel32!SetEvent`.
- Judge under 0.3.0: 20 of 20 conforms, released a in 20 of 20; exit code 0 in 20 of 20; elapsed 2.007-2.017 s. Under 0.3.1 (accepted on this evidence): 20 of 20 conforms.
- The 12 earlier executions: inconclusive under 0.3.0 (no witness), conforms under 0.2.1, as declared.
- Tamper tests: removing the witness, moving it after SetEvent, or changing state 5 to 2 each gives inconclusive.
- What this settles and what it does not: the 12-of-12 FIFO-looking result could have come from waiter b not yet waiting; with both proven pending, a is still released 20 of 20. Creation order and wait order still coincide (a first in both), so FIFO by wait order is not yet separated from FIFO by creation order (open question in 0.3.1).
- Process: part 1 was committed with a failing test (gates chained with `;`); fixed in the next commit, and `scripts/gates.sh` now gates every commit.
