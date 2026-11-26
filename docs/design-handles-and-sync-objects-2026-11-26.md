# The handle table's lock and the sync objects' lock in the guest runtime (Sprint 350)

Change 1 of the host design (`docs/design-rust-host-2026-10-24.md` 95-140) names two lock classes that are not built
yet. One is "the handle table's lock, held to take, free or look up one slot". The other is "a lock per sync object
(event, semaphore, mutant, critical section's slow path, timer), held to change its state and pick a waiter; readying
the waiter then takes the run-table word". This document maps the runtime C those classes must cover, as it stands at
23b860e79. It decides which lock guards which state, and how an object outlives a handle closed while another vCPU
uses it. It places each service's class in `crates/guest_runtime/src/services.rs`, and lists the cards and judges that
build it. Its line numbers are those of 23b860e79; the runtime change, the defect's fix, is named below.

## The state, as it is

Today the runtime word guards every item below: each service bridge takes it around the whole call into the C
(`crates/w2m_vm/src/guest.rs` 601-614). The map has 17 items. Eight are arrays and words of
`tools/guest_runtime/sync_objects.c`, one is a host-written input it reads, and eight are state of other files that a
handle service or a wait reaches.

| # | State | Where | What it holds; who writes it |
|---|---|---|---|
| 1 | `objects[256]`, `{kind, count, max, refs}` | sync_objects.c 45 | One entry per object. `kind` FREE marks a free entry (35), `new_object` takes the first FREE (69-72), and `w2m_object_close` sets FREE when `refs` reaches 0 (119-124). `count` and `max` are a semaphore's count and maximum, an event's 0/1, a mutex's recursion and owner tid (257, 345), and are changed by releases, `take` and `take_for` (349-358, 260-265). For THREAD, PROCESS, TOKEN, MAPPING, SHARED_EVENT, PIPE_READ, PIPE_WRITE and REMOTE_THREAD, `count` is an id or a descriptor and `max` a mapping's size, both written once at creation. `refs` counts the entry's handles (61, 119, 524). |
| 2 | `handle_object[256]` | sync_objects.c 46 | Handle slot to object index + 1, 0 when free. Written by `handle_for` (59), `w2m_object_close` (118) and `inherit_init` (522). Handle value = 0x2000 + 4 * slot (22, 62). |
| 3 | `handle_flags[256]` | sync_objects.c 47 | HANDLE_FLAG_INHERIT and PROTECT_FROM_CLOSE per slot. Written by `handle_for` (60), `w2m_object_new` (484), `inherit_init` (523), and through the pointer `w2m_object_flags` returns (493-496) by file.c's SetHandleInformation (376-377). |
| 4 | `inherited_ready` | sync_objects.c 48 | The one-time install of a parent's handles (513-526), called at the top of `handle_for` (56) and `slot_of` (85). |
| 5 | `waiters[256]`, `{tid, object, granted, slot}` | sync_objects.c 183 | Threads blocked on a semaphore, auto-reset event or mutex, in wait order. Entered by `wait_one` (394) and WaitForMultipleObjects-any (470). `grant` marks one granted (227). Dropped by `waiter_drop` and `waiters_drop` (192-202). `object` is an index into item 1. |
| 6 | `nwaiters` | sync_objects.c 184 | Item 5's length. |
| 7 | `all_waiters[16]`, `{tid, n, granted, objects[64]}` | sync_objects.c 238 | Blocked waits for all of several objects, entered at 436-438. `grant_all` takes every object of the first satisfiable entry and readies it (267-278). |
| 8 | `nall` | sync_objects.c 239 | Item 7's length. |
| 9 | The inherit table at `W2M_INHERIT_VA` | sync_objects.c 516 | Count, three standard handles, then (handle, kind, id) triples. Written by the host before the guest's first instruction; only read here. |
| 10 | Each thread's TEB scratch `wait_deadline` | runtime.h 47 | The counter value a blocked wait ends at, ~0 for never. Set by `w2m_block` (threads.c 104), cleared by each wait's success path (sync_objects.c 387, 402, 428, 446, 455, 461) and by `timed_out` (370). Read by the switch on any vCPU (threads.c 269). |
| 11 | Each thread's TEB scratch `wait_key` | runtime.h 55 | What a blocked thread waits on: `(u64)o` for one object (391), 0 for several (433, 465). Written by `w2m_block` (threads.c 105), read by `w2m_signal` on any vCPU (136). |
| 12 | Each thread's TEB `last_error_value` | sync_objects.c 50 | Its own last error; only its own thread writes it. |
| 13 | Run-table entries' `status` and `deadline`, and the head's `blocked` and `waiting` | threads.c 38-56 | Set by `w2m_block` (111-115), `w2m_ready_thread` (150-159) and `w2m_signal` (130-144). `readied` (124-127) makes the host request `W2M_THREAD_WAKE` when a vCPU waits in the host. |
| 14 | `w2m_wake_census` | threads.c 86 | Counters written by `w2m_block` (95-101), `w2m_signal` (139) and `w2m_ready_thread` (155). |
| 15 | `files[64]`, the file handles | file.c 95 | A table of its own: handle value 0x1000 + 4 * slot (79). CloseHandle (260-271) looks a handle up here before item 2. |
| 16 | `info[256]`, priorities and descriptions by tid | threads.c 414 | Reached through a thread handle (`thread_of` 303-308). The descriptions are heap blocks (`copy_string` 422-429). |
| 17 | `list[69][3]` | processes.c 143 | A static CreateProcessW fills from item 2 (`w2m_object_inheritable` 497-511). |

Two more files hold sync objects keyed by guest address, not by handle, and so lie outside this table.
`critical_section.c` changes a CRITICAL_SECTION's fields in guest memory (`acquire` 72-84, LeaveCriticalSection
102-112) and blocks on its address (90, 109). `slim_locks.c` keeps `variables[]` for condition variables (91). Change 1's
"critical section's slow path" is theirs. Their class is left to a design of their own (Revisit if).

One host constraint shapes every class below. The transport page is shared by every vCPU, so a host request (`hvc #8`,
`w2m_host`, runtime.c 26-36) needs the runtime word ("the transport page is every vCPU's", runtime.c 60-62).
`w2m_unimplemented` takes the word itself (`hold_runtime_word`, runtime.c 63-77). `w2m_logged` takes no lock: its
records are reserved by atomic add (runtime.c 79-94).

## The services, as they are

There are 43 services across 8 files that take, free, look up or change a handle or a sync object. Every one takes
the runtime word today (`Lock::RuntimeWord`: none is in `NO_LOCK`, `HEAP_LOCK` or `SLOTS_LOCK`, services.rs 40-73). "Host"
means the call makes a host request on some path.

| Service | File:line | Handle table | Sync state | Host |
|---|---|---|---|---|
| CreateSemaphoreExW | sync_objects.c 129 | take a slot, a new object (`make` 78) | creates count/max | no |
| CreateSemaphoreW | sync_objects.c 611 | the same | the same | no |
| CreateMutexA | sync_objects.c 166 | the same | creates | no |
| CreateEventW, CreateEventA, CreateEventExW | sync_objects.c 151, 155, 159 | the same (`create_event` 138) | creates | a named event: `W2M_KERNEL_EVENT_OPEN` (145) |
| SetEvent | sync_objects.c 286 | look up | count = 1, `grant`, `grant_all`; `w2m_signal` for a manual-reset event (296) | a shared event: EVENT_SET (288) |
| ResetEvent | sync_objects.c 299 | look up | count = 0 | a shared event: EVENT_RESET (301) |
| ReleaseSemaphore | sync_objects.c 306 | look up | count, `grant`, `grant_all` | no |
| ReleaseMutex | sync_objects.c 320 | look up | count, owner, `grant`, `grant_all` | no |
| WaitForSingleObject, WaitForSingleObjectEx | sync_objects.c 407, 409 | look up (`wait_one` 377) | `take`, items 5, 6, 10, 11, 13 | THREAD (`w2m_thread_ended`, threads.c 347), PROCESS (342), SHARED_EVENT (343, 356) |
| WaitForMultipleObjects | sync_objects.c 414 | look up each of n | `take` per object, items 5-8, 10, 11, 13 | as above, per object |
| WaitForMultipleObjectsEx | processes.c 199 | calls the above | the same | the same |
| DuplicateHandle | sync_objects.c 560 | take a slot; for a pseudo-handle a new object (568-569) | none | no |
| OpenThread | sync_objects.c 587 | take a slot, a new THREAD object | none | no |
| GetThreadId, GetProcessId | sync_objects.c 597, 604 | look up | none (reads an id) | no |
| CloseHandle | file.c 260 | free a slot, `refs`, the entry at 0 (`w2m_object_close` 114-127) | none | a mapping's FILE_CLOSE (120), KERNEL_CLOSE (123); a file's FILE_CLOSE (268) |
| GetHandleInformation, SetHandleInformation | file.c 361, 374 | look up, read or change item 3 | none | no |
| GetFileSize, GetFileInformationByHandleEx | file.c 389, 427 | look up on the error path (`file_or_stop` 350-355) | none | yes, for a file |
| CreateFileMappingW | file.c 534 | take a slot, a MAPPING (560) | none | FILE_SIZE, FILE_DUP |
| MapViewOfFile | file.c 569 | look up (572) | none | maps the view |
| ReadFile | file.c 323 | a pipe's kind (`w2m_pipe_read`, processes.c 71-89) | items 10, 13 when it blocks | PIPE_READ |
| WriteFile | runtime.c 115 | a pipe's kind (`w2m_pipe_write`, processes.c 93-111) | `w2m_signal(0)` (98), items 10, 13 | PIPE_WRITE |
| GetFileType | console.c 60 | a pipe's kind (66) | none | no |
| CreatePipe | processes.c 55 | take two slots (65-66) | none | PIPE_CREATE |
| CreateProcessW | processes.c 130 | read every slot (144), take two (167-168) | none | PROCESS_CREATE |
| OpenProcess | processes.c 176 | take a slot (185) | none | PROCESS_STATE, REF |
| GetExitCodeProcess | processes.c 189 | look up (191) | none | PROCESS_STATE |
| CreateThread | threads.c 311 | take a slot (319) | none | THREAD_CREATE |
| GetExitCodeThread, SuspendThread, ResumeThread, GetThreadContext | threads.c 339, 354, 366, 377 | look up (`thread_of` 303) | none | THREAD_* |
| SetThreadPriority, SetThreadDescription, GetThreadDescription | threads.c 402, 429, 438 | look up | item 16; the heap for descriptions | no |
| GetThreadPriority | thread_queries.c 45 | look up (47) | item 16 | no |
| OpenProcessToken, GetTokenInformation (ADVAPI32) | token.c 34, 44 | take a slot; look up | none | no |

**How each wait blocks today.**
- **WaitForSingleObject(Ex) on an object that is not signaled.** `timed_out` (362-375) calls `w2m_block(deadline,
  (u64)o)`. That sets the thread BLOCKED in the run table under the word (threads.c 111-115) and asks the bridge to RETRY
  (`W2M_SCHEDULE_RETRY`, runtime.h 136-142). The bridge returns no value and re-enters the call when the thread runs
  again (guest.rs 624-627). The switch stub takes the runtime word itself (guest.rs 402).
- **A semaphore, auto-reset event or mutex** also enters `waiters[]` (392-395). A release grants it and readies that
  thread alone (`grant` 228). The retried call finds the grant and returns WAIT_OBJECT_0 (385-389).
- **A manual-reset event** wakes every thread blocked on its key (`w2m_signal`, 296).
- **A thread, a process or a shared event** is the host's. The waiter wakes at the host's signal, or at its deadline
  (`wake_expired`, threads.c 165-172; the warp, 266-277).
- **WaitForMultipleObjects** blocks on key 0, which any signal wakes (433, 465). Waiting for any, it enters a waiter per
  handed-over object with its index as `slot` (466-471). Waiting for all, it enters `all_waiters[]` (434-439).

## A defect found: a wait whose handle is closed leaves its waiter entry and deadline

The design rests on this case. It was reported to the Supervisor when found (2026-11-26).
- **What leaves state behind.** `wait_one` blocks on a semaphore, auto-reset event or mutex and enters `waiters[]`
  (394). If CloseHandle then frees the object (118-124), the retried call returns WAIT_FAILED at 379-383. That path
  neither drops the entry nor clears `wait_deadline`; every other return path does one or both (386-387, 398, 401-402).
- **(1) A stolen unit.** The entry names index X, and `new_object` gives X to the next object made (69-72). Once that
  object is released, `grant` hands it to the stale entry: a semaphore's count falls, or the mutex is owned by that tid
  (221-222). No wait returns either. A thread waiting INFINITE on the new object then never wakes.
- **(2) A false WAIT_OBJECT_0.** The stale thread's next `wait_one`, on any handle, finds the grant and returns
  WAIT_OBJECT_0 (385-389).
- **(3) An early WAIT_TIMEOUT.** Its next wait on an unsignaled object reads the stale deadline as passed and returns
  WAIT_TIMEOUT at once, INFINITE included (369-372). With the entry still present, it does not register on the new
  object either (392).

Windows calls the trigger undefined: "If this handle is closed while the wait is still pending, the function's
behavior is undefined" (Microsoft Learn, WaitForSingleObject,
https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitforsingleobject; WaitForMultipleObjects
says the same of "one of these handles"). The lifetime rule below removes the cause: a waiter's entry holds a
reference, so a close never frees an object that has a waiter.

Sprint 350's amendment fixed it under the word, ahead of the locks:
- Each waiter entry holds a reference: `waiter_add`, and `refs++` for a wait-all's objects. `waiter_drop`,
  `waiters_drop` and `all_drop` let the references go through `unref`, which marks the entry FREE at none and makes no
  host request, as `w2m_object_close` now does.
- The handles are counted apart (`handles`). `w2m_object_close` makes the host's closes when they reach 0, as before.
- A retried WaitForSingleObject waits on its entry's object.
- The WAIT_FAILED path clears the deadline.
- `probes/wait_closed_handle` and its `run_loop.rs` test hold the main thread's defined row to Windows' on one vCPU and
  on two. Before the fix, w2m answered that row's zero-timeout wait with 258, WAIT_TIMEOUT, on both.
- The runner, Windows run 45797800625 (Server 2025 10.0.26100.33828, an Intel Xeon Platinum 8573C, three runs alike),
  answered `{"s":"main","closed":1,"made":1,"released":1,"previous":0,"wait":0,"ended":0}`. Its undefined worker row
  read `{"s":"worker","wait":258,"error":0}`, a timeout on the closed semaphore. w2m with the fix gives both rows.

## The design

### Which lock guards which state

- **The handle table's lock**, a word `handles_lock` in sync_objects.c's data, taken and let go in the form of
  `hold_slots` and `let_slots_go` (thread_storage.c 65-75): compare-and-swap of 0 for 1 with acquire, plain loads while
  another vCPU holds it, and a store of 0 with release. It guards items 2, 3 and 4, and in item 1 the `kind` of a FREE
  entry, the allocation of an entry (`new_object`) and both counts below. Item 9 is read only under it, in
  `inherit_init`.
- **The sync objects' lock**, `sync_lock`, in the same form. It guards the mutable state of semaphores, events and
  mutexes (item 1's `count` and `max` for those kinds) and items 5 to 8.
- **Fixed after creation.** An object's `kind`, and `count` and `max` for every other kind, are written once, before
  the handle that names the object is published under the handle table's lock. They are read with no lock by anyone
  holding a reference.
- **The runtime word** keeps items 10, 11, 13 and 14, as the run-table word, and every host request (above). It also
  keeps items 15, 16 and 17: the file table, the thread information and CreateProcessW's list stay with the services that
  stay on the word.
- **Item 12** is its own thread's and takes no lock.

The sync objects' state takes one lock, not one per object. The waiter lists are shared by every object (`grant`
scans all of `waiters[]` for its index, 216). A grant to a wait on several objects reads the thread's other entries
(`has_grant` 204-208, 218). And a release completes a wait for all, taking objects other than its own (`grant_all`
267-278). With a lock per object, a release of object A would take the lock of a wait-all's object B while holding A's,
in either index order. ReactOS sets an event and begins a wait under one lock for both, the dispatcher lock:
`KiAcquireDispatcherLock` (`ntoskrnl/include/internal/ke_x.h` 349-353, "acquire the dispatcher lock"), taken by
KeSetEvent (`ke/eventobj.c` 183, "Lock the Dispathcer Database") and by KeWaitForSingleObject (`ke/wait.c` 578), at
commit 3e6033a613fbf7c5bd58aaeaa05af42514af59d5
(https://github.com/reactos/reactos/blob/3e6033a613fbf7c5bd58aaeaa05af42514af59d5/ntoskrnl/ke/eventobj.c#L183). A lock
per object, Change 1's form, follows only when step 6 measures this one lock contended (Revisit if). By then the waiter
lists must be per object, and a wait-any's grant must be claimed per thread.

### How an object outlives a handle closed while another vCPU uses it

**The rule.** An entry of `objects[]` keeps two counts, as Windows and Wine do:
- `handles`: one per handle slot naming it, what `refs` counted until Sprint 350.
- `refs`: one per handle, one per lookup in flight, and one per object each blocked wait names.

Sprint 350 built both counts (reviewer-4's nineteenth review, Medium 1):
- `refs` counts the handles and the waiter entries of a semaphore, an auto-reset event, a mutex and a wait for all.
- `handles` counts the handles alone, and CloseHandle makes the host's closes when it reaches 0, as before the sprint.
- The lookup's reference is card 2's.

**Taking and dropping references.**
- A lookup (`object_of`) takes a reference under the handle table's lock and returns the pointer. The caller drops the
  reference under the same lock when it is done with the object.
- A wait that blocks keeps its lookups' references in its waiter entry. They are dropped when the wait ends, at
  success, timeout or failure. A blocked wait of any kind enters an entry, not only the handed-over kinds.
- A retried wait reads its objects from its entry, not by the handle again. So a handle closed meanwhile leaves the
  wait on the object.

**When counts reach 0.**
- CloseHandle frees the slot and drops one of each count. At `handles` 0 it makes the host's closes as today (120-123),
  under the word, after the handle table's lock is let go.
- The entry returns to FREE when `refs` reaches 0, never before, and that makes no host request. So an index is never
  reused while any vCPU holds a pointer to it, and defect (1) cannot occur.
- A wait for all may still hold a host-backed object, a shared event or another process, whose last handle is closed.
  The host's object is then closed, so the object's state is not asked of the host: it reads unsignaled. Asked, the
  host's event request stops the run on an id no longer an event ("which is not an event", `crates/w2m_vm/src/kernel.rs`
  209-211).
- Another process sees the close of a shared object at the same call as before, CloseHandle of the last handle.
  `tests/data/timed_waits/shared_close` holds it: the exit ledger's W2M_KERNEL_CLOSE row comes before the "closed"
  line's write.

**The grounds.**
- **Windows** documents the two counts and the rule that the second governs freeing:
  - "Each routine that opens a handle increases both the object reference count and the object handle count by
    one", "Once that counter falls to zero, the object is freed", and a temporary object "can still be accessed by
    pointer as long as their reference count remains greater than zero" (Microsoft Learn, Life Cycle of an Object,
    https://learn.microsoft.com/en-us/windows-hardware/drivers/kernel/life-cycle-of-an-object).
  - The lookup is a reference: "Incrementing this count prevents the object from being deleted while the pointer is
    being referenced. The caller must decrement the reference count with ObDereferenceObject as soon as it is done
    with the object" (ObReferenceObjectByHandle,
    https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-obreferenceobjectbyhandle).
  - "Closing a thread handle does not terminate the associated thread or remove the thread object" (CloseHandle,
    https://learn.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-closehandle). A THREAD entry holds only
    a tid, and the thread lives in the run table whatever its handles do.
- **ReactOS** waits holding a reference across the whole wait (ntoskrnl at commit
  3e6033a613fbf7c5bd58aaeaa05af42514af59d5):
  - NtWaitForSingleObject takes `ObReferenceObjectByHandle(ObjectHandle, SYNCHRONIZE, ...)` (`ob/obwait.c` 398), calls
    `KeWaitForSingleObject` (421) and only then `ObDereferenceObject(Object)` (437).
  - The reference is `InterlockedIncrementSizeT(&ObjectHeader->PointerCount)` taken while the handle entry is locked
    (`ob/obref.c` 633-672).
  - Closing a handle does `ExDestroyHandle`, then `ObpDecrementHandleCount`, then `ObDereferenceObject(Body)`
    (`ob/obhandle.c` 761-770).
  - https://github.com/reactos/reactos/blob/3e6033a613fbf7c5bd58aaeaa05af42514af59d5/ntoskrnl/ob/obwait.c#L398
- **Wine's server** gives a wait its own reference on each object (wine at commit
  274fcfa9691bf3b7f24c0833ad33e9d57c3df65f):
  - `entry->obj = grab_object( obj );` in `wait_on` (`server/thread.c` 1102), released in `end_wait` (1063).
  - A handle holds `grab_object_for_handle( obj )`, which raises `handle_count` and the reference count (`server/handle.c`
    232, 98-105).
  - `release_object` frees only at reference count 0, asserting "if the refcount is 0, nobody can be in the wait
    queue" (`server/object.c` 563-577).
  - https://gitlab.winehq.org/wine/wine/-/blob/274fcfa9691bf3b7f24c0833ad33e9d57c3df65f/server/thread.c#L1102

**What Windows returns** to a wait whose handle another thread closed is documented as undefined. The rule above
gives the wait the outcome it would have had: WAIT_OBJECT_0 at a release, or its timeout. `probes/wait_closed_handle`
records the worker's result on the runners without comparing it. The design is changed if a runner disagrees.

### The order, and the two leaves

The handle table's lock and the sync objects' lock are leaves: held, neither takes another lock, makes a host request
or readies a thread. The rule of Sprints 343 and 345 holds unchanged:
- `w2m_unimplemented` and `w2m_logged` are called only with both let go (heap.c 24-26, thread_storage.c 28-30).
- `w2m_unimplemented` takes the runtime word; held under a leaf, that would wait on a word whose holder may want the
  leaf.

So the two locks are never nested with each other:
- A wait looks up its handles under the table's lock and lets it go, takes the sync objects' lock for the objects'
  state, and lets that go.
- Only then does it take the run-table word to park. A release likewise grants under the sync objects' lock, lets it
  go, then readies under the word.
- References are dropped under the table's lock after the sync objects' lock is let go.

The order as built differs from Change 1's list (modules → handle table → TLS slots → a sync object → a heap → the
run-table word), which put the word last as a leaf:
- The runtime word is outermost. A service whose bridge holds it may take any lock under it, as LocalAlloc takes the
  heap's lock under the word (services.rs 50-55, heap.c 24-26).
- Then the TLS slots' lock, then the heap's: FlsSetValue's first set takes the heap's under the slots' (thread_storage.c
  27-29, 82).
- The handle table's lock and the sync objects' lock are leaves. Held, they take nothing, and they are never held
  together.
- A call holding neither leaf nor any other lock may take the word. So a wait or a release takes the word only after it
  has let its leaf go, never under it.

No cycle can form: every nesting runs from the word inward, and no leaf's holder waits for anything.

One `sync_lock` replaces Change 1's lock per sync object, until step 6 measures it busy (Revisit if). The grounds are
the paragraph above:
- The waiter lists are shared by every object, and a grant reads a thread's other entries.
- A release completes a wait for all over objects other than its own, which a lock per object could not take in one
  order.
- ReactOS serializes a set and a wait under one dispatcher lock.

### The wait across two locks

A release that falls between the waiter's letting go of the sync objects' lock and its parking must not be lost
(Change 1, 126-130). `w2m_ready_thread` readies only a BLOCKED thread (threads.c 153). In that window it would do
nothing, the waiter's `w2m_block` would then block it, and only its deadline would wake it. Under the one word today
the window does not exist.

The woken mark Change 1 asks for is a word of the thread's. Card 3's lookup places it in its run-table entry or its TEB
scratch, whichever the host's layout tests leave free (threads.c 46-49 hold the entry's offsets).
- `w2m_ready_thread`, under the word, sets it on a thread that is not BLOCKED.
- `w2m_block`, under the word, finds it set, clears it, and leaves the thread READY. The switch then retries the call
  at once (`retry` needs BLOCKED, threads.c 251), and the retried wait finds its grant.

Both halves run under the run-table word, so the mark needs no ordering of its own.

### Where each service's class lands

`Lock` (services.rs 27-32) gains two variants. For each, the bridge holds nothing (guest.rs 601-614), as for `Heap`
and `Slots`, and the C takes its locks.
- **`Lock::Handles`, a list `HANDLES_LOCK`:** CreateSemaphoreExW, CreateSemaphoreW, CreateMutexA, CreateEventW,
  CreateEventA, CreateEventExW, DuplicateHandle, OpenThread, GetThreadId, GetProcessId.
  - None reaches the sync objects' state: a create initializes its object before the handle is published.
  - A named event's EVENT_OPEN (145) is made by taking the runtime word in the C with no other lock held, and letting
    it go after.
- **`Lock::SyncObjects`, a list `SYNC_LOCK`:** SetEvent, ResetEvent, ReleaseSemaphore, ReleaseMutex,
  WaitForSingleObject, WaitForSingleObjectEx, WaitForMultipleObjects, WaitForMultipleObjectsEx.
  - They take the table's lock to look up, the sync objects' lock for the state, and the word to park, to ready and
    for the host-backed kinds' requests (THREAD, PROCESS, SHARED_EVENT).
  - A wait naming a host-backed object reads those objects' state under the word before it takes the sync objects'
    lock for the others. A thread's or a process's end is final, so the order of the two readings cannot hide a
    signal. A shared event can change between them, as it can today from the other process's VM, which takes no lock
    of this one.
- **Every other service in the table above stays `Lock::RuntimeWord`** and takes the table's lock inside, as a leaf
  under the word:
  - The file services, for `files[]`.
  - The process, pipe and thread services, which make host requests.
  - GetThreadPriority and the description calls, for `info[]`.
  - The ADVAPI32 token calls, whose DLL no list of `services.rs` names.
  - CloseHandle among them: its file path needs the word, and its object path frees under the table's lock inside.

## The judges each card needs

The races below need at least two vCPUs. Each is a guest program built as `tests/data/timed_waits`' guests are
(`crosswake.c`, `handshake.c` beside them), run on two vCPUs as the multi-vCPU tests run (`store/gates/multi-vcpu.tsv`).
Each names a plant that a run must catch.
- **The close during a wait.**
  - `probes/wait_closed_handle` (Sprint 350): a worker waits 1000 ms on a semaphore; the main thread closes its only
    handle, makes a new semaphore, releases it once and waits on it with a zero timeout.
  - The main thread's row is defined and held to Windows' on one vCPU and on two. The worker's is recorded, not
    compared.
  - Plant: the waiter's reference not taken. The main thread's wait then answers 258.
  - Card 2 adds the other kinds and a wait on several objects. Its Windows row: a handle closed on one thread while
    another waits on its event, then SetEvent through a duplicate handle, with the wait's return and last error
    recorded on the runners. The documentation does not settle it.
- **Handles from two vCPUs.**
  - Guest: two threads each create and close 10,000 unnamed events, and each DuplicateHandle 10,000 times.
  - Every value open at once is distinct and every close answers TRUE. No slot is lost: a further create at the end
    answers the value the first create answered.
  - Plant: `handles_lock` taken nowhere. The run must show a duplicate value or a failed close.
- **A semaphore's units across vCPUs.**
  - Guest: four threads release and wait on one semaphore (maximum 1,000) 100,000 times.
  - The final count equals the releases less the satisfied waits, and every wait returns.
  - Plant: `sync_lock` taken nowhere. The count must disagree.
- **The lost wakeup.**
  - Guest: two threads ping-pong two auto-reset events 100,000 rounds with INFINITE waits.
  - The run ends inside its instruction limit.
  - Plant: the woken mark ignored in `w2m_block`. The run must hang to its limit and stop as a deadlock.
- **Each card's existing judges stay green.** The batch run conforming (`batch_run.rs`), `probes/wait_handoff`'s rows
  on one and on two vCPUs, the `contracts/windows.sync.objects` rows, and `services.rs`'s class tests extended with the
  new lists.

## The cards that follow

Each card is one concept, at the kit's two-file sweet spot.

1. **The wait's reference.** Done in Sprint 350's amendment (the defect above). The rest of the rule follows in card 2.
2. **The lookup's reference.**
   - The two counts, `handles` and `refs` in `struct object`, were built in Sprint 350 (above). Here `object_of` takes
     a reference and every caller drops it.
   - A blocked wait of any kind keeps its references in its entry, not only a semaphore's, an auto-reset event's or a
     mutex's. Its retry, a wait on several objects among them, reads them there.
   - Still under the word.
   - Files: sync_objects.c, and the callers' drops in file.c and processes.c (one concept, three files: named here as
     the exception).
   - Gate: a close-during-a-wait guest on a manual-reset event and on a wait for several objects, on two vCPUs, and
     the batch run conforming.
3. **The woken mark.**
   - `w2m_ready_thread` and `w2m_block` in threads.c, under the word.
   - Gate: the batch run conforming, `wait_handoff` unchanged, and the lost-wakeup guest with its plant (the race
     reaches the mark only once card 5 takes waits off the word, so the plant is run again there).
4. **The handle table's lock.**
   - `handles_lock` in sync_objects.c; `Lock::Handles` and `HANDLES_LOCK` in services.rs, with its class test.
   - Gate: the two-vCPU handles guest and its plant.
5. **The sync objects' lock.**
   - `sync_lock` in sync_objects.c, with the C's own take and release of the runtime word around parking, readying and
     host requests; `Lock::SyncObjects` and `SYNC_LOCK` in services.rs.
   - Gate: the semaphore-units guest, the lost-wakeup guest, `wait_handoff` on two vCPUs, and both plants.

## Against the goal

The goal is play at speed:
- about 37,600 system calls a second in play;
- 41 million instructions a frame at 112 frames a second, on six threads;
- the first scene, 325.4 billion instructions in 62 s on the laptop (`docs/rust-port-plan-2026-10-24.md` 32-40).

The waits and releases are the calls this design moves. Change 1 counts at least 278 scheduler calls a frame, about
31,000 a second at 113 frames (`docs/design-rust-host-2026-10-24.md` 132-135). Each takes the one runtime word around
the whole call today, so six threads waiting and releasing serialize on it with every other service.

After cards 4 and 5:
- A wait or a release holds the word only to park, to ready or to make a host request.
- It holds the sync objects' lock for a few loads and stores, and the handle table's lock for one slot.
- A create, a duplicate or an id lookup never takes the word.

The first scene's threads are the batch run's: its workers block on semaphores and auto-reset events (the Background
Job.Worker threads, sync_objects.c 176-178). The lifetime rule is what lets those calls leave the word without a close
on one vCPU freeing an object another vCPU is reading.

## Against the systems that solved it

- **Wine** gives every handle and every pending wait a reference to the object, and frees an object only at reference
  count 0 (`server/handle.c` 232, `server/thread.c` 1102 and 1063, `server/object.c` 563-577,
  https://gitlab.winehq.org/wine/wine/-/blob/274fcfa9691bf3b7f24c0833ad33e9d57c3df65f/server/object.c#L563).
  - Taken: the two counts and the wait's own reference.
  - Not taken: Wine's server runs one thread and so needs no lock for them; w2m's runtime runs on every vCPU, hence
    the two leaves.
- **Firecracker** runs "one or more vCPU threads (one per guest CPU core)"
  (https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md, as the plan quotes it).
  - Taken: each vCPU enters the runtime's C on its own, which is why a lookup's pointer must outlive another vCPU's
    close.
- **crosvm** locks each device, not the machine: "Each `BusDevice` is reference counted and wrapped in a mutex"
  (https://github.com/google/crosvm/blob/main/ARCHITECTURE.md, as the plan quotes it).
  - Taken: a reference count with a lock of the thing's own. Here that is the object table's count under the handle
    table's lock, not the machine-wide word.
- **QEMU** looks translations up without a global lock: lookups go through QHT, "designed for lockless lookups"
  (https://gitlab.com/qemu-project/qemu/-/blob/master/docs/devel/multi-thread-tcg.rst, as the plan quotes it).
  - Not taken for the handle table: a lookup here also takes a reference, which needs the table's lock or an atomic
    on a count that a free can race. A lock held for one slot is the simpler form to judge at step 6.
- **Rosetta 2** translates ahead of time and compiles what it meets at run time ("If Rosetta newly encounters code
  that haven't been translated at install time, then we'll compile it on the fly", Apple, WWDC20 session 10686,
  https://developer.apple.com/videos/play/wwdc2020/10686/, as the plan quotes it).
  - What the plan quotes of it is about code. Nothing of it bears on a handle table, and nothing is taken.
- **FEX** keeps the code its JIT makes on disk and looks there first (https://fex-emu.com/FEX-2609/, as the plan quotes
  it).
  - The same holds: the quoted source is about code, and nothing of FEX's is taken here.

## Revisit if

- Step 6 measures the sync objects' lock busy more than 5% of the time, or a p99 wait for it above 10 µs, the
  bounds Change 1 sets for the run-table word (136-139). Then a lock per object follows, Change 1's form, with waiter
  lists per object and a wait-any's grant claimed per thread.
- `probes/wait_closed_handle`'s worker row, or card 2's, shows Windows answering a wait whose handle was closed otherwise than the rule gives (WAIT_FAILED, or
  no return at the release). Then the retry reads the handle again, as Windows does, and the reference stays for the
  object's lifetime alone.
- A trace shows the game calling EnterCriticalSection, LeaveCriticalSection, the SRW locks or the condition variables
  from several vCPUs while they stay on the word. Then their own design follows, keyed by guest address.
