# Sprint 350 — A design for the handle table's lock and the sync objects' locks in the guest runtime

```yaml
---
id: 350
status: done
phase: 6
pass_kind: docs
---
```

## scope

The host design's Change 1 (`docs/design-rust-host-2026-10-24.md` 95-140) names two lock classes still to build:
- **the handle table's lock**, "held to take, free or look up one slot";
- **a lock per sync object** (event, semaphore, mutant, critical section's slow path, timer), "held to change its state
  and pick a waiter; readying the waiter then takes the run-table word".

It also gives the guest's lock order and the protocol for a wait across two locks.

The design does not say how these fit the runtime C as it stands. `tools/guest_runtime/sync_objects.c` keeps the
objects' state, `objects[]` with `kind`, `count`, `max` and `refs`, beside the handle table, `handle_object[]` and
`handle_flags[]`, in one file. `object_of` returns a pointer into `objects[]` that callers change after the lookup.
Six files reach the table:
- `file.c`, `processes.c`, `threads.c`, `thread_queries.c` and `token.c`;
- `sync_objects.c` itself.

Splitting the two locks raises a question the design leaves open: what keeps an object alive while one vCPU waits on
it, or changes it, and another closes its last handle? On Windows an object lives while any handle or reference holds
it. Microsoft Learn's CloseHandle reads: "Closing a thread handle does not terminate the associated thread or remove
the thread object". Quote the pages you rely on.

This card writes the design, and no code. Its output is `docs/design-handles-and-sync-objects-2026-11-26.md`, which
`target/release/design_check` passes. It holds:
- **Against the goal** and **Against the systems that solved it**, as `design_check` requires (`crates/design_check`).
  The systems are how Wine's server and ReactOS keep a handle table and object references (their sources), and
  what Windows documents of handle and object lifetime.
- **The state, as it is.** Every array and word in `sync_objects.c` and the files above that the table or an object
  touches, each with its line.
- **The services, as they are.** Every kernel32 service that takes, frees, looks up or changes a handle or a sync
  object, with its file and lines, and what shared state each touches. The waits are among them
  (WaitForSingleObject, WaitForMultipleObjects and their Ex forms in `sync_objects.c` and `processes.c`), with how
  each blocks today.
- **The design.**
  - Which lock guards which state.
  - How an object's lifetime is kept across the two locks, whether by a reference the lookup takes and the user drops
    or by another rule, with its grounds.
  - Where each service's lock class lands in the service table (`crates/guest_runtime/src/services.rs`, `Lock`).
  - The order against the slots' and heap's locks and the run-table word.
  - Whether `w2m_unimplemented` and `w2m_logged` stay outside every lock, as in Sprints 343 and 345.
- **The judges each implementation card will need.** For each, the guest program and the two-vCPU race it must show
  safe, and the Windows rows where a probe is needed, such as a handle closed on one thread while another waits on
  its object.
- **The cards that follow**, in order, each one concept at the kit's two-file sweet spot.

The design must also name any defect it finds that can stop a run or leave a thread past its deadline. Under the
user's Decision of 2026-10-18 such a defect is fixed in this card's sprint, not deferred. Report it at once, with its
evidence, and stop for the Supervisor to amend the card.

## prerequisites

- 349 (closed)

## context_files

- `docs/design-rust-host-2026-10-24.md` 95-140 (Change 1, its classes, its order, the wait across two locks).
- `tools/guest_runtime/sync_objects.c` whole (619 lines), and the handle and object calls in `file.c`, `processes.c`,
  `threads.c`, `thread_queries.c` and `token.c` (`grep -n 'object_of\|handle_for\|w2m_object_' tools/guest_runtime/*.c`).
- `tools/guest_runtime/thread_storage.c` and `tools/guest_runtime/heap.c`: the locks of Sprints 343 and 345, their form
  and their rule on `w2m_unimplemented` and `w2m_logged`.
- `crates/guest_runtime/src/services.rs` (`Lock`, the classes so far).
- `crates/design_check/src/main.rs` (what a design must state) and one design that passes it, such as
  `docs/design-translation-cache-2026-10-30.md`.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None; a design document.

### Invariants

- No source file changes.

## artifact contract

### Files created

- `docs/design-handles-and-sync-objects-2026-11-26.md`

### Command exit codes

- `target/release/design_check` returns 0, naming the new design among those it checked.
- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The state and the services, each with file and line, counted.
  - The lifetime rule and its grounds, quoting Windows' documentation and one system's source.
  - The cards that follow.
- **Planted faults:**
  - (1) The design's "Against the systems that solved it" section removed. `design_check` fails, naming the file.

  Before writing, read the judge's reach: `design_check` on a scratch copy of a passing design with that section
  removed.

## done criteria

A design, passing `design_check`, says which lock guards each piece of the handle table's and the sync objects' state,
how an object outlives a handle closed while another vCPU uses it, where each service's class lands, and the cards and
judges that build it.

## self-review checklist

- [x] Scope names one verifiable property: a design that passes `design_check` and answers the lifetime question.
- [x] context_files cite Change 1, the runtime C, the earlier locks, the service table and `design_check`.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the counts, the lifetime rule, the cards; one numbered plant, its reach read.
- [x] Sweet spot: one document, one concept, the split of two lock classes.

## amendment (2026-11-26)

Builder-350's reading found a defect that can leave a thread past its deadline, so it is fixed in this sprint (the
user's Decision of 2026-10-18). A thread blocked in `wait_one` keeps its `waiters[]` entry with the object's index
(`sync_objects.c` 394). CloseHandle of the object's last handle frees the slot and the object (`w2m_object_close`
114-127) and readies no waiter. The woken wait's `!o` path (379-383) returns WAIT_FAILED with its entry and its
`wait_deadline` left. The next object made reuses the index (`new_object` 69-72), and `grant` (214-230) hands its first
release to the stale entry, so a thread waiting INFINITE on the new object may never wake.

Microsoft Learn, WaitForSingleObject: "If this handle is closed while the wait is still pending, the function's
behavior is undefined." The close is therefore the guest's error. The damage to a later, correct object is w2m's.

- **The fix.** Implement the design's lifetime rule for this case: a waiter holds a reference on its object, so the
  object outlives its last handle while any waiter holds it. Every path out of `wait_one` drops its entry and its
  deadline; the `!o` path is among them. List every return path of `wait_one` and `wait_many` with what each drops,
  and the same for WaitForMultipleObjects's stop at 421.
- **The judge.** A probe, committed by the Supervisor as a "Sprint 350 card amendment:" commit (Sprint 349's kind). A
  worker waits with a finite timeout on a semaphore. The main thread closes the semaphore's last handle, makes a new
  semaphore, releases it once, and waits on it with a zero timeout. Windows defines that last wait, WAIT_OBJECT_0, and
  the rows compare it. The worker's result is undefined, so record it but do not compare it. Write the probe in
  `probes/` in the working tree and send its path. The Supervisor reviews, commits and pushes it, then you run it on
  the Windows runner. A `run_loop.rs` test holds the defined rows on one vCPU and on two to Windows', and fails on
  today's code.
- **Plant 2.** The waiter's reference taken out. The test fails.

## result

- **The design:** `docs/design-handles-and-sync-objects-2026-11-26.md`, its lines those of 23b860e79.
  - The state: 17 items, eight arrays and words of `sync_objects.c` (45-48, 183-184, 238-239), each with its line.
  - The services: 43 across 8 files (`sync_objects.c`, `file.c`, `runtime.c`, `console.c`, `processes.c`,
    `threads.c`, `thread_queries.c`, `token.c`), each with its line, how it uses the table, the sync state it touches,
    and whether it makes a host request. Each wait's way of blocking is stated.
  - The lifetime rule: two counts, `handles` and `refs`. A lookup and every blocked wait's entry hold a reference, and
    an entry is freed only at `refs` 0. Its grounds are quoted: Microsoft Learn's Life Cycle of an Object,
    ObReferenceObjectByHandle and CloseHandle; ReactOS `ob/obwait.c` 398-437 at 3e6033a6; Wine `server/thread.c` 1102
    and 1063 at 274fcfa9.
  - The locks: one `handles_lock` and one `sync_lock`, both leaves; the order as built; the woken mark; the classes
    `Lock::Handles` (10 services) and `Lock::SyncObjects` (8). Every other service stays on the runtime word.
  - Cards 2 to 5 follow, with the judges and plants each needs.
  - `target/release/design_check`: "design_check: 8 plans and designs state the goal and the systems", exit 0. The 8
    are the 15 matching names under `docs/` less the 7 of `HISTORY`, the new design among them.
- **The defect and its fix** (the amendment):
  - `sync_objects.c`: `unref`, `waiter_add`, and the references in `waiter_drop`, `waiters_drop` and `all_drop`.
  - `handles` counted apart, so the host's closes stay at the last handle's CloseHandle (reviewer-4's nineteenth
    review, Medium 1).
  - `signaled()` reads a SHARED_EVENT or PROCESS with no handle left as unsignaled.
  - `wait_one`'s retry waits on its entry's object, and its WAIT_FAILED path clears the deadline.
  - The return paths of `wait_one` and WaitForMultipleObjects, the stop at 421 among them, are listed in the comments
    above each.
  - `crates/guest_runtime/tests/python_image.rs` names `sync_objects.c` in `DEPARTURES`.
- **The Windows run:** run 45797800625 (`windows-2025`, 10.0.26100.33828, an Intel Xeon Platinum 8573C),
  `probes/wait_closed_handle` as committed 1decadd51, three runs, each exit 0 in 1.031, 1.023 and 1.031 s, the bodies
  alike byte for byte:
  `{"s":"main","closed":1,"made":1,"released":1,"previous":0,"wait":0,"ended":0}` and
  `{"s":"worker","wait":258,"error":0}`. Its files are in `store/probe-runs/45797800625/` and its binary in
  `store/binaries/f0ae78e5b964….exe`. The walk fixture gained its entry, 293 instructions, all walked, 69 block starts;
  `fixtures.rs` now reads 286 images and 643,637 instructions.
- **Before the fix:** the stored binary through the test on 23b860e79's `sync_objects.c`, in a scratch worktree, failed
  "wait_closed_handle on 1 vCPUs, fields unlike Windows' [\"main.wait: 258 against 0\"]". Its rows read
  `"wait":258` for main and `"wait":0` for the worker. A scratch mingw build gave the same rows on 1 vCPU and on 2.
- **The tests:**
  - `run_loop::a_semaphore_made_after_a_waited_handle_is_closed_keeps_its_unit_on_one_and_two_vcpus`: main's row held
    to Windows' on 1 vCPU and on 2.
  - `run_loop::the_hosts_close_of_a_shared_event_comes_at_its_last_handle_while_a_wait_holds_it`: the new guest
    `tests/data/timed_waits/shared_close`, with the exit ledger's KERNEL_CLOSE row before the first HOST_WRITE row;
    its stop and output are held in `store/batch_run/first-stop-2026-10-28.tsv`.
  - Both pass.
- **Plant 1** "docs/design-handles-and-sync-objects-2026-11-26.md: no \"## Against the systems that solved it\"
  section": the section removed on a scratch copy of `docs/` and `BLACKBOARD.md`; `design_check --root` exit 1. Its
  reach was first read on a scratch copy of `design-translation-cache-2026-10-30.md`.
- **Plant 2** "wait_closed_handle on 1 vCPUs, fields unlike Windows' [\"main.wait: 258 against 0\"]": `waiter_add`'s
  `o->refs++;` replaced by `(void)0;` in a scratch worktree's copy, one line for one. Run on the first fix and again
  on the final one.
- **Plant 3** "the host's close of the shared event (ledger row None) before the write of \"closed\" (row Some(24))":
  the host's closes moved back from `w2m_object_close` into `unref`, in a scratch worktree's copy. The other test
  passed.
- **The gates:**
  - The first run passed (10:14:12Z) on the tree before the nineteenth review's changes.
  - The second failed at 10:36Z on `batch_run::the_batch_run_and_the_guests_stop_where_held`: shared_close's stop was
    not yet held.
  - The final run: `exit 0 in 364 s	default	2026-11-26T10:43:12Z	sprint 350	1decadd51d21963fecb7523c7ec27a0b22405f10	53c8ff87d490539daac0c75eda93c88583031ca210a30c03e31f08f6931a24ff	root=b68c9e35c661badc36997ec812f9b64c5fc64e5b	rust=all python=0 samples=0	base=068d573dab6b4e130b0af8317acf7133076a415a`
