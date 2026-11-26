# Sprint 356 — The sync objects' lock: waits and releases leave the runtime word

```yaml
---
id: 356
status: done
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

## amendment (2026-11-26)

Reviewer-4's twenty-sixth review, Medium 1, verified by the Supervisor. A mutex whose owner thread ends is never
released. `sync_objects.c` keeps a mutex's owner tid in `max` and its recursion in `count`, and no thread-exit path
(`k32_ExitThread`, `threads.c` 365-370, or the host's end of a thread) touches a mutex. "Abandon" appears in the
runtime C only in a `block_compiler.c` 317 comment. A later INFINITE wait on such a mutex never returns, which can
leave a thread past its deadline, so the user's Decision of 2026-10-18 puts the fix in this card. Abandonment writes
the very state `sync_lock` guards: a mutex's owner and recursion, and a waiter's grant. The game's import capture
counts 0 calls of CreateMutexA, CreateMutexW, OpenMutexW and ReleaseMutex (`store/game-runs/38006665419/
jit_capture.bin.imports.tsv` 21, 776-778), so today's batch run does not reach it.

What Windows defines (Microsoft Learn, fetched 2026-11-26):
- **WaitForSingleObject**, WAIT_ABANDONED 0x00000080L: "The specified object is a mutex object that was not released
  by the thread that owned the mutex object before the owning thread terminated. Ownership of the mutex object is
  granted to the calling thread and the mutex state is set to nonsignaled."
  https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitforsingleobject
- **WaitForMultipleObjects**, WAIT_ABANDONED_0 to WAIT_ABANDONED_0 + nCount - 1:
  - "If bWaitAll is TRUE, a return value within the specified range indicates that the state of all specified objects
    is signaled and at least one of the objects is an abandoned mutex object."
  - "If bWaitAll is FALSE, the return value minus WAIT_ABANDONED_0 indicates the lpHandles array index of an abandoned
    mutex object that satisfied the wait."
  https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitformultipleobjects
- **Mutex Objects:** "If the thread proceeds as though the mutex object had not been abandoned, it is no longer
  considered abandoned after the thread releases its ownership."
  https://learn.microsoft.com/en-us/windows/win32/sync/mutex-objects
- **ReleaseMutex:** "fails if the calling thread does not own the mutex object". The page names no error code; the
  runtime's ERROR_NOT_OWNER (288, `sync_objects.c` 11) stays as the runner shows.

The work:
1. **A Windows row, first.** Write `probes/mutex_abandoned/`, in the form of `probes/wait_closed_kinds`. Each step is
   on a mutex whose owner thread returns from its thread function while holding it:
   - the main thread waits with WaitForSingleObject;
   - it waits again after a ReleaseMutex, to read the abandoned state cleared;
   - a WaitForMultipleObjects any and all form;
   - a worker blocked INFINITE on the mutex when its owner ends;
   - a non-owner's ReleaseMutex, with its last error.
   Send it to the Supervisor, who commits it as a card amendment and runs it on the Windows runner. Hold its rows to
   the runner's.
2. **The fix.** When a thread ends, under the word and then `sync_lock` as this card's order allows, every mutex its
   tid owns is marked abandoned and left unowned. The first waiter is granted with WAIT_ABANDONED (or WAIT_ABANDONED_0
   + index), and a later wait that finds the mark takes the mutex and answers it once. Find every path by which a
   thread ends (ExitThread, a return from the start routine, the host's end of a thread, process exit) and list them in
   the class list.
3. **The judge.** The probe's rows on one vCPU and on two, held to Windows'.
4. **Plant (6).** The exit path's abandonment removed. The probe's test fails, the main thread's wait timing out, or a
   worker never returning.

## amendment 2 (2026-11-26)

Reviewer-4's twenty-seventh review, Medium 1, verified by the Supervisor against `WORKING_AGREEMENT.md` 300,
"Signals that bind": "'None new' is not an Emits for code that runs". Cards 343, 345, 355 and this card declared
`Emits: None` while moving services off the runtime word. Yet `crates/w2m_vm/src/bin/run_measurements.rs` (Sprint
337) records the word's takings, hold and wait on two vCPUs (`runtime_word_hold_ns_p50/p99`,
`runtime_word_wait_ns_p50/p99`, vocabulary 0.11 and later), and Sprint 342 emitted them.

- **This card's Emits become:** MEASUREMENT_RECORDED with `vcpus` 2 and `runtime_word_hold_ns_p50`,
  `runtime_word_hold_ns_p99`, `runtime_word_wait_ns_p50` and `runtime_word_wait_ns_p99`, on a run that drives the eight
  services. The expectation: the runtime word's takings by WaitForSingleObject, WaitForMultipleObjects, SetEvent,
  ResetEvent, ReleaseSemaphore and ReleaseMutex fall, and the waits and releases take the word only to park, to ready
  and for host requests.
- **The measurement:**
  - Run `run_measurements` on two vCPUs before the change (HEAD's C and `services.rs`, put back by `cp` and restored)
    and after. Add `semaphore_units` (and `lost_wakeup` if it serves) to its rows, so the record covers these services.
  - Commit the after record under `store/runs/`, and quote both sets of figures in the result.
  - The close's Signal Report reads that record's trace, and the Rubber Duck Pass narrates it.
  - Sprint 342's record, `store/runs/w2m-vm-measurements-1795665127000000000`, shows the form.
- **Not here:** the per-lock hold and wait metrics, which need a vocabulary bump. They go to the card after this one
  (a Decision at the close).

## result

- **Files changed:** `tools/guest_runtime/sync_objects.c` (`sync_lock`, the eight services, abandonment), `threads.c`
  (the armed mark, `W2mThreadEnd`'s abandonment), `crates/guest_runtime/src/services.rs` (`Lock::SyncObjects`,
  `SYNC_LOCK`, `lock_in` on the host's clock, the class test), `crates/w2m_vm/src/guest.rs` (the bridge test's class),
  `crates/w2m_vm/tests/run_loop.rs` (semaphore_units' test; lost_wakeup's failures counted; the woken mark's rows armed
  and unarmed; the C switch's load test; mutex_abandoned's test), `crates/w2m_vm/src/bin/run_measurements.rs` and
  `tests/measurements_record.rs` and `harness/w2m-vm-measurements.toml` (amendment 2), `crates/guest_runtime/tests/
  python_image.rs` (`DEPARTURES`), `crates/image_walk/tests/fixtures.rs`, `tests/data/timed_waits/semaphore_units.{c,exe}`
  (new; built `x86_64-w64-mingw32-gcc -O1 -nostdlib -Wl,-e,start -o semaphore_units.exe semaphore_units.c -lkernel32`,
  GCC 16.2.0), `store/batch_run/first-stop-2026-10-28.tsv` (its held line), `store/binaries/2e7af44b….exe` (the runner's
  mutex_abandoned binary) with its walk-fixture entry, `store/runs/w2m-vm-measurements-1795719438000000000` (new).
- **The class list** (final tree; every line HEAD held under the runtime word):
  - `grant` sync_objects.c 432, `grant_all` 552, `has_grant` 421, `sync_take` 493 (was `take`, `take_for`), `take_every`
    512, `sync_signaled` 487 (was `signaled`, `signaled_for`): `sync_lock`, called only inside a release's or a wait's
    holding.
  - `waiter_add` 394, `waiters_out` 401 (was `waiter_drop`, `waiters_drop`), `all_out` 479 (was `all_drop`),
    `waiter_of` 385, `all_of` 471, `granted_slot` 414, `entries_of` 863: `sync_lock`; the references the entries held are
    dropped after it under `handles_lock` (`object_done`, `objects_done`).
  - `timed_out_at` 716 (was `timed_out`): no lock; it reads and clears the waiting thread's own TEB `wait_deadline` and
    makes no host request (the counter read happens outside every leaf, `w2m_counter_now`; a host request on clock mode 1,
    where `lock_in` puts the eight on the word).
  - `wait_one` 760: entry read in one holding, check-and-entry in another, mark armed in it, park under the word alone
    (`park` 379). `wait_one_held` 821 (a thread, a process, a shared event): the word from check to park; entry under
    `sync_lock` inside it. `wait_any` 880 and `wait_all` 994 (this file's objects): one holding each, armed, `park`.
    `wait_any_held` 930 and `wait_all_held` 1055: the word throughout; per-object peeks and one deciding holding.
    `k32_WaitForMultipleObjects` 1134: retry entries read under `sync_lock`, lookups under `handles_lock`.
  - SetEvent 620, ResetEvent 646, ReleaseSemaphore 662, ReleaseMutex 687: state and grants under `sync_lock`; readies
    (`ready_all` 372) and a manual-reset event's `w2m_signal` under the word after; a shared event's request under the word.
  - `w2m_mutexes_abandoned` 582 (amendment): mutexes found and referenced under `handles_lock`, abandoned and granted
    under `sync_lock`, readied under the word the bridge of `k32_W2mThreadEnd` (threads.c 405) holds.
  - `host_signaled` 528, `host_take` 545: the word, never `sync_lock` (they take `handles_lock` and make host requests).
  - Further readers found and reported: `w2m_object_close` 234 read `count` of every kind under `handles_lock`, and
    `w2m_object_kind` 1172 with no lock; both now read it only of kinds fixed at creation (`changes`). `grant_all` reached
    `signaled` (handles_lock and host requests) for a wait-all naming a host object; such a wait is now readied to check
    again under the word, never taken for. `timed_out`'s counter read is a host request in clock mode 1; `take` made
    EVENT_TAKE inside a wait-all's loop; `signaled` called `w2m_unimplemented` inside a check.
  - Not this lock's state: CloseHandle's path (kind, refs, handles under `handles_lock`), GetThreadId, GetProcessId,
    DuplicateHandle, OpenThread, `w2m_object_mapping_of`, `w2m_object_thread_of`, `w2m_object_inheritable` (fixed kinds
    only), Get/SetHandleInformation (flags).
  - Items 10, 11, 13, 14: `wait_deadline` is written only by its own thread (`timed_out_at`, the waits' success paths,
    `w2m_block` threads.c 97) and read by it and its vCPU's switch (`nearest_deadline` of the current entry); `wait_key`
    written by `w2m_block` and read by `w2m_signal` 175 only of BLOCKED threads, both under the word; the run table
    (`set_status` 76) and the census written by `w2m_block`, `w2m_ready_thread` 198, `w2m_signal`, all under the word;
    the woken mark armed by `w2m_wait_armed` 136 under `sync_lock` (a relaxed store), set by `mark_woken` 154 (CAS ARMED
    to WOKEN) and cleared by `w2m_block` (exchange), both under the word, `w2m_wait_again` 143 under the word, the switch's
    load 362.
  - Thread ends (amendment): ExitThread and a start routine's return both reach `k32_W2mThreadEnd` (the attach stub
    calls ExitThread; ExitThread's detach stub calls W2mThreadEnd); the runtime has no TerminateThread; ExitProcess ends
    the process, leaving no waiter of an unnamed mutex.
- **RETRY with no word held by the bridge:** the bridge stores RAX, reads `schedule`'s RETRY bit, puts RIP at the thunk
  and branches to the switch stub, which takes the word itself (guest.rs 402) before any store to the run table; a
  readying between the C's release of the word after `w2m_block` and that take finds the thread BLOCKED and readies it,
  and the switch then does not block it (`retry` needs BLOCKED). Judged by semaphore_units and lost_wakeup below.
- **Deferred (a) line by line:** `w2m_ready_thread` and `w2m_signal` now mark only an ARMED thread (the waits checked
  outside the word arm); the host's `wake_blocked` and SCHEDULE_END keep `held_elsewhere`'s marking (threads.rs 636),
  which arming does not cover: no wait checks a thread's, a process's or a shared event's state outside the word, so
  they need no mark, but removing them is the host's code, not this card's. The woken test: armed, all four READY;
  unarmed, the C's two BLOCKED (no spare retry), the host's two READY.
- **The built lock** (image `build/guest_runtime_rust/ae251e58….bin`, `_sync_lock` at 0x7f00044a8d48),
  `_k32_ReleaseSemaphore`: `ldr w10, [x8, #0xd48]; cbnz w10; casa w10, w9, [x26]` at …5040-…5048, `bl _grant`
  …50f0, `bl _grant_all` …50fc, `stlr wzr, [x26]` …5100, then `bl _w2m_take_runtime_word` …5114 and
  `bl _w2m_ready_thread` …5128; the over-maximum path's `stlr wzr, [x26]` at …5060.
- **semaphore_units** (its last printed run, the tree before the two count readers' fix; the gates' run of the final
  tree passes it): 21 of 21, every run "done": 400,000 releases TRUE, 400,000 waits satisfied, final count 0, each
  lock's two counts equal (mutex 110,298 to 114,464 entries, turn 109,564 to 113,569 on two vCPUs), shared 0, gated
  6,248, the run table's blocked count 0 against 0 entries; on HEAD's code 21 of 21 (the guest before its last two
  strengthenings).
- **lost_wakeup:** 21 of 21, 1,800,073 instructions in each of the 20 two-vCPU runs, limit 20,000,000.
- **mutex_abandoned** (Windows run 45871497648): all five rows equal on one vCPU and on two.
- **`two_counters`:** before (HEAD binary) 166,309,103, 170,008,530, 164,036,208, 165,635,525, 167,564,873 ticks;
  after 156,187,110, 155,840,725, 153,220,162, 154,654,666, 156,507,346.
- **Amendment 2:** before (HEAD 9a0f0e937's C and services.rs put back by cp, the new row kept; root
  c2ac1a917af5b3cadce94230897561f47751cadc): 1,576,566 takings; bridge 15,472 of the first 16,384, hold p50 208 ns p99
  1,500. After (the final tree, root 56c5c9eadf731ea9d5a3cf7066392184776eb4e5 before the record was added;
  `store/runs/w2m-vm-measurements-1795720575000000000`, replay level 2, check_records 0 violations): 69,264 takings;
  bridge 12 of 16,384, hold p50 13,917 p99 36,667 ns; switch 16,195, hold 583/958. The holds and the takings counted are
  the figures. On two vCPUs every wait figure is a lower bound, untrusted: the timing area keeps one wait start, which a
  second vCPU's wait writes over while the first holds the word, moving the start later; the evidence is the waits whose
  start passed their taking, 4,349 of 16,384 before and 146 of 16,384 after, left out and named in each variant (a wait
  start a vCPU is the per-lock vocabulary card's, a Decision).
- **Plant 1** "semaphore_units: 20 of 21 runs failed (one on one vCPU, 20 on two)": `hold_sync` returns at once; run 2
  "output \"failed\\n\" … wrong 3, final 1".
- **Plant 2** "lost_wakeup: 20 of 21 runs failed (one on one vCPU, 20 on two)": the mark ignored in `w2m_block`; run 1
  "every thread waits with no timeout and nothing can wake one (a deadlock)".
- **Plant 3** "assertion `left == right` failed: the services of lock class sync objects": WaitForSingleObjectEx
  renamed in `SYNC_LOCK`, seven against eight.
- **Plant 4** "semaphore_units: N of 21 runs failed": each holding removed in turn, 20 two-vCPU runs each: SetEvent,
  ReleaseSemaphore, ReleaseMutex, wait_one's check, wait_one_held's entry, wait_any, wait_any_held's decision, wait_all,
  wait_all_held's decision, the word's ready, wait_one_held's word, wait_all_held's word, the arming (lost_wakeup);
  the park's word ("panicked at crates/w2m_vm/src/threads.rs:619:67: attempt to subtract with overflow", the blocked
  count corrupted); WFMO's retry entries (2 of 20); and, after the two count readers' fix, `w2m_object_close`'s table
  holdings (sync_objects.c 235-247), "handle_values: 20 of 21 runs failed (one on one vCPU, 20 on two)", run 1
  "CreateEventA argument form 0x100", made 0xfa. The count readers' first form, `sync_kind`, zeroed a mapping's
  descriptor at its close: the gates' family run file_mapping, "file primitive 8 on descriptor 0, which is not open". Grounds for those no run fails: ResetEvent's lone store; the two
  peeks of the held waits (the deciding holding re-reads every object); SetEvent's word (no two-vCPU run sets a named
  event; the gate's signal finds a blocked waiter rarely); wait_any_held's word (its first holding takes a unit nearly
  always); w2m_mutexes_abandoned's holding (no other vCPU is in a section on the mutex at a thread's end in any run);
  `w2m_wait_again` (a later release readies the wait). Each plant restored by cp, checked by cmp.
- **Plant 5** "assertion `left == right` failed: the switch's answer, the current thread, T's holder, T's mark after its
  load, and T's status after its park; left: (1, 1, 1, 1, 0) right: (1, 1, 1, 0, 1)".
- **Plant 6** "mutex_abandoned, fields unlike Windows': [\"1 vCPU(s) single.after: 258 against 0\", …": the abandonment
  in `W2mThreadEnd` removed.

exit 0 in 436 s	default	2026-11-26T19:28:36Z	sprint 356	9a0f0e937046131660b0bc637cb6f91a3c97630f	54edb180965c91cf8de489b40cdd5d443ffc749f329e78693466d774ffbcf2f5	root=672212e2740664b687df1ab4401a849b50835b3f	rust=all python=0 samples=1	base=96360d6264068da6e9b6ec8392345cec7c4362eb
