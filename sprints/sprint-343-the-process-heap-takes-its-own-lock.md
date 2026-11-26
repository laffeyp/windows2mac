# Sprint 343 — The process heap takes its own lock, not the runtime word

```yaml
---
id: 343
status: active
phase: 6
pass_kind: functional
---
```

## scope

The second lock class of Change 1 in the host design is **a lock per heap**, "as Windows has one"
(`docs/design-rust-host-2026-10-24.md` 95-140, citing Microsoft Learn, HeapCreate: "the heap serializes access within
the calling process"). HeapAlloc is one of the game's four hot imports, the mix G6 measures. Sprint 342 built the None
class. This card moves the process heap's services off the runtime word and onto the heap's own lock:

- GetProcessHeap;
- HeapAlloc, HeapFree, HeapSize and HeapReAlloc.

All five are in `tools/guest_runtime/heap.c`.

- **The work:**
  - **The heap's lock.** `struct heap` (`heap.c` 37-41, the arena's first page at `W2M_HEAP_VA`) gains a lock word.
    The lock is taken and released by compare-and-swap with acquire, and by a store with release, in the C as
    `lock_runtime` does in the monitor. Use `__atomic_compare_exchange_n` and `__atomic_store_n`, and confirm the
    built image inlines them (`casa` and `stlr`, no call), quoting the disassembly.
  - **What the lock covers.** It is held around the heap's shared state alone:
    - the free list and the bump pointer, in `take` and `give_back`;
    - the heap's one-time set-up, `the_heap`'s `ready`, which two vCPUs could otherwise race to run.

    Two calls must never run while it is held: `w2m_unimplemented`, which takes the runtime word and makes a host
    request, and `w2m_logged`. So the heap's lock never waits on another lock, and a service that already holds the
    runtime word can call `w2m_heap_take` and `w2m_heap_release` without an inversion. Today `com.c`, `environment.c`,
    `file_search.c`, `known_folders.c`, `local_memory.c`, `messages.c`, `threads.c` and `thread_storage.c` all do,
    from services on the word.
  - **The proof, before the change.** Read `heap.c` whole and quote it: every store each of the five makes, and every
    call each makes while the lock would be held. A block's own header edits (`size_and_magic_check`, `reach`, in
    `HeapReAlloc`) belong to the block's holder, as Windows leaves a block to its caller. Say so with the lines.
    `check_handle` and `check_flags` call `w2m_unimplemented` before any lock is taken.
  - **The bridge.** The service table gains a third lock class, the heap. For the five, `guest::bridge` takes no
    runtime word, as for class none, and keeps its length the same way (Sprint 342's Decision on the frozen host's
    code addresses).
- **The judge.** A two-vCPU test drives the image's own `_k32_HeapAlloc` and `_k32_HeapFree` from a loop on each of
  two vCPUs at once, as Sprint 341's driver drove `_w2m_logged`:
  - each loop writes its vCPU's mark into every block it holds, and checks the mark before it frees the block;
  - after many rounds, the test holds that no block was ever handed to both vCPUs at once, that the free list is a
    well-formed chain of blocks inside the arena, and that the bump pointer lies inside the arena.

  Before the change, run the same test with each service still taking the word, through its bridge. That shows its
  reach.
- **Not here:**
  - HeapCreate's heaps, which do not exist yet (`check_handle`).
  - The low-fragmentation heap.
  - Change 1's other classes.
  - Any guest-visible change to what the heap returns.

## prerequisites

- 342 (closed)

## context_files

- `docs/design-rust-host-2026-10-24.md` 95-140, Change 1, its classes and its order.
- `tools/guest_runtime/heap.c` whole (184 lines), and `tools/guest_runtime/runtime.c` (`w2m_logged`,
  `w2m_unimplemented` and its `hold_runtime_word`, Sprint 342).
- The callers of `w2m_heap_take` and `w2m_heap_release`: `grep -n 'w2m_heap_take\|w2m_heap_release'
  tools/guest_runtime/*.c`.
- `crates/guest_runtime/src/services.rs` (`Lock`, `NO_LOCK`, the table test) and `crates/w2m_vm/src/guest.rs` (`bridge`
  and its padding), as Sprint 342 left them.
- Card 341's and card 342's results, for the driver form and the bridge's class and padding.
- `crates/w2m_vm/tests/run_loop.rs`: Sprint 341's ring driver and Sprint 342's ring check.
- `crates/guest_runtime/tests/python_image.rs` (`DEPARTURES`, which must name `heap.c`).
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands", with "A sprint closes in its work commit"
  and "no gate row is dropped".

## signal contract

### Emits

- None; which lock the heap's services take, no new event.

### Invariants

- On one vCPU every heap service returns what it returns today, and the batch run, the exit ledger and the family runs
  pass unchanged.
- The heap's lock is never held across `w2m_unimplemented` or `w2m_logged`.

## artifact contract

### Files modified

- `tools/guest_runtime/heap.c`, `crates/guest_runtime/src/services.rs` (the heap class),
  `crates/guest_runtime/tests/python_image.rs` (`DEPARTURES`), and a test. `crates/w2m_vm/src/guest.rs` changes only if
  the bridge needs a case for the new class beyond the table's.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The proof quoted.
  - The built heap functions' `casa` and `stlr` quoted.
  - The two-vCPU heap test, its figures quoted (rounds, blocks, any mark found wrong).
  - The table test naming the five services of class heap.
- **Planted faults:**
  - (1) The heap's lock taken and released as no-ops while the services stay off the word. The two-vCPU test fails on
    a block held twice or a broken free list.
  - (2) HeapAlloc set to class none, not heap. The table test fails by name.

  Before the work, read each judge's reach: show the two-vCPU test failing with the lock absent before the lock
  exists.

## done criteria

The process heap's five services take the heap's own lock and not the runtime word. The lock covers only the free list,
the bump pointer and the set-up, and is never held across another lock. Two vCPUs allocating and freeing at once never
share a block.

## self-review checklist

- [x] Scope names one verifiable property: the heap's services off the word, on their own lock, safe under two vCPUs.
- [x] context_files cite Change 1, the heap's C, its callers, the class and bridge, the driver form and the frozen-C list.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the proof, the built lock, two tests; two numbered plants, their reach read.
- [x] Sweet spot: `heap.c` and `services.rs` plus a test; one concept, the heap's lock.

## result

GATES-LINE
