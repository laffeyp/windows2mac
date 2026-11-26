# SIGNAL_REPORT — Sprint 355 — The handle table's lock: creates, duplicates and id lookups leave the runtime word

Made by `signal_report 355` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 355.

## 1. Observed

### scope_confirmation

> Card 4 of `docs/design-handles-and-sync-objects-2026-11-26.md`: the handle table's own lock, so that a create, a
> duplicate or an id lookup never takes the runtime word. The design is the spec ("Which lock guards which state",
> "The order, and the two leaves", "Where each service's class lands", "The judges each card needs", "The cards that
> follow" 358-360). The parts:
> - `handles_lock` in `tools/guest_runtime/sync_objects.c`'s data, taken and let go in the form of `hold_slots` and
>   `let_slots_go` (`thread_storage.c` 65-75).
> - In `crates/guest_runtime/src/services.rs`, `Lock::Handles` and the list `HANDLES_LOCK`: CreateSemaphoreExW,
>   CreateSemaphoreW, CreateMutexA, CreateEventW, CreateEventA, CreateEventExW, DuplicateHandle, OpenThread, GetThreadId,
>   GetProcessId. The class test follows Sprint 345's.
> - **The class first.** Once these ten services run with no runtime word, every other reader or writer of the table's
>   state races them. That state is items 2, 3 and 4, the allocation of an entry, a FREE entry's `kind`, and both counts,
>   `handles` and `refs`. So, before any service moves, list every reader and writer of that state in the runtime C, one
>   line each with its file:line and the lock it holds today. Each must take `handles_lock`, or the list must say why it
>   need not. The list includes at least:
>   - `handle_for`, `new_object`, `slot_of`, `object_of`, `unref`, `w2m_object_close` and `inherit_init`;
>   - the waits' and releases' `refs` changes (the waiter references of Sprints 350 and 352), still under the word;
>   - CloseHandle (`file.c`), CreateProcessW's read of every slot (`processes.c` 144), and the token calls;
>   - `w2m_object_flags`, which returns a pointer into `handle_flags` that `file.c`'s Get/SetHandleInformation read and
>     write after the lookup returns. A pointer used outside the lock is this class's defect: change the interface so
>     that the read and the write happen under the lock.
>   - Report each further one to the Supervisor at once, with its evidence.
> - **The order.** `handles_lock` is a leaf: held, it takes no other lock, makes no host request, readies no thread and
>   calls neither `w2m_unimplemented` nor `w2m_logged`. A named event's `W2M_KERNEL_EVENT_OPEN` is made by taking the
>   runtime word in the C with no other lock held (the design, 290-291). Say how the C takes the word, whether through
>   `hold_runtime_word` (`runtime.c` 63-77) or another way, and why. A service still on the word takes `handles_lock`
>   inside the word as a leaf.
> - **Under one vCPU nothing observable changes.** The batch run, the family runs, the exit ledger, `wait_handoff`,
>   `wait_closed_handle` and `wait_closed_kinds` must pass unchanged.
> - **The judge.** The design's "Handles from two vCPUs" guest, built as `tests/data/timed_waits`' guests are: two
>   threads, each creating and closing 10,000 unnamed events and each calling DuplicateHandle 10,000 times, on two
>   vCPUs. It holds that every value open at once is distinct, that every close answers TRUE, and that no slot is lost:
>   a further create at the end answers the value the first create answered. Run it 20 times on two vCPUs. It must
>   reach the race: with the services moved off the word and `handles_lock` taken nowhere (plant 1), it must fail, and
>   the result says in how many of 20 runs. If plant 1 does not fail, strengthen the judge until it does, and report
>   that.
> - **Not here:** the sync objects' lock (card 5); any wait or release off the word.

### work_performed

- ee2180a63 Sprint 355 card: the handle table's lock, creates, duplicates and id lookups off the runtime word, every reader and writer listed

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; which lock the handle services take.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 355: its Rubber Duck Pass and close.
