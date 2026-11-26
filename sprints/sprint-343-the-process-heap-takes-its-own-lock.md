# Sprint 343 — The process heap takes its own lock, not the runtime word

```yaml
---
id: 343
status: done
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

Done. The five services are of lock class heap (`crates/guest_runtime/src/services.rs` `Lock::Heap`, `HEAP_LOCK`), and
their bridges take no runtime word, as for class none (`guest::bridge`, `guest.rs` 601, unchanged: `word` is set only
for the runtime word's class or a traced call). `heap.c`'s `hold` and `let_go` (54-81) take and release the heap's own
lock word, `struct heap`'s `lock` at `W2M_HEAP_VA + 0x18`, around the free list, the bump pointer and the set-up. The
judge is `run_loop.rs`' `the_process_heap_hands_no_block_to_two_vcpus_at_once`.

- **The proof, from `heap.c` whole as it stood (184 lines).** The heap's shared state is `struct heap` (37-41) and
  `ready` (43). Its stores: the set-up in `the_heap` (47-55: "`ready = 1;`", `h->magic`, `h->next` twice, `h->free_list
  = 0`, PEB.ProcessHeap); `take` (77-94: "`*link = *(u64 *)(b + 1);`", the free list's head or a free block's link,
  and "`h->next += sizeof(struct header) + want;`", then the carved block's "`b->capacity = want;`"); `give_back`
  (96-100: the freed block's link and "`h->free_list = (u64)b;`"). The services' other stores belong to the block's
  holder or the thread: HeapAlloc's "`b->size_and_magic_check = size; b->reach = ...`" and `zero` (134-136) on the
  block `take` returned; HeapReAlloc's in-place "`zero((u8 *)p + old, ...)`", "`b->size_and_magic_check = size;`" and
  "`if (size > b->reach) b->reach = size;`" (167-169) on the caller's own block, and the moved block's header, `copy`
  and `zero` (178-181), as Windows leaves a block to its caller; `set_error` (60), the calling thread's TEB; and
  `w2m_logged`, the ring by atomic add. HeapSize stores nothing but the ring. Calls: `check_handle` (72-74) and
  `check_flags` (68-70) call `w2m_unimplemented` first in each of the five; `header_of` (102-106) calls it before
  `give_back` (`give_back(header_of(...))`, 119, 143) and before any `take` in HeapReAlloc (156); HeapAlloc's and
  HeapReAlloc's "`if (!b) w2m_unimplemented(...)`" (133, 177) follow `take`'s return. Inside `take`, `give_back` and
  the set-up the only call is `w2m_parameters()`, an inline load of PEB + 0x20 (`runtime.h` 62). So the lock, held
  inside those three alone, is never held across `w2m_unimplemented` or `w2m_logged`. The callers of `w2m_heap_take`
  and `w2m_heap_release` on the word: the card's eight files and two more, `command_line_args.c` 85 and 99 and
  `dxgi.c` 57 and 68; the order word then heap has no cycle, since the heap's holder waits on nothing.
- **The set-up's race, found by the judge.** The old set-up stored "`ready = 1;`" (48) before `h->next` (50); on
  today's C the judge faulted in round 0 in three of four runs, "ESR 0x96000046, FAR 0x8, ELR 0x7f000417c544", which
  is `ldr x8, [x9]` (`h->next`, read 0) then `str x11, [x8, #0x8]`, `b->capacity` at address 8. The set-up now runs
  under the lock, so neither vCPU takes a block before it is done.
- **The built lock** (`build/guest_runtime_rust/4d9a8ce3...bin`): `_k32_HeapAlloc` at 0x7f000417c468 "`mov x8, #0x18;
  movk x8, #0x2, lsl #32; mov w9, #0x1; mov w10, #0x0; casa w10, w9, [x8]; cbnz w10`" at ...4e0, its release
  "`stlr wzr, [x9]`" at ...5d8; `_k32_HeapFree` `casa` at ...6f0, `stlr wzr, [x8]` at ...780; `_k32_GetProcessHeap`
  ...3d0 and ...454; `_w2m_heap_take` ...1b0, ...294 and ...2b8; `_w2m_heap_release` ...324 and ...3b0;
  `_k32_HeapReAlloc` ...8ec with ...aa4, and ...bcc with ...c4c. All inline; every path from a `casa` reaches its
  `stlr` before any call, the arena-full paths in `_k32_HeapAlloc.cold.1` and `_k32_HeapReAlloc.cold.1` storing
  "`stlr wzr, [x8]`" before "`bl _w2m_unimplemented`". `_k32_HeapSize` takes no lock.
- **The judge.** Two vCPUs run a driver at `MONITOR_VA + 0x2400` (one page table, a stack each) calling the image's own
  `k32_HeapAlloc` and `k32_HeapFree` with no word: each round from the heap as a process starts it (`ready` 0, the
  heap's first page zero), 64 passes over 32 slots a vCPU, a slot's block marked at its second and last words and the
  mark checked before HeapFree. After each round the host walks the carved blocks from the heap's start to the bump
  pointer and holds each to one holder, a slot (its marks) or the free list, none lost, the lock word 0; a round
  whose loops have not ended in 60 s has its vCPUs stopped.
- **Before the change**, today's C: "round 0: vCPU 0's block 0x200001700 in slot 15 read 0xb10c000000000002, not its
  mark 0xb10c000000000001, with 63 passes left", then three runs of the set-up's fault above. The same driver with the
  runtime word taken around each call, as each service's bridge takes it today (a temporary edit, restored by `cp`,
  checked by `cmp`): "2000 rounds ... 8192000 HeapAlloc and 8064000 HeapFree calls in all; no mark found wrong; ...
  none lost (at most 127 carved, 63 free)", 1.488 s.
- **After**: "the process heap on two vCPUs: 2000 rounds, each from the heap as a process starts it, of 64 passes over
  32 slots a vCPU, 8192000 HeapAlloc and 8064000 HeapFree calls in all; no mark found wrong; at each round's end every
  carved block held by one slot or on the free list once, none lost (at most 174 carved, 110 free), the bump pointer at
  most 0x20000b520 of the arena's 0x210000000, the lock free", 3.086 s.
- **The table and the bridge.** `the_five_services_of_lock_class_heap_are_the_process_heaps` holds class heap to the
  five names, each defined in `heap.c` and of class heap in clock modes 0, 1 and 2, and LocalAlloc and LocalFree on
  the word. `a_bridge_of_lock_class_none_takes_no_runtime_word` now holds a heap-class bridge to class none's word for
  word, traced and untraced, timed and not. `DEPARTURES` names `heap.c` with Sprint 343.
- **Plant 1** "round 0: vCPU 0: Exception { syndrome: 1509949454, virtual_address: 0, physical_address: 0 } at
  0x7f0000001cc0, not the loop's hvc #0x77; the save area's ESR 0x96000046, FAR 0x8, ELR 0x7f000417c544": `hold`'s
  compare-and-swap loop and `let_go`'s store removed, the services off the word; four runs, each failing in round 0
  on the set-up's race. A variant with the lock around the set-up alone failed on the card's named form, "round 0:
  vCPU 0's block 0x200001a50 in slot 13 read 0xb10c000000000002, not its mark 0xb10c000000000001, with 62 passes
  left", and twice stopped at 60 s in `take`'s free-list walk, a cycle. Restored by `cp`, checked by `cmp`.
- **Plant 2** "assertion `left == right` failed: the services of lock class heap left: [\"KERNEL32!GetProcessHeap\",
  \"KERNEL32!HeapFree\", \"KERNEL32!HeapReAlloc\", \"KERNEL32!HeapSize\"]": HeapAlloc added to `NO_LOCK` (`[&str; 8]`),
  `the_five_services_of_lock_class_heap_are_the_process_heaps` failing by name; restored by `cp`, checked by `cmp`.
- **Test and test-and-set** (reviewer-4's fourteenth review, Low 1): `lock_runtime` waits on a plain load after a
  failed CASA until the word reads 0 (`monitor_code.rs` 128-131); `hold` now does too, "`while
  (__atomic_load_n(&h->lock, __ATOMIC_RELAXED)) {}`" before each swap, so a waiter does not take the line the holder
  writes `next` and `free_list` in. Rebuilt (`build/guest_runtime_rust/da2873d4...bin`), `_k32_HeapAlloc` at
  0x7f000417c474: "`mov x9, #0x8; movk x9, #0x2, lsl #32; mov x8, #0x18; movk x8, #0x2, lsl #32; mov w10, #0x1;
  ldr w11, [x9, #0x10]; cbnz w11, 0x7f000417c4f0; casa w11, w10, [x8]; cbnz w11, 0x7f000417c4f0`" at ...4dc-...4fc,
  the release "`stlr wzr, [x9]`" at ...5e8; the same load loop before every `casa` of `_w2m_heap_take` (...1b4),
  `_w2m_heap_release` (...32c), `_k32_GetProcessHeap` (...3dc), `_k32_HeapFree` (...704) and `_k32_HeapReAlloc`
  (...904, ...be0). The heap test after it: the same figures, "none lost (at most 174 carved, 110 free)", 4.086 s.
  Plant 1 rerun on it: "round 0: vCPU 1: Exception { syndrome: 1509949454, virtual_address: 0, physical_address: 0 }
  at 0x7f0000001cc0, not the loop's hvc #0x77; the save area's ESR 0x96000046, FAR 0x8, ELR 0x7f000417c544";
  restored by `cp`, checked by `cmp`. The first gates run's row (731 s, `root=440d5ac8...`) stays in
  `store/gates/passes.tsv`.
- **The gates** (copied from `tail -1 store/gates/passes.tsv`): `exit 0 in 797 s	default	2026-11-26T05:02:30Z	sprint 343	0d085b17f12e7b220eed01edb39b2f10b6fc3024	ce9cb157b9d47e1f9caf36574fc28b4478fc2149905771e7fd8fb5e7dd875b39	root=3137020964ee52293b285893e9b6a4c979f2aa74	rust=all python=0 samples=0	base=60210d183ab0c7295b73bde71e4e78ea9f3e3203`
