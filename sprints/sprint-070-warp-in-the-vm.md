# Sprint 070 — the deadline warp inside the VM

```yaml
---
id: 070
status: closed
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
- **A** (3de2ed89; indexes bc6c956c: 456 of 458 classes, 68 families). `w2m_switch` warps the instruction clock to
  the earliest deadline (threads.c `nearest_deadline`) when a blocking wait leaves no thread ready, and switches to
  that thread. The host counts its own hand-overs by cause on the progress line. `tests/test_switch_in_vm.py`
  `test_the_vm_warps_as_the_host_does`: threads, running_thread_context, wait_handoff and processes give equal rows and
  census with the VM's warp and with the warp taken out of the VM (the host warps), and the host makes fewer hand-overs
  with no thread ready (wait_handoff 15 to 0, threads 3 to 0, running_thread_context 1 to 0, processes 7 to 3; the 3
  left run beside its child process, which the host keeps). The first plant, the warp leaving the clock where it was,
  hung the probe instead of changing a census: the woken thread finds its deadline unpassed, blocks again, and the warp
  repeats. The reach check replaced it.
- **B, the census** (the same tree, threads.c's warp in the VM and taken out; warm translation cache, the Mac to
  itself; exits summed over the progress lines):

  | | warp in the host | warp in the VM |
  |---|---|---|
  | wall seconds | 63.17 | 62.42 |
  | rate at the last progress line | 2,288 M/s | 2,839 M/s |
  | hvc 12 | 57,352 | 2 |
  | hand-overs with no thread ready | 57,350 | 0 |
  | switches in the VM | 1,192,117 | 1,602,079 |
  | comparison | 0 divergences, 168 lines | 0 divergences, 168 lines |

  The wall time moved 0.75 s: 57,350 exits that each ran a short Python path. hvc 1 (275,799 dispatch misses over
  the run) is now the largest exit by count and by host time (33.2 s of the recorded run's host time, cold).
- **The interleaving differs between a cold and a warm cache.** The host-warp tree made 39,117 hand-overs with no
  thread ready cold and 57,350 warm, and switched 954,378 and 1,192,117 times in the VM; hvc 8, 10 and 11 differ too.
  Both runs conform. A slice ends at a block's stop, and which blocks exist depends on what was translated before, so
  the same instruction count can fall in a different place. Not yet checked; it bears on step 10 (the snapshot).
- **Closed** on bc6c956c (CI run 38662524546) with frontier-643a6283510c-1792025575233476000 conforming (168 of 168
  lines, 0 divergences).
