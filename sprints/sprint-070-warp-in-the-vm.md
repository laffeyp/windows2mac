# Sprint 070 — the deadline warp inside the VM

```yaml
---
id: 070
status: open
phase: 5
pass_kind: functional
---
```

## scope

Recentering step 5 (`docs/recentering-2026-10-14.md`), the rest of the scheduler's part. Sprint 069's batch run with
switches in the VM still left the VM 57,782 times to hand the vCPU over (hvc 12). Its closing note names them: waits
that leave no thread ready, where the host's `warp_to_deadline` moves the instruction clock to the earliest deadline of
a blocked thread and runs that thread. The rule reads only the run table and the clock words, which the guest reads.

- **A. The warp in the VM.** When a blocking wait (RETRY) leaves no thread ready and the clock is the instruction
  clock, `w2m_switch` moves CLOCK_WARP to the earliest finite deadline among the unsuspended blocked threads (the
  waiting thread counted, the lower thread id first on a tie), makes that thread ready and switches to it, as
  `_switch` and `_resume_idle` do. Another clock, a thread's end and no deadline at all stay with the host.
- **B. The census.** hvc 12 by cause, exits by kind, the rate and the wall time of the batch run on a warm translation
  cache, before and after, the Mac to itself.

## prerequisites

- 069 (closed: CI run 38654917874)

## context_files

- `src/w2m/vm_backend.py` (`_switch`, `warp_to_deadline`, `_resume_idle`, `_other_process_due`),
  `src/w2m/kernel.py` (`Machine._next`), `tools/guest_runtime/threads.c` (`w2m_switch`),
  `tools/guest_runtime/process.c` (`counter_now`)

## signal contract

### Emits

- The frontier's records (EXECUTION_STARTED, its stop, TRACE_INGESTED, COMPARISON_COMPLETED, SESSION_COMPLETED);
  the family sweep's records (CANDIDATE_ACCEPTED for all 68 families) on the runtime change.

### Invariants

- Vocabulary unchanged; no new tags without a proposal.
- The frontier's comparison stays conforming (0 divergences by `call_compare.by_cause`).
- `SWITCH_IN_VM` on and off give equal thread censuses and equal rows.

## artifact contract

- modified: `tools/guest_runtime/threads.c`, `tests/test_switch_in_vm.py` (a plant: the warp leaves the clock
  where it was)
- `uv run pytest` exit 0; the family sweep 68 of 68 accepted; CI `success` on the closing commit

## observation contract

- **The census before and after,** from the frontier's `time:` and `progress:` lines on the batch run: exits by kind,
  instructions a second, wall seconds.
- **The batch run conforms:** 0 divergences, 168 of 168 lines.

## done criteria

A built with a plant that the test catches; the batch run conforms; B measured on a census run that had the Mac to
itself; CI `success`.

## observed
