# Sprint 356 — The sync objects' lock: waits and releases leave the runtime word

```yaml
---
id: 356
status: active
phase: 6
pass_kind: functional
---
```

## scope

Card 5 of `docs/design-handles-and-sync-objects-2026-11-26.md`, the last of its five: the sync objects' own lock, so a
wait or a release holds the runtime word only to park, to ready or to make a host request. The design is the spec
("Which lock guards which state", "The order, and the two leaves", "The wait across two locks", "Where each service's
class lands", "The judges each card needs", "The cards that follow" 361-364). The parts:
- **The lock.** `sync_lock` in `tools/guest_runtime/sync_objects.c`, in the form of `hold_handles` and
  `let_handles_go`. It guards the mutable state of semaphores, events and mutexes (item 1's `count` and `max` for those
  kinds) and items 5 to 8: `waiters[]`, `nwaiters`, `all_waiters[]` and `nall`.
- **The services.** In `crates/guest_runtime/src/services.rs`, `Lock::SyncObjects` and the list `SYNC_LOCK`: SetEvent,
  ResetEvent, ReleaseSemaphore, ReleaseMutex, WaitForSingleObject, WaitForSingleObjectEx, WaitForMultipleObjects,
  WaitForMultipleObjectsEx. The class test follows Sprint 355's.
- **The word in the C.** These services take the runtime word themselves, with no leaf held, around:
  - parking (`w2m_block`);
  - readying (`w2m_ready_thread`, `w2m_signal`);
  - the host-backed kinds' requests (THREAD, PROCESS, SHARED_EVENT; the design, 294-299).
  Use `w2m_take_runtime_word` (Sprint 355). Say how the RETRY path (`W2M_SCHEDULE_RETRY`, runtime.h 136-142;
  `guest.rs` 624-627) behaves when the bridge holds no word, and judge it.
- **The class first.** Before any service moves, list every reader and writer of the state `sync_lock` guards, and
  every place a wait or a release reaches state the word keeps (items 10, 11, 13 and 14). Give one line each, with its
  file:line and the lock it holds today. Each must take the lock its state needs, or the list says why it need not.
  The list includes at least:
  - `grant`, `grant_all`, `take`, `take_for`, `has_grant`, `waiter_add`, `waiter_drop`, `waiters_drop`, `all_drop`,
    `timed_out`, `wait_one`, and WaitForMultipleObjects' any and all paths;
  - CloseHandle's path through these objects, and the services that stay on the word but read these kinds' state;
  - the TEB's `wait_deadline` and `wait_key` (items 10 and 11), written by a waiter and read by `w2m_signal` and the
    switch on another vCPU.
  Report each further one to the Supervisor at once, with its evidence.
- **The leaf rule holds.** `sync_lock` and `handles_lock` are never held together. Neither is held across the word,
  a host request, `w2m_unimplemented` or `w2m_logged`. References are dropped under the table's lock after `sync_lock`
  is let go (the design, 241-246).
- **What card 5 owes from earlier cards:**
  - **Sprint 354, Deferred (a).** Arm the woken mark at a wait's check, under `sync_lock`, so only a thread that has
    checked and not yet parked can take it. A thread running guest code elsewhere then takes none, and the spare
    retry ends. Remove the `held_elsewhere` marking only if the arming covers every path in Sprint 354's class list,
    and say so line by line.
  - **Sprint 354, Deferred (b).** The C switch's clearing of the mark at load gets a judge of its own.
  - **Sprint 353's Decision, "How a readying in the VM on one vCPU ends another vCPU's slice".** This card changes the
    switch path, so measure `two_counters` by the command in the Decision "How the slicing Decision's 7.6% is measured
    again", before and after, and quote both.
- **Under one vCPU nothing observable changes.** The batch run, the family runs (with `wait_handoff`), the exit ledger,
  `wait_closed_handle`, `wait_closed_kinds`, `three_counters` and `handle_values` must pass unchanged.
- **The judges.**
  - **The semaphore's units across vCPUs**, the design's guest, built as `tests/data/timed_waits`' guests are: four
    threads release and wait on one semaphore (maximum 1,000) 100,000 times. The final count equals the releases less
    the satisfied waits, and every wait returns. Run it 20 times on two vCPUs.
  - **The lost wakeup**, `tests/data/timed_waits/lost_wakeup`, kept by Sprint 354 for this card: 20 runs on two vCPUs,
    each inside its limit.
  - **Each plant must make its judge fail on two vCPUs**, and the result says in how many of 20 runs. A judge that does
    not catch its plant is strengthened until it does (Sprint 355's lesson), and the result says how.
- **Not here:** a lock per object (the design's Revisit if); the critical sections and the slim locks.

## prerequisites

- 355 (closed)

## context_files

- `docs/design-handles-and-sync-objects-2026-11-26.md`: in full.
- `tools/guest_runtime/sync_objects.c` and `tools/guest_runtime/threads.c`: in full. `tools/guest_runtime/runtime.c`
  55-100, `runtime.h` 30-150, and `processes.c` (WaitForMultipleObjectsEx).
- `crates/guest_runtime/src/services.rs`: in full, Sprint 355's `HANDLES_LOCK` and its tests as the form.
- `crates/w2m_vm/src/guest.rs` 395-410 (the switch stub's take of the word) and 595-630 (the bridge and RETRY).
- `crates/w2m_vm/tests/run_loop.rs`: the tests of `handle_values`, `lost_wakeup` and `three_counters`, and the woken
  mark's test, `a_readying_before_the_block_leaves_the_thread_ready`.
- `grep -n 'agent-resolved\*\* — How the slicing\|Which threads take the woken mark' BLACKBOARD.md`: those two
  Decisions, and `grep -n '(Sprint 354)' BLACKBOARD.md`: its Deferred entry.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None; which lock the waits and releases take.

### Invariants

- On one vCPU every run, the batch run, the exit ledger and the family runs pass unchanged.
- The 273 family runs on two vCPUs pass as at Sprint 351; `three_counters`, `lost_wakeup` and `handle_values` as at
  Sprint 355.

## artifact contract

### Files modified

- `tools/guest_runtime/sync_objects.c`, `threads.c` and `crates/guest_runtime/src/services.rs`; `processes.c` and
  `runtime.c` as the word's takes need (one concept, the design's card 5); the semaphore guest and its test; and any held
  line the gates name (`DEPARTURES`, the batch run's held stops).

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The class list, one line each.
  - The semaphore guest's 20 runs on two vCPUs, with its counts.
  - `lost_wakeup`'s 20 runs, with their instruction counts.
  - The class test naming the eight services.
  - The built lock inline in one service.
  - `two_counters` before and after.
- **Planted faults:**
  - (1) `sync_lock` taken nowhere, the eight services still in `SYNC_LOCK`. The semaphore guest's test fails: the
    count disagrees, or a wait does not return.
  - (2) The woken mark ignored in `w2m_block`. `lost_wakeup`'s test fails: a run hangs to its limit or stops as a
    deadlock.
  - (3) One service taken out of `SYNC_LOCK`. The class test fails.
  - (4) Each further holding the class list adds, removed in turn. Its judge fails, or the result gives the grounds
    why no run reaches it.
  - (5) The C switch's clearing of the mark at load removed. Its new judge fails.

  Before any change, read each judge on today's code: with the word around every call, the semaphore guest and
  `lost_wakeup` pass 20 of 20.

## done criteria

Every reader and writer of the sync objects' state is listed and holds `sync_lock`, or says why it need not. The eight
services hold the runtime word only to park, to ready and for host requests. Two vCPUs keep a semaphore's units
exact, and lose no wakeup. One vCPU and every existing two-vCPU run are unchanged.

## self-review checklist

- [x] Scope names one verifiable property: the sync objects' state under one leaf lock; eight services off the word.
- [x] context_files cite the design, the C, the class form, the judges and the Decisions this card answers.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the class list, two guests' runs, the class test, the built lock, `two_counters`; numbered
      plants with their reach read.
- [x] Sweet spot: `sync_objects.c`, `threads.c` and `services.rs`, the word's takes named as the design's card 5; one
      concept.

## result

GATES-LINE
