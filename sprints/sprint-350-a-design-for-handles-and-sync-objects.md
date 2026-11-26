# Sprint 350 — A design for the handle table's lock and the sync objects' locks in the guest runtime

```yaml
---
id: 350
status: active
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

## result

GATES-LINE
