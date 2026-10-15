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
- **B, the census.** The first census (63.17 s against 62.42 s, hvc 12 57,352 against 2) compared runs of unequal
  length and is withdrawn. The frontier checks its stop (the log's 168 lines and the main thread's calls) only on the
  progress line's 30-second wall-clock tick (`vm_backend.py` run loop, `run_frontier.py` `stop_when`), so a faster run
  goes further before it stops: thread 4 ran 72.8 billion instructions in the warm host-warp run and 85.4 billion in
  the warm VM-warp run. The progress line's time by exit and switch count also stop at its last tick; the end report
  now prints the whole run's (d6edf5ca).
- **The interleaving is the same in every configuration.** The batch run to a fixed count, twice with the translation
  cache warm and twice with it off, gave identical per-thread instruction and PAUSE counts at 20 billion and at 60
  billion instructions. At 100 billion, all six runs below (three configurations, two runs each) gave the same thread
  census, and each pair gave identical exits by kind and switch counts. The cold and warm figures that differed above
  came from runs of different length.
- **B, measured at equal work** (100,000,000,020 instructions, each run stopped at max_steps; warm cache; d6edf5ca's
  end report; switches in the host by `SWITCH_IN_VM` off, the warp in the host by threads.c's warp taken out):

  | | switches in the host | warp in the host | all in the VM |
  |---|---|---|---|
  | session seconds (two runs) | 79.4, 86.3 | 58.6, 56.7 | 51.6, 50.5 |
  | seconds inside the VM | 52.5, 56.4 | 44.8, 43.3 | 40.5, 39.5 |
  | hvc 4 | 647,278 | 9 | 9 |
  | hvc 12 | 352,097 | 39,484 | 2 |
  | host seconds at hvc 4, hvc 12 | 11.0, 5.1 and 5.2 | 0.0, 1.8 | 0.0, 0.0 |
  | switches in the VM | 0 | 959,611 | 998,771 |
  | comparison | 0 divergences, 168 lines | the same | the same |

  The warp in the VM takes 39,482 hand-overs with no thread ready out of the host; the session went from 56.7-58.6 s
  to 50.5-51.6 s. Two runs of one configuration differ by up to 6.9 s with identical work (load average 2.0 to 3.7 at
  their starts). hvc 1 (275,785 dispatch misses, 2.7-2.8 s of host time) and hvc 8 (50,405 host requests, 4.3-7.2 s)
  are the largest host exits left.
- **Closed** on bc6c956c (CI run 38662524546) with frontier-643a6283510c-1792025575233476000 conforming (168 of 168
  lines, 0 divergences).
