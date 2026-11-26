# Sprint 348 — A run-table entry is published by a release store of the count, read with acquire

```yaml
---
id: 348
status: active
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-4's eighteenth review, Low 2, recorded in the Decision "Two cards from the eighteenth review". The host's
`threads::add_thread` (`crates/w2m_vm/src/threads.rs` 168-172) writes a new entry's `E_TID`, `E_STATUS` and `E_TEB`,
then `RT_COUNT`. Each is a `Relaxed` atomic store through `ConcurrentGuestMemory::write` (`concurrent_memory.rs`
128-130).

Since Sprint 347, `tls_clear_everywhere` (`tools/guest_runtime/thread_storage.c` 140-153) reads `RUN_COUNT` by a plain
`volatile` load, then each entry's `teb` and `status`. It holds only the slots' lock, not the runtime word the host's
writer holds. On several vCPUs it may see the count before the entry's words. That is safe today only because:
- entries are never reused, so an unseen `teb` reads 0 and is skipped;
- a new thread's TEB and arrays are fresh, zeroed memory;
- 0 is what the clear writes.

No comment states that argument. The Decision of 2026-10-31 on guest memory on several vCPUs has shared words reached
as atomics. This card applies that rule to ordering as well.

- **The change:**
  - `add_thread` stores `RT_COUNT` by `ConcurrentGuestMemory::atomic_u64(..).store(n + 1, Release)`, after the
    entry's words.
  - `thread_storage.c`'s `RUN_COUNT` is read by `__atomic_load_n(.., __ATOMIC_ACQUIRE)`. Confirm the built image's
    `ldar` and quote it.
  - The comment on `tls_clear_everywhere` names the pairing, not the zeroed-memory argument.
- **The other readers.** Name every reader of the count in the runtime C and the host: `threads.c`'s nine reads of
  `RUN->count`, and `threads.rs` 151 and 193. For each, quote what orders it against `add_thread`'s store. Where it is
  the runtime word, say so with the lines, and leave it unchanged. Make any reader that holds neither the word nor an
  acquire an acquire.
- **The judge.**
  - A host test holds that `add_thread` publishes the count by a release store after the entry's words. Check it
    through the code path, not the source text, for instance by a `ConcurrentGuestMemory` call recorder, or by
    reading the order of the stores into a test page.
  - The built-image check holds `RUN_COUNT`'s load in `_k32_TlsAlloc` and `_k32_TlsFree` to an `ldar`.
- **Not here:**
  - any reuse of run-table entries;
  - the scheduler's own locking;
  - Change 1's next classes.

## prerequisites

- 347 (closed)

## context_files

- `crates/w2m_vm/src/threads.rs` (`add_thread`, `read_u64`, `write_u64`, lines 140-200) and
  `crates/w2m_vm/src/concurrent_memory.rs` (`write`, `atomic_u64`).
- `tools/guest_runtime/thread_storage.c` (`RUN_COUNT`, `struct run_head`, `tls_clear_everywhere`) and
  `tools/guest_runtime/threads.c` (`struct w2m_run_table`, each `RUN->count`).
- `docs/design-guest-memory-on-several-vcpus-2026-10-31.md`, the rule for shared words.
- `crates/guest_runtime/tests/python_image.rs` (`DEPARTURES`, naming `thread_storage.c`; extend its reason).
- Sprint 343's result (`sprints/sprint-343-the-process-heap-takes-its-own-lock.md`) for how a built image's
  instructions were quoted.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None; the ordering of one shared word.

### Invariants

- On one vCPU every run, the batch run, the exit ledger and the family runs pass unchanged.
- Card 347's tests and card 345's test pass unchanged.

## artifact contract

### Files modified

- `crates/w2m_vm/src/threads.rs`, `tools/guest_runtime/thread_storage.c`, a test, and `crates/guest_runtime/tests/python_image.rs`.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - Each reader of the count, with what orders it.
  - The host test's figures.
  - The built image's `ldar` quoted from both services.
- **Planted faults:**
  - (1) `add_thread`'s count stored `Relaxed`, as before. The host test fails by name.
  - (2) `RUN_COUNT` read by a plain load. The built-image check fails on the missing `ldar`.

  Before the change, read each judge's reach: the host test failing on today's store.

## done criteria

A thread's run-table entry is published to readers outside the runtime word by a release store of the count after
its words, read with acquire, so no reader depends on zeroed memory for its safety.

## self-review checklist

- [x] Scope names one verifiable property: the count published by release, read with acquire.
- [x] context_files cite the writer, the readers, the memory design's rule and the built-image form.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the readers, the host test, the built image; two numbered plants, their reach read.
- [x] Sweet spot: `threads.rs` and `thread_storage.c`; one concept, one word's ordering.

## result

GATES-LINE
