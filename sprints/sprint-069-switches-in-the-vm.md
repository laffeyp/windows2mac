# Sprint 069 — thread switches inside the VM

```yaml
---
id: 069
status: closed
phase: 5
pass_kind: functional
---
```

## scope

Recentering step 5 (`docs/recentering-2026-10-14.md`), the scheduler's part. The conforming batch run on the corrected
count (f99c0d06's records) left the VM 539,947 times at slice ends with another thread ready (hvc 4), 267,255 times to
hand the vCPU over at a wait, a yield or a thread's end (hvc 12), and 135,994 times for host requests (hvc 8), among
them the signals that wake a waiter. Each of these runs Python to make a decision that needs only memory the guest
can read. This sprint moves the decision and the switch into the VM, by the host's own rule, so a run that switches in
the VM makes the same switches at the same instruction counts as one that switches in the host.

- **A. The run table.** Each thread's saved state (x0-x30, the per-thread monitor words, the state block), its status,
  suspend count, deadline and census counters live in one table in the monitor's memory. The host's `GuestThread`
  reads and writes them there; the running thread's index is the table's, so the host and the guest read one record.
- **B. Signals in the VM.** `w2m_signal` readies the waiters on its key itself, by the host's rule (blocked, and the
  signal's key 0, the wait's key 0, or the two equal); the published key table and the host's THREAD_SIGNAL go.
- **C. The switch in the VM.** A slice's end, a blocking wait (RETRY) and a yield (YIELD, ANSWER) choose the next
  thread as `_switch` does (round robin by thread id over ready, unsuspended threads; at a slice's end, timed-out waits
  made ready first on the instruction clock) and switch without an exit. The host keeps a thread's creation and end, a
  wait with no thread ready (idle, warp, sleep), lockstep, a forced stop, and a machine of several VMs.
- **D. A slice switch's count.** A switch at a block's stop resumes the thread at its block's entry, which counts the
  block's instructions again; the switch gives them back (each host slice switch counted its block twice).
- **E. The census.** Exits by kind, the rate and the wall time of the batch run on a warm translation cache, before
  and after, the Mac to itself.

The GICv3, several vCPUs (step 7), and waking one waiter where Windows wakes one stay for later items of step 5 and 7,
each from a census.

## prerequisites

- 068 (closed: CI run 38632198138)

## context_files

- `src/w2m/vm_backend.py` (`_slice_stub`, `_bridge`, `_schedule`, `_switch`, `_save_thread`, `_load_thread`,
  `_wake_expired`, `_slice_until`, `_thread_request`), `src/w2m/scheduler.py`, `tools/guest_runtime/threads.c`,
  `tools/guest_runtime/runtime.h`, `tools/guest_runtime/runtime.c` (`guest_rip_at`), `tools/vm_monitor/vm_monitor.c`

## signal contract

### Emits

- The frontier's records (EXECUTION_STARTED, its stop, TRACE_INGESTED, COMPARISON_COMPLETED, SESSION_COMPLETED);
  the family sweep's records (CANDIDATE_ACCEPTED for all 68 families) on the runtime change.

### Invariants

- Vocabulary unchanged; no new tags without a proposal.
- The frontier's comparison stays conforming (0 divergences by `call_compare.by_cause`).
- With a switch `SWITCH_IN_VM` off, every switch goes through the host as before; on and off give equal thread
  censuses (instructions and PAUSEs per thread) and equal call streams.

## artifact contract

- modified: `src/w2m/vm_backend.py`, `src/w2m/scheduler.py`, `tools/guest_runtime/threads.c`,
  `tools/guest_runtime/runtime.h`, the sync services that call `w2m_signal`
- new: `tests/test_switch_in_vm.py`: the threads families switched in the VM and in the host give equal censuses and
  rows; a plant in the VM's rule (the rotation's start, a skipped suspend count) changes a census
- `uv run pytest` exit 0; the family sweep 68 of 68 accepted; CI `success` on the closing commit

## observation contract

- **The census before and after,** from the frontier's `time:` and `progress:` lines on the batch run: exits by kind,
  instructions a second, wall seconds.
- **Equal interleaving:** a frontier prefix of 10 billion instructions with `SWITCH_IN_VM` on and off gives the same
  thread census and the same call stream.
- **The batch run conforms:** 0 divergences, 168 of 168 lines.

## done criteria

A to D built with the test failing before each; the batch run conforms with switches in the VM; E measured on a
census run that had the Mac to itself; CI `success`.

## observed
- **A to D** (d220b420; index de414a3b). The run table holds each thread's saved state, status, suspend count,
  deadline and census in the monitor's memory, and `GuestThread` is a view of its entry. threads.c `w2m_switch` makes
  slice ends, blocking waits and yields switch without an exit; `w2m_signal` readies waiters itself (the published key
  table and THREAD_SIGNAL are gone). Each slice switch had counted its block's instructions twice; both paths now give
  them back. `tests/test_switch_in_vm.py`: threads, running_thread_context, wait_handoff, thread_stacks, late_tls and
  processes, on the instruction clock with `SWITCH_IN_VM` on and off, give equal rows and equal per-thread instruction
  and PAUSE counts, with fewer hvc 12 exits. Plants: a suspended thread switched to and a slice's count not given back
  each change the census. A third, the rotation started one thread late, hangs the probe (two threads never switch).
  A fourth, SwitchToThread's answer inverted, changes nothing: no threaded probe reads it.
- **The switch beside a second process** (d33e4fb8; index 773579d1). The first batch run with A to D conformed and
  made 0 switches in the VM: the game starts UnityCrashHandler64.exe, and the switch stayed off for any machine of two
  VMs. It now stays on unless another process is ready or waits with a deadline, `_slice_until`'s own rule.
- **E, the census** (warm translation cache, the Mac to itself, the same tree with the switch off and on):

  | | switch in the host | switch in the VM |
  |---|---|---|
  | wall seconds | 92.4 | 63.1 |
  | rate at the last progress line | 1,297 M/s | 2,355 M/s |
  | hvc 4 | 290,780 | 0 |
  | hvc 12 | 206,756 | 57,782 |
  | switches in the VM | 0 | 1,255,821 |
  | comparison | 0 divergences, 168 lines | 0 divergences, 168 lines |

  hvc 10 rose from 117 to 13,415 and hvc 11 from 2,561 to 20,514: another interleaving reaches other code.
- **Closed** on 773579d1 (CI run 38654917874) with frontier-643a6283510c-1792020875595657000 conforming. Left for
  later items: hvc 12's 57,782 exits are waits with no thread ready (the host warps the instruction clock to the
  nearest deadline), which the VM can do too; hvc 1 (59,417 dispatch misses) is step 6's.

