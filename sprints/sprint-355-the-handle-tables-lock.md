# Sprint 355 — The handle table's lock: creates, duplicates and id lookups leave the runtime word

```yaml
---
id: 355
status: active
phase: 6
pass_kind: functional
---
```

## scope

Card 4 of `docs/design-handles-and-sync-objects-2026-11-26.md`: the handle table's own lock, so that a create, a
duplicate or an id lookup never takes the runtime word. The design is the spec ("Which lock guards which state",
"The order, and the two leaves", "Where each service's class lands", "The judges each card needs", "The cards that
follow" 358-360). The parts:
- `handles_lock` in `tools/guest_runtime/sync_objects.c`'s data, taken and let go in the form of `hold_slots` and
  `let_slots_go` (`thread_storage.c` 65-75).
- In `crates/guest_runtime/src/services.rs`, `Lock::Handles` and the list `HANDLES_LOCK`: CreateSemaphoreExW,
  CreateSemaphoreW, CreateMutexA, CreateEventW, CreateEventA, CreateEventExW, DuplicateHandle, OpenThread, GetThreadId,
  GetProcessId. The class test follows Sprint 345's.
- **The class first.** Once these ten services run with no runtime word, every other reader or writer of the table's
  state races them. That state is items 2, 3 and 4, the allocation of an entry, a FREE entry's `kind`, and both counts,
  `handles` and `refs`. So, before any service moves, list every reader and writer of that state in the runtime C, one
  line each with its file:line and the lock it holds today. Each must take `handles_lock`, or the list must say why it
  need not. The list includes at least:
  - `handle_for`, `new_object`, `slot_of`, `object_of`, `unref`, `w2m_object_close` and `inherit_init`;
  - the waits' and releases' `refs` changes (the waiter references of Sprints 350 and 352), still under the word;
  - CloseHandle (`file.c`), CreateProcessW's read of every slot (`processes.c` 144), and the token calls;
  - `w2m_object_flags`, which returns a pointer into `handle_flags` that `file.c`'s Get/SetHandleInformation read and
    write after the lookup returns. A pointer used outside the lock is this class's defect: change the interface so
    that the read and the write happen under the lock.
  - Report each further one to the Supervisor at once, with its evidence.
- **The order.** `handles_lock` is a leaf: held, it takes no other lock, makes no host request, readies no thread and
  calls neither `w2m_unimplemented` nor `w2m_logged`. A named event's `W2M_KERNEL_EVENT_OPEN` is made by taking the
  runtime word in the C with no other lock held (the design, 290-291). Say how the C takes the word, whether through
  `hold_runtime_word` (`runtime.c` 63-77) or another way, and why. A service still on the word takes `handles_lock`
  inside the word as a leaf.
- **Under one vCPU nothing observable changes.** The batch run, the family runs, the exit ledger, `wait_handoff`,
  `wait_closed_handle` and `wait_closed_kinds` must pass unchanged.
- **The judge.** The design's "Handles from two vCPUs" guest, built as `tests/data/timed_waits`' guests are: two
  threads, each creating and closing 10,000 unnamed events and each calling DuplicateHandle 10,000 times, on two
  vCPUs. It holds that every value open at once is distinct, that every close answers TRUE, and that no slot is lost:
  a further create at the end answers the value the first create answered. Run it 20 times on two vCPUs. It must
  reach the race: with the services moved off the word and `handles_lock` taken nowhere (plant 1), it must fail, and
  the result says in how many of 20 runs. If plant 1 does not fail, strengthen the judge until it does, and report
  that.
- **Not here:** the sync objects' lock (card 5); any wait or release off the word.

## prerequisites

- 354 (closed)

## context_files

- `docs/design-handles-and-sync-objects-2026-11-26.md`: in full.
- `tools/guest_runtime/sync_objects.c`: in full. Also `tools/guest_runtime/thread_storage.c` 20-90 (the lock's form and
  the rule on `w2m_unimplemented`), and `tools/guest_runtime/runtime.c` 55-100.
- `tools/guest_runtime/file.c`, `processes.c` and `token.c`: every call into `sync_objects.c`.
- `crates/guest_runtime/src/services.rs`: in full, Sprint 345's `SLOTS_LOCK` and its tests as the form.
- `crates/w2m_vm/src/guest.rs` 595-630 (the bridge and its lock classes).
- `crates/guest_runtime/tests/python_image.rs` (`DEPARTURES`), and `tests/data/timed_waits/three_counters.c` with its
  test in `crates/w2m_vm/tests/run_loop.rs`, for a w2m-only guest's form and its held stop.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None; which lock the handle services take.

### Invariants

- On one vCPU every run, the batch run, the exit ledger and the family runs pass unchanged.
- The 273 family runs on two vCPUs pass as at Sprint 351; `three_counters` and `lost_wakeup` as at Sprint 354.

## artifact contract

### Files modified

- `tools/guest_runtime/sync_objects.c` and `crates/guest_runtime/src/services.rs`; the callers' changes in `file.c`,
  `processes.c` and `token.c` that the class list requires (one concept, the design's named exception); the guest and
  its test; and any held line the gates name (`DEPARTURES`, the batch run's held stops).

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The class list, one line for each reader or writer.
  - The guest's 20 runs on two vCPUs: the count of values open at once, closes answered TRUE, and the final create's
    value.
  - `services.rs`' class test naming the ten services.
  - The built lock inline in one service, its loads and its `casa`, as Sprint 343 quoted the heap's.
- **Planted faults:**
  - (1) `handles_lock` taken nowhere, the ten services still in `HANDLES_LOCK`. The guest's test fails: a duplicate
    value, a failed close or a lost slot.
  - (2) One service taken out of `HANDLES_LOCK`. The class test fails.
  - (3) Each further reader or writer that the class list puts under the lock, its lock removed in turn. The guest's
    test, or a test the class list adds, fails. A reader that no run can reach says so in the result, with the grounds.

  Before any change, read the judge's reach on today's code. With the word around every call, the guest passes
  20 of 20.

## done criteria

Every reader and writer of the handle table's state is listed and takes `handles_lock`, or says why it need not. The
ten services run with no runtime word. Two vCPUs creating, duplicating and closing 40,000 handles keep every value
distinct and every slot. One vCPU and every existing two-vCPU run are unchanged.

## self-review checklist

- [x] Scope names one verifiable property: the table's state under one leaf lock; ten services off the word.
- [x] context_files cite the design, the C's readers and writers, the class form and the guest's form.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the class list, the guest's runs, the class test, the built lock; numbered plants with
      their reach read.
- [x] Sweet spot: `sync_objects.c` and `services.rs`, the callers named as the design's exception; one concept.

## result

GATES-LINE
