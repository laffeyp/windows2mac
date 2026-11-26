# Sprint 348 — A run-table entry is published by a release store of the count, read with acquire

```yaml
---
id: 348
status: done
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

The run table's count is published by a `Release` store in `threads::add_thread`, after every word the function writes
in the entry, and `thread_storage.c`'s `RUN_COUNT` reads it with `__atomic_load_n(.., __ATOMIC_ACQUIRE)`. The built
image holds `ldapr`, not the `ldar` the card names: clang's target, `arm64-apple-macos14`, whose apple-m1 has
FEAT_LRCPC, builds an acquire load as the RCpc load-acquire (LLVM D126250, "[CodeGen][AArch64] Add support for LDAPR",
https://lists.llvm.org/pipermail/llvm-commits/Week-of-Mon-20220530/1042633.html; Arm's "Enabling RCpc in GCC and
LLVM", https://community.arm.com/arm-community-blogs/b/tools-software-ides-blog/posts/enabling-rcpc-in-gcc-and-llvm).
LDAPR orders every later load after the count, which is all the reader needs; it differs from LDAR only for an earlier
store-release by the same thread, which the reader does not make. The judge holds the load to a load-acquire, `ldar`
or `ldapr`, and fails a plain load.

- **The readers of the count, and what orders each against `add_thread`'s store.** The writer holds the runtime word:
  `create_thread` runs in a CreateThread request, made by a service on the word (`guest_runtime::services`, CreateThread
  in none of `NO_LOCK`, `HEAP_LOCK`, `SLOTS_LOCK`), and the first thread is added at `run.rs` 966 before any vCPU runs.
  - `threads.c` 111 (`w2m_block`), 133 (`w2m_signal`), 151 (`w2m_ready_thread`), 351 (`put_down_if_suspended`): called
    only from services on the word (`critical_section.c` 90 and 109, `sync_objects.c` 228, 276, 296, 366 and 373,
    `processes.c` 77, 97 and 100, `slim_locks.c` 37, 58, 82, 123, 142, 149 and 170; SuspendThread for 351). The word:
    unchanged.
  - `threads.c` 168 (`wake_expired`), 180 (`slice_until`), 223 and 224 (`nearest_deadline`), 246 (`w2m_switch`):
    reached only from `w2m_switch`, which the switch stub calls after "`take_runtime(a, timed); // before the first store
    to the shared run table`" (`guest.rs` `switch_stub`). The word: unchanged.
  - `threads.rs` 151, `add_thread`'s own read: the count's one writer reading it, under the word. Unchanged.
  - `threads.rs` 193 (199 after the change), `current_tid`: from `schedule` (which asserts "the scheduler entered without
    the runtime word"), `run.rs` 1654 (a host request, "the guest's own hold of the word"), 1741, 1756 and 1896 (after
    `take_word`), `host_request.rs` 639 and `threads.rs` 388, 476, 866, 1271 and 1351 (inside a request or the scheduler), and
    `w2m-vm.rs` 1057 after the run has stopped. The word, or no vCPU running. Unchanged.
  - `thread_storage.c` 143, `tls_clear_everywhere` in TlsAlloc and TlsFree: the slots' lock alone, neither the word nor
    an acquire before. Now the acquire.

  No other reader: `grep` for `RUN->`, `rt->count`, `RUN_COUNT` and `W2M_RUN_TABLE_VA` in `tools/`, and for `RT_COUNT`
  and `RUN_TABLE)` in `crates/`, finds only these.
- **The change.** `add_thread` writes tid, status, TEB, vcpu and vslot (vslot and the first thread's `set_current` moved
  before the count, where vslot followed it), then `mem.store_u64(RUN_TABLE + RT_COUNT, n + 1, Release)`;
  `ConcurrentGuestMemory::store_u64` is `self.atomic_u64(offset).store(value, order)`, recorded in the crate's tests.
  `RUN_COUNT` is `__atomic_load_n((const u64 *)W2M_RUN_TABLE_VA, __ATOMIC_ACQUIRE)`. The comment on
  `tls_clear_everywhere` names the pairing, and its `!teb` test, which the zeroed-memory argument needed, is gone: every
  entry below an acquired count holds its TEB. `DEPARTURES` names Sprint 348 for `thread_storage.c`.
- **The host test**, `threads::tests::a_new_threads_entry_is_published_by_a_release_store_of_the_count_after_its_words`:
  a recorder in `concurrent_memory.rs` (`record`, test builds only) logs each `write` and `store_u64` in order while
  `create_thread` makes a third thread. "add_thread's publication: 579 stores recorded in CreateThread's host half; the
  count's one, a Release store of 3, at 537; the entry's last stores: tid 532, status 533, teb 534, vslot 536, vcpu
  535". Before the change, on today's store: "assertion `left == right` failed: the run table's count stored by these,
  not one Release store / left: [Copy { offset: 165384192, len: 8 }] / right: [Store { offset: 165384192, order:
  Release }]" (165384192 is 0x9DB9000, `RUN_TABLE`).
- **The built image** (`build/guest_runtime_rust/88f72d09800f2f23c776ccc507197713b87efddfc3dfad7feaa3739c208afa2a.bin`),
  `tls_clear_everywhere` inlined in both: `_k32_TlsAlloc` "`mov x13, #0x9000; movk x13, #0x9db, lsl #16; movk x13,
  #0x7f00, lsl #32; ldapr x11, [x13]`" at 0x7f0004186cbc-...cc8; `_k32_TlsFree` "`mov x10, #0x9000; movk x10, #0x9db,
  lsl #16; movk x10, #0x7f00, lsl #32; ldapr x8, [x10]`" at 0x7f0004186f4c-...f58. The judge,
  `crates/guest_runtime/tests/run_count_load.rs` `tls_alloc_and_tls_free_read_the_run_tables_count_with_a_load_acquire`,
  decodes both functions' words, follows `movz`/`movk` to each 64-bit load from 0x7f0009db9000, and prints "the count
  at 0x7f0009db9000 read by a load-acquire at _k32_TlsAlloc 0x7f0004186cc8 0xf8bfc1ab AcquirePc, _k32_TlsFree
  0x7f0004186f58 0xf8bfc148 AcquirePc"; `the_decoder_finds_the_counts_load_and_tells_ldar_from_ldr` holds its decoder to
  `ldr`, `ldar`, `ldapr`, another offset and a moved base. Before the change, on today's C (image 570d35f5...): "k32_TlsAlloc:
  k32_TlsAlloc's load of the run table's count at 0x7f0004186cc8 is 0xf94001ab, a plain load, not ldar".
- **Plant 1** "assertion `left == right` failed: the run table's count stored by these, not one Release store / left:
  [Copy { offset: 165384192, len: 8 }] / right: [Store { offset: 165384192, order: Release }]": `threads.rs` 185 made
  "`write_u64(mem, RUN_TABLE + RT_COUNT, n + 1);`", the host test failing by name; restored by `cp`, checked by `cmp`.
- **Plant 2** "assertion `left != right` failed: k32_TlsAlloc: k32_TlsAlloc's load of the run table's count at
  0x7f0004186cc8 is 0xf94001ab, a plain load, not a load-acquire (ldar or ldapr) (image 68f07a5b...)": `thread_storage.c`
  129 made "`#define RUN_COUNT (*(volatile u64 *)W2M_RUN_TABLE_VA)`", the built-image judge failing on the plain `ldr`;
  restored by `cp`, checked by `cmp`.

- **The gates** (copied from `tail -1 store/gates/passes.tsv`): `exit 0 in 531 s	default	2026-11-26T09:22:10Z	sprint 348	ae9782bef718545ba05805019c71fce120797c5d	1330ae379fb29233eaf4e607c7d743ee5e54213cf07ed5f78893fbf22ae9bbe4	root=c3e06d933dec5bb3a289587fda0b717e5f36ed94	rust=all python=0 samples=0	base=3db0098a25fd959a094e01dfb7e1d7bd29f372dc`
