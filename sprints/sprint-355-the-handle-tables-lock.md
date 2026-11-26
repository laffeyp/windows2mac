# Sprint 355 — The handle table's lock: creates, duplicates and id lookups leave the runtime word

```yaml
---
id: 355
status: done
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

- **The lock** (`sync_objects.c`): `static u32 handles_lock;` (70) in the image's zeroed data, `hold_handles` (74: plain
  loads until it reads 0, then compare-and-swap of 0 for 1, acquire) and `let_handles_go` (83: a store of 0, release),
  the form of `hold_slots` and `let_slots_go`. A leaf: no holding calls w2m_host, w2m_unimplemented, w2m_logged,
  w2m_ready_thread, w2m_signal or another lock; `publish` (123), `inherit_init` (711), `w2m_object_inheritable` (686),
  `object_of`'s callers and DuplicateHandle let it go before a stop.
- **The class list** (`sync_objects.c` lines as changed; before the change every one ran under the runtime word, its
  bridge's, and none held another lock):
  - `handle_for` 97 (slots, flags, `refs`, `handles`) and `new_object` 111 (an entry's allocation, a FREE `kind`):
    under the table, held by their callers, `publish` 123, DuplicateHandle 778-783 and `inherit_init`.
  - `publish` 123, every create: `make` 134 (CreateSemaphoreExW 234, CreateSemaphoreW 827, CreateMutexA 273, an
    unnamed event 243), a named event 253, OpenThread 801, DuplicateHandle's pseudo-handles 774-775 (no word now);
    `w2m_object_token` 180, `w2m_object_mapping` 195, `w2m_object_new` 663, `w2m_object_thread` 755 (still in the word:
    token.c 38, file.c 561, processes.c 65-66 167-168 185, threads.c 360).
  - `slot_of` 141, `unref` 150 and `inherit_init` 711 (`inherited_ready`, the inherit table): under the table, held
    by their callers.
  - `object_of` 161 and `object_done` 170: hold the table. Callers off the word: GetThreadId 811, GetProcessId 819.
    In the word: Set/ResetEvent 410 424, ReleaseSemaphore 432, ReleaseMutex 448, `wait_one` 523, WaitForMultipleObjects
    585 (`objects_done` 572), `w2m_object_is_token` 181 (token.c 46), `w2m_object_is_open` 187 (file.c 352),
    `w2m_object_mapping_of` 196 (file.c 573), `w2m_object_kind` 664 (processes.c 73 93 111 191),
    `w2m_object_thread_of` 756 (threads.c 347, thread_queries.c 47).
  - The waits' `refs` (Sprints 350 and 352), in the word: `waiter_drop` 307 (through `object_done`), `waiters_drop`
    314 and `all_drop` 371 hold the table around their drops; `waiter_add` 303 and a wait-all's entry keep the lookup's
    reference and change no count.
  - `signaled` 470: reads a SHARED_EVENT's or PROCESS's `handles` under the table.
  - CloseHandle (file.c 264) through `w2m_object_close` 211, in the word: the slot freed and both counts dropped under
    the table, the kind and id read there; the host's closes after it is let go.
  - `w2m_object_flags` 676, the pointer's defect: now `u32 w2m_object_flags(u64 h, u32 mask, u32 values)`, the flags
    before the change and the mask's bits made `values`', both under the table; file.c 365 and 378 call it, processes.c
    46 declares it.
  - CreateProcessW's read of every slot (processes.c 144): `w2m_object_inheritable` 686 holds the table over the scan.
  - Not the table's: `entries_of`, `grant`, `grant_all`, `has_grant` (indices and the sync objects' state, the word's
    until card 5).
  - **The further reader** (reported to main): DuplicateHandle of a source that names no object read `files[]`
    (`w2m_is_file_handle`, file.c 108) and the standard handles (`w2m_is_std_handle`, console.c 122-125), both the
    word's; it now takes the word around them (786).
- **The order.** A named event's EVENT_OPEN (250-252) and DuplicateHandle's look (786-788) take the word in the C, no
  other lock held, through `w2m_take_runtime_word` (runtime.c 78), which takes it as `hold_runtime_word` does
  (compare-and-swap of 0 for this vCPU's LOCK_OWNER, acquire) and answers 1 when this call took it, and
  `w2m_let_runtime_word_go` (87), a store of 0 with release when it did. Not `hold_runtime_word` itself: it never lets go
  (its caller's run ends at the request), and a traced bridge holds the word for any class, so the call must know
  whether it took it.
- **`services.rs`:** `Lock::Handles` and `HANDLES_LOCK`, the ten; `the_ten_services_of_lock_class_handles_are_the_handle_tables`
  holds the ten, each KERNEL32's, defined in sync_objects.c, of class handles in clock modes 0, 1 and 2, and 21 other
  callers (CloseHandle, the waits and releases, Get/SetHandleInformation, CreateFileMappingW, MapViewOfFile, the
  process, pipe and thread services, GetThreadPriority, ADVAPI32's OpenProcessToken and GetTokenInformation) on the
  word. Sprint 342's test held CreateEventW on the word; its example is now CloseHandle. `guest.rs`'
  `a_bridge_of_lock_class_none_takes_no_runtime_word` holds a handles bridge to class none's, word for word.
- **The built lock** (image `build/guest_runtime_rust/a07de64e….bin`, `_handles_lock` at 0x7f00044a8508):
  `_k32_GetThreadId` inline "`ldr w9, [x21, #0x508]; cbnz w9, 0x7f00041867f4; casa w9, w8, [x20]; cbnz w9,
  0x7f00041867f4`" at ...67f4-...6800, `bl _inherit_init` at ...6804, the reference "`ldur w8, [x22, #-0x8]; add w8,
  w8, #0x1; stur w8, [x22, #-0x8]`", "`stlr wzr, [x20]`" at ...6848 before `bl _w2m_unimplemented` at ...6860; the
  drop's `casa w9, w8, [x20]` at ...6874, "`subs w8, w8, #0x1; str w8, [x22, #0x18]`", `stlr wzr, [x20]` at ...6890; a
  handle naming no object, `stlr wzr, [x20]` at ...6898 before `bl _w2m_unimplemented` at ...68a4. The image holds 56
  `casa`; plant 1's, 17.
- **The judge** (`tests/data/timed_waits/handle_values.c`, built `x86_64-w64-mingw32-gcc -O1 -nostdlib -Wl,-e,start
  ... -lkernel32`, GCC 16.2.0; `run_loop.rs`
  `two_vcpus_creating_duplicating_and_closing_handles_keep_every_value_distinct_and_every_slot`; held line in
  `first-stop-2026-10-28.tsv`). The design's rounds (two threads, 10,000 creates, 10,000 duplicates by turns of two
  unnamed events, a named event, a semaphore and an auto-reset event, every value claimed while open, every close
  counted, 256 creates at the end), then bursts of 16 duplicates and zero-timeout waits, then a drops phase: thread 1
  calls GetThreadId on duplicates of the main thread's and an ended thread's handles while thread 2 drops references to
  the same two objects through 4,000 zero-timeout waits and 2,000 rounds of a granted wait-any and wait-all. A third
  thread, until both workers end, waits on the semaphore and the auto-reset event (1 ms, blocked; all of both, and the
  semaphore alone), releases and sets them and takes both, waits on the named event, and makes pipes whose read end's
  flags it sets and reads back (Get/SetHandleInformation). Strengthened from the design's form, which caught plant 1 in
  20 of 20 but let plant 3 on `object_done`, `waiters_drop` and `all_drop` pass 20 of 20 (their windows are one plain
  load, subtract and store against the only writer off the word); the third thread is the Supervisor's request.
  - Reach on today's code (HEAD's four C files and `services.rs` put in, restored by `cp`, checked by `cmp`): 21 of 21,
    every run "done", overlap 10,292 to 10,391 rounds of 11,000, the third thread 211 to 214 rounds.
  - The change, 20 runs on two vCPUs: every run ExitProcess { code: 0 } and "done"; 104,064 values claimed while open
    (0x19680), none held by both threads at once; 104,332 closes TRUE (0x1978c), none FALSE; no wait or duplicate
    unlike its object; 256 made; the final create's value 0x2000, the first create's. Overlap 10,054 to 10,191 rounds,
    the third thread 205 to 210.
- **The ten services' reads and writes outside items 1-4** (none takes the word from its bridge; `w2m_teb()` reads the
  state block's gs_base, runtime.h 39, and each vCPU maps its own state block, guest.rs 1217-1222, so a TEB is the
  calling thread's):
  Every one reaches `inherit_init` through the table's lock, so the inherit table (item 9) is read under it, written by
  the host before the guest's first instruction; `w2m_logged`'s call ring is written by atomic add (Sprint 341);
  `w2m_unimplemented` takes the word itself (Sprint 342); last error is the calling thread's TEB. None reads `info[]`
  or the run table. One line a service:
  1. CreateSemaphoreExW 234: the flags at RSP + 0x28 (its caller's stack, no lock); last error 0; the ring.
  2. CreateSemaphoreW 827: last error 0 or 87; the ring. No other read or write.
  3. CreateMutexA 273: last error 0; the ring. No other read or write.
  4. CreateEventW 258: a name's UTF-16 text (the caller's memory, no lock); a named event's EVENT_OPEN under the word
     taken in the C (250-252), no other lock held; last error 0 or 183; the ring.
  5. CreateEventA 262: as CreateEventW, an ANSI name stopping the run before any host request (`w2m_unimplemented`).
  6. CreateEventExW 266: as CreateEventW.
  7. DuplicateHandle 766: its stack arguments and `*lpTargetHandle` (the caller's); a pseudo-handle's current tid and
     pid (TEB 0x48 and 0x40, written by the host before the thread's first instruction, process.rs 376-377, threads.rs
     1335-1336; read by `current_tid`/`current_pid`, as GetCurrentThreadId and GetCurrentProcessId read them with no
     lock); a source naming no object, `files[]` and the standard handles, under the word taken in the C (786-788, the
     further reader); last error; the ring.
  8. OpenThread 801: the current tid (TEB, written once), compared with the argument; no run table entry is read; last
     error 87 on another id; the ring.
  9. GetThreadId 811: the pseudo-handle's current tid (TEB), or a THREAD object's `count`, its tid, written before its
     handle was published under the lock and read holding a reference; the ring.
  10. GetProcessId 819: as GetThreadId, the pid (TEB 0x40, or a PROCESS object's `count`).
- **Plant 1** "handle_values: 20 of 21 runs failed (one on one vCPU, 20 on two)": `hold_handles` returning at once,
  the ten services still in `HANDLES_LOCK`. Run 1: "stop HostRequest { op: 4, ..., why: \"ReleaseSemaphore argument
  form 0x2018\" }, output \"\", claimed 0xc84, collided 0x51, closed 0xcc0, refused 0x12, wrong 0x74": 81 values held by
  both threads at once, 18 closes FALSE, a semaphore's handle naming another kind. 20 of 20 on two vCPUs, the one-vCPU
  run "done". (On the guest before its third thread: 20 of 20, "CreateEventA argument form 0x100", collided 0x467.)
- **Plant 2** "assertion `left == right` failed: the services of lock class handles": `"GetThreadId"` renamed
  `"GetThreadId taken out"` in `HANDLES_LOCK`; left lists nine services, right ten, GetThreadId missing.
- **Plant 3** "handle_values: 20 of 21 runs failed (one on one vCPU, 20 on two)", for each holding its lock removed in
  turn (`hold_handles` and `let_handles_go` made `(void)0;` in that function), each 20 of 20 on two vCPUs:
  - `publish` (124, 129): "why: \"CreateEventA argument form 0x100\"", collided 0x1bc, refused 0xa7.
  - `object_of` (162, 166): "why: \"WaitForSingleObject argument form 0xa\"", wrong 0x1a6.
  - `object_done` (172, 174): "why: \"CreateEventA argument form 0x100\"", made 0xff: an entry leaked.
  - `w2m_object_close` (212, 215, 224): "CreateEventA argument form 0x100", made 0xfb, wrong 0x1d3.
  - `waiters_drop` (316, 320) and `all_drop` (372, 374): "CreateEventA argument form 0x100", made 0xff.
  - DuplicateHandle's (778, 783): "why: \"CreatePipe argument form 0x100\"", collided 0x3e1, refused 0x16a.
  - Three the third thread runs every round and no run can make observable, each 20 of 20 "done" with its lock removed
    (the third thread waits on the named event and sets and reads a pipe's flags 205 to 214 times a run, and the
    workers duplicate the named event every fifth round): `signaled`'s read of `handles` (472, 474) only
    tests it for 0 on a SHARED_EVENT or PROCESS its caller holds a reference to; the one writer off the word,
    `handle_for`, reaches an existing object only through DuplicateHandle of a live handle (1 or more already), and the
    one decrement, CloseHandle, runs in the word the reader holds. `w2m_object_flags` (677, 681): the slot a live handle
    names is written only by Get/SetHandleInformation and freed only by CloseHandle, all in the word; the writers off
    the word write a free slot's flags. `w2m_object_inheritable` (688, 692, 697, 705): only CreateProcessW scans, and on
    two vCPUs its spawn is refused at the PROCESS_CREATE request after the scan (`MACHINE_ON_ONE`, host_request.rs
    532-533; run_loop.rs 993-1011), so no run on two vCPUs uses the scan, and on one no other vCPU writes the table.
  - Each plant restored by `cp` and checked by `cmp`.
- **One vCPU and the existing runs:** the gates' batch run, family runs on one and two vCPUs, exit ledger,
  `wait_handoff`, `wait_closed_handle`, `wait_closed_kinds`, `three_counters`, `lost_wakeup` and the python image's
  departures pass; 402 tests run, 402 passed, 1 skipped (the hypervisor crate's ignored test), in each of two gates runs.

exit 0 in 414 s	default	2026-11-26T16:23:38Z	sprint 355	ee2180a63073bd95576bd1e3cc99bb066929a266	f1871709b2ff1f30030a7796098ca36ffef3d25eca81f9d880611c1e251a383e	root=3bcdca5f220454d1d04b39027fb61161a3bf948d	rust=all python=0 samples=0	base=2e180c096623124ceb8d9fe24dfc7b50e714acfc
