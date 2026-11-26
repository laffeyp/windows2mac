# Sprint 359 — The runtime C's locks released by a scoped guard

```yaml
---
id: 359
status: done
phase: 6
pass_kind: functional
---
```

## scope

The Decision "The runtime C's leaf locks released by a scoped guard" (2026-11-26, reviewer-4's twenty-fifth review,
Low 1). The runtime C takes its locks in hand-written pairs, and each function's early returns repeat the release:
- `hold`/`let_go` (heap.c);
- `hold_slots`/`let_slots_go` (thread_storage.c, 14 releases at 304470288);
- `hold_handles`/`let_handles_go` and `hold_sync`/`let_sync_go` (sync_objects.c, 49 releases between them);
- since Sprint 355, `w2m_take_runtime_word`/`w2m_let_runtime_word_go` (runtime.c), which lets the word go only when
  this call took it.

Correctness rests on every future edit remembering the release. Sprint 338 removed that pattern from the host with
`RuntimeWordGuard` and its source test (`crates/w2m_vm/src/run.rs` 2288-2330). This card gives the C the same form.

- **The guard.** Clang builds the runtime (`crates/guest_runtime/src/lib.rs` 152), and its `cleanup` attribute calls a
  function when a variable leaves scope. Look its behavior up in Clang's attribute reference, and quote it.
  - Write a scoped form per lock, for instance `HELD(handles)`: it declares a guard whose cleanup lets the lock go,
    and every take in these five files goes through it.
  - The runtime word's guard lets it go only when its take took it (`w2m_take_runtime_word`'s answer).
  - Under `W2M_TIME_LOCKS` (Sprint 358) the guard's release is the timed one, so the timing's entries are unchanged.
- **The one exception stays explicit.** `w2m_unimplemented` never returns, and cleanup runs at scope exit, not before
  a call. So a function that calls it, or calls `w2m_logged` while it would otherwise hold a lock, still releases first
  by hand, as today. Do it through a named form that ends the guard early (for instance `LET_GO_EARLY(handles)`), not a
  bare release, so the source test can tell the two apart. The macro's comment says why.
- **Unchanged behavior.**
  - The batch run, the family runs on one vCPU and two, the exit ledger, every run test, and the two-vCPU judges of
    Sprints 343-356 pass unchanged: `semaphore_units`, `handle_values`, `lost_wakeup`, `mutex_abandoned`,
    `heap_blocks`, `thread_slots_race`.
  - The record of Sprint 358 can be remade with the same figures, within a run's spread.
- **The judges.**
  - **A source test**, in the form of `the_word_is_released_only_by_its_guards_drop`: in the five files, no release
    outside a guard's cleanup or a named early release. Each early release sits on a path that then calls
    `w2m_unimplemented` or `w2m_logged`, or its line is named with its reason.
  - **The built image**, by disassembly of at least three functions with early returns (for instance
    `w2m_object_close`, TlsFree and `wait_one`): a release store (`stlr wzr`) on each return path while the lock is
    held, as today.
- **Not here:** any change of lock class or of what a lock guards.

## prerequisites

- 358 (closed)

## context_files

- `tools/guest_runtime/heap.c`, `thread_storage.c`, `sync_objects.c` and `runtime.c`: in full. Also `runtime.h`
  (`w2m_lock_take` and `w2m_lock_let_go` under `W2M_TIME_LOCKS`).
- `crates/w2m_vm/src/run.rs` 790-830 (`RuntimeWordGuard`) and 2280-2335 (its source test).
- `crates/guest_runtime/src/lib.rs` 140-170 (the runtime's build), and `crates/guest_runtime/tests/python_image.rs`
  (`DEPARTURES`, the frozen key).
- `grep -n "The runtime C's leaf locks released by a scoped guard" BLACKBOARD.md`: the Decision.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None: no emit site changes. The runtime's behavior and its records are unchanged, and Sprint 358's
  `run_measurements` record, remade on this tree, carries the same metrics. That remade record is the observation's,
  not a new emit.

### Invariants

- Every existing run, record and judge passes unchanged on one vCPU and two.

## artifact contract

### Files modified

- `tools/guest_runtime/heap.c`, `thread_storage.c`, `sync_objects.c`, `runtime.c`, `runtime.h`, the source test, and
  `DEPARTURES`. One concept: the locks' scoped form.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The count of hand releases before (by `grep`) and after: after, only the named early releases, each listed with
    its reason.
  - The disassembly of three functions' return paths.
  - Clang's `cleanup` quoted from its reference.
  - The six two-vCPU judges' runs.
- **Planted faults:**
  - (1) A bare release written beside a guard in one function. The source test names its line.
  - (2) A guard removed from one take in `sync_objects.c`, its releases removed with it. The source test fails, a
    take with no guard; and on two vCPUs `semaphore_units` or `handle_values` fails, the lock left held.
  - (3) An early release removed before a `w2m_unimplemented` call. The source test fails on the path that calls it.
  - (4) The runtime word's guard releasing when its take did not take it. A run test fails: a traced bridge's word
    let go under it, or the record's takings disagree.

## done criteria

Every take of the runtime C's five locks goes through a scoped guard, every release outside one is a named early
release before a call that must not run under the lock, and the built image releases on every return path as before,
with every existing judge unchanged.

## self-review checklist

- [x] Scope names one verifiable property: no hand-paired release left outside a guard or a named early release.
- [x] context_files cite the five files, the host's guard and its test, the build and the Decision.
- [x] Emits: none, with its reason; no emit site changes.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the counts, the disassembly, the reference, the judges; numbered plants.
- [x] One concept: the locks' scoped form.

## amendment (2026-11-26)

*(Withdrawn 2026-11-26, the same hour, by the Supervisor. The line quoted below was builder-359's plant 4, live in the
tree from 22:07:17Z to 22:10:23Z; reviewer-4 and the Supervisor each read `runtime.h` in that window. The committed
line, the builder's backup `plants/runtime.h.bak` (22:07:17Z) and the tree now read
`{ w2m_let_runtime_word_go(*taken); }`. Plant 4 was caught: four two-vCPU tests (`mutex_abandoned`, `semaphore_units`,
`handle_values` and `mutual_context` on two vCPUs) failed at `run.rs` 905, "a served exit's lock taken without the
runtime word". What stays of the amendment is the class check: the source test holds that every one of the five
`*_guard_end` reads its flag, with plant 4 also failing it. The behavioral judge below is not required: plant 4's
catch shows the host's own check reaches the word's misuse on two vCPUs.)*

Reviewer-4's thirty-first review, High 1, as first written. `runtime.h` 392 reads
`static inline void runtime_word_guard_end(int *taken) { (void)taken; w2m_let_runtime_word_go(1); }`: the runtime word's
guard ignores its take's answer, and the four leaf guards (`heap.c` 96, `thread_storage.c` 88, `sync_objects.c` 103,
368) read theirs. `w2m_let_runtime_word_go(taken)` stores 0 when `taken` is nonzero (`runtime.c` 130-135). Two
consequences, each of which can put two vCPUs inside the runtime word:
- After `W2M_LET_GO_EARLY(runtime_word)`, the cleanup at the scope's end releases a second time (`sync_objects.c`
  894, 1039, 1176). Between the two stores another vCPU can take the word, and the second store releases its hold.
- A `W2M_HELD(runtime_word)` reached while this vCPU already holds the word (the take answering 0) releases it at the
  scope's end, leaving an outer holder unprotected (`sync_objects.c` 288, 412, 683, 870, 982, 1115, 1357).

The source test checks where takes and releases stand, not what the end function does. Plant 4's only failing test was
`block_judge`, a build-provenance test, so no judge of the word's behavior caught it.

- **The fix:** `runtime_word_guard_end` passes `*taken`, the leaf guards' form.
- **A behavioral judge** driving the image's own code, as Sprint 341's driver drove `w2m_logged`:
  - (a) the word, held by this vCPU before a call that declares `W2M_HELD(runtime_word)`, is still held by this vCPU's
    owner after the call returns;
  - (b) after `W2M_LET_GO_EARLY(runtime_word)` and the scope's end, the word is released exactly once (the timed
    image's `word_released` count, `runtime.c` 132, or the word's value read at each store).
- **Plant 4 becomes:** the flag ignored again, today's line. Both checks fail.
- **The class:** every `*_guard_end`, the five, reads its flag, held by the source test.
- Rerun every plant and the six two-vCPU judges on the fixed tree, then the gates.

## result

- **The guard.** Clang's attribute reference (clang.llvm.org/docs/AttributeReference.html, `cleanup`; Apple clang
  21.0.0 builds the runtime): "This attribute allows a function to be run when a local variable goes out of scope. The
  attribute takes the identifier of a function with a parameter type that is a pointer to the type with the attribute."
  `runtime.h` defines `W2M_HELD(lock)`, an `int lock_guard` set by `hold_lock()` whose cleanup is `lock_guard_end`;
  `W2M_LET_GO_EARLY(lock)`, which calls the end and empties the guard; and `W2M_LET_GO_BEFORE_STOP(lock)`, for a function
  its caller's guard holds the lock for, before `w2m_unimplemented`. The four leaf takes (`hold_heap`, `hold_slots`,
  `hold_handles`, `hold_sync`) answer 1; the runtime word's take is `w2m_take_runtime_word`'s answer, and
  `runtime_word_guard_end` passes it to `w2m_let_runtime_word_go`. `sync_objects.c`'s lazily taken word, `struct word`,
  is a guard by `WORD_WHEN_NEEDED`, its cleanup `let_word_go`. Each end calls the release it called before, timed under
  `W2M_TIME_LOCKS` as before. `heap.c`'s `hold`/`let_go` are `hold_heap`/`let_heap_go`; `runtime.c`'s compare-and-swap
  take is `claim_runtime_word`, its old name now the guard's take.
- **Hand releases.** Before, at 5ff435201, by grep of each call less its definitions: heap 5, slots 14, handle table 14,
  sync objects 35, runtime word 15 (5 `w2m_let_runtime_word_go`, 10 `let_word_go`), 83 in all. After: none bare, and 24
  named, each before `w2m_unimplemented` (the call never returns, so the guard's end never runs) or `w2m_logged` (a leaf
  never calls it; the word, before the lookups' references drop and `w2m_logged`, as before):
  - slots 9: `fls_mine` 99 (before-stop); TlsAlloc 183 (logged), 186; TlsFree 219; FlsAlloc 235 (logged), 238;
    FlsSetValue 284 (logged); FlsFree 298, 303; the rest before `w2m_unimplemented`;
  - sync objects 8: ReleaseSemaphore 703 and ReleaseMutex 730 (logged); `wait_one` 841, `wait_one_held` 877, `wait_any`
    952, `wait_any_held` 1020, `wait_all` 1075, `wait_all_held` 1151 (full waiter lists, unimplemented);
  - handle table 4: `w2m_object_inheritable` 1260, 1265; `inherit_init` 1286, 1291 (before-stop); all unimplemented;
  - runtime word 3: `wait_one_held` 894, `wait_any_held` 1039, `wait_all_held` 1176, each before `w2m_logged`.
  A wait's counter read and its park, which ran after a hand release, run after the holding's end (`need_now` and a
  `continue`; `parked` and a `break`).
- **The source test**, `crates/guest_runtime/tests/scoped_locks.rs`, in the form of `run.rs`' test: the five files' text,
  comments, literals and preprocessor lines blank; a take outside the takes' own bodies, a release outside the releases'
  and the guards' ends, a `w2m_unimplemented` or `w2m_logged` called while a leaf guard is armed (in the function or one
  it calls, through the five files), an early release on a path that calls neither, a before-stop release whose next
  call is not `w2m_unimplemented` or whose lock some caller does not hold, a `struct word` with no guard, and (the
  amendment's class) a guard's end, of the six, that does not read its flag are each named by line. One named take:
  `w2m_unimplemented`'s `claim_runtime_word`, the run ending at its request. Its own cases pass.
- **The word's behavioral judge** (the amendment's, kept though withdrawn): `run_loop.rs`
  `the_runtime_words_guard_lets_it_go_only_when_its_take_took_it_and_once` drives the image's `k32_DuplicateHandle` of a
  source naming no handle and `k32_WaitForSingleObject` of a handle to this process with no wait, in the untimed and
  timed images, the word held by vCPU owner 1 and free: 8 rows, each as Windows answers; held, the word is 1 after and
  no taking counted; free, 0 after and, timed, 1 counted.
- **The built image.** `w2m_object_close` untimed is the instructions of 5ff435201's: the not-found return's
  `stlr wzr, [x20]` at +0xa0, the close's at +0xc8; timed, both paths join one release at +0x134 before either `ret`.
  `k32_TlsFree` untimed: the unimplemented path's `stlr wzr, [x20]` at +0x70 before `bl _w2m_unimplemented`, the found
  path's at +0x17c before `b _w2m_logged`. A path checker over each function's disassembly (every path, the lock words'
  addresses followed through adrp, mov, movk and add, a compare's outcome and a `cset` carried along it, the stack's own
  slots; a scratch tool, not committed) finds every one of the 34 functions that take a lock, `wait_one` (626
  instructions, 13 release stores, 9,798 states) among them, with no end and no `w2m_logged` or `w2m_unimplemented`
  reached holding a lock, in today's and the new image, untimed and timed; on plant 2's image it names
  `k32_ReleaseSemaphore`, "tail _w2m_logged holding sync".
- **The six two-vCPU judges** pass on the tree, twice (the second after `run_loop.rs`' change): `mutex_abandoned` 1.75 s,
  `semaphore_units` 19.6 s, `lost_wakeup` 97.3 s (20 runs at 1,800,073 instructions, limit 20,000,000), `heap_blocks`
  2.9 s, `thread_slots_race` 2.0 s, `handle_values` 47.4 s.
- **Sprint 358's record remade**, `store/runs/w2m-vm-measurements-1795732027000000000`: the takings the same, heap
  256,001 and slots 640,034 on one vCPU and two, the handle table's and sync objects' within the spread of 358's two
  records; its times, made at a load of 6.40 (llama-server at 120% of a core, another process's), above 358's. Run in
  turn under one load (3.85 to 4.19), 5ff435201's five files and this tree's give overlapping times, the slots' hold p50
  on two vCPUs 250 and 333 ns against 167 and 167, the heap's hold p99 292 and 833 against 708 and 250.
- **Plant 1** (a bare `let_handles_go()` beside `object_of`'s guard): the source test failed, "a release outside a
  guard, sync_objects.c:203: if (o) o->refs++; let_handles_go();".
- **Plant 2** (`k32_ReleaseSemaphore`'s guard a bare `hold_sync()`, its early release removed): the source test failed,
  "a take with no guard, sync_objects.c:701: hold_sync();"; `semaphore_units` failed, "semaphore_units: 21 of 21 runs
  failed (one on one vCPU, 20 on two)", each "stop TimeLimit", 1 released and 0 satisfied.
- **Plant 3** (TlsFree's early release removed): the source test failed, "w2m_unimplemented called holding slots,
  thread_storage.c:220: w2m_unimplemented(W2M_API_TlsFree, index);".
- **Plant 4** (`runtime_word_guard_end` letting go whatever its take answered): the source test failed, "a guard's end
  that does not read its flag, runtime.h:392"; the word's judge failed on 4 rows, "untimed (a) DuplicateHandle, the word
  held before true: answered as Windows true, the word 0 after"; and in its first run, before the amendment, four
  two-vCPU tests failed at `run.rs` 905, "a served exit's lock taken without the runtime word".

exit 0 in 520 s	default	2026-11-26T22:39:55Z	sprint 359	5ff435201dbc21f8cdc838dae8b8949329d56bb4	7676b3409a95f7da367b58c6916e26cbd7b40417917b5625e6fa0c33078c02f9	root=3b94b8b90304861fb220386d7977e0c2b3ea8ab4	rust=all python=0 samples=0	base=304470288b07ba73f1d4370bb2f08f1b750c5331
