# Sprint 358 — Each leaf lock's takings, hold and wait on the record

```yaml
---
id: 358
status: done
phase: 6
pass_kind: functional
---
```

## scope

Vocabulary 0.14 (Sprint 357) gave a record the words for each leaf lock's takings, hold and wait. No code writes them
yet. Change 1's four leaf locks carry the waits, releases, creates and heap calls that left the runtime word in
Sprints 343, 345, 355 and 356, and G6 asks for "each lock's hold and wait". The runtime word's own wait figures on two
vCPUs are a lower bound: one shared wait-start word is overwritten by a second waiter (Sprint 356, 4,349 of 16,384
waits before its change and 146 after starting past their taking; the Decision "Where the timing area's wait start per
vCPU goes" puts the fix here). This card writes every leaf lock's figures, and fixes that.

- **The leaf locks' timing.** The four takes and releases are `hold`/`let_go` (heap), `hold_slots`/`let_slots_go`,
  `hold_handles`/`let_handles_go` and `hold_sync`/`let_sync_go`.
  - When timing is on, each taking records its wait start, taken and released times by `CNTVCT_EL0`, as `process.c`
    25-27 reads it. The record goes to a per-lock area the host reads at the stop.
  - Each lock counts its takings by an atomic add, even past its area's end.
  - **No shared word in flight.** A taker keeps its own wait start in a local, not in a shared header, and writes its
    whole entry while it holds the lock. That is the shared-header fault this card fixes in the runtime word's area.
  - When timing is off, the lock's built code is unchanged. Show it by disassembly of one take, or hold the cost to a
    stated bound by measurement.
- **Switching timing on.** The way `--runtime-word` turns on the runtime word's timing (`Options::time_runtime_word`,
  `crates/w2m_vm/src/bin/w2m-vm.rs` 368-415) also turns on the leaf locks', through a word the C reads. The batch run,
  the family runs and every test that does not ask for timing must be unchanged.
- **The runtime word's wait start per vCPU.** The timing area (`crates/w2m_vm/src/monitor_code.rs` 227-241) keeps the
  wait start per vCPU, or in the taker's own register, so no other vCPU writes it. Two-vCPU waits then run backward 0
  times in 16,384, and `run_measurements` refuses any that do.
- **The record.** `run_measurements` writes on two vCPUs:
  - for each leaf lock: MEASUREMENT_RECORDED `lock_takings`, `lock_hold_ns_p50`, `lock_hold_ns_p99`,
    `lock_wait_ns_p50` and `lock_wait_ns_p99`, each with its `lock`;
  - `runtime_word_takings` beside the runtime word's figures, now trusted.

  Its rows are the guests that drive each lock: `semaphore_units` (sync objects), `handle_values` (handles), and the
  heap's and slots' two-vCPU guests of Sprints 343 and 345. The record passes `check_records`, E16 included, and is
  committed under `store/runs/`.
- **The judges.**
  - **Takings are counted, not guessed.** For each guest, a lower bound on each lock's takings, from the guest's own
    counts of the calls it made (`handle_values` claims 104,064 values, each create a taking), held by the record's
    `lock_takings`.
  - **Waits are real.** On a two-vCPU guest whose threads contend for one lock, its p99 wait is above 0. On one vCPU
    every lock's waits are 0 (no other taker), and the record shows it.
  - **The runtime word's waits:** 0 of 16,384 run backward on `semaphore_units` on two vCPUs.
- **Not here:** the scoped guard (the next card); any lock's change of class.

## prerequisites

- 357 (closed)

## context_files

- `signals/0.14.json`: MEASUREMENT_RECORDED, the six new metrics, `lock`, E16.
- `tools/guest_runtime/heap.c`, `thread_storage.c` 20-90, `sync_objects.c` (each lock's take and release),
  `process.c` 1-60 (the counter), `runtime.h`.
- `crates/w2m_vm/src/monitor_code.rs` 200-320 (the runtime word's timing area), `crates/w2m_vm/src/bin/w2m-vm.rs`
  360-430 (`--runtime-word`), and `crates/w2m_vm/src/bin/run_measurements.rs` in full, with `harness/w2m-vm-measurements.toml`.
- `crates/w2m_vm/tests/run_loop.rs`: the tests of `semaphore_units`, `handle_values`, the heap's and the slots'
  two-vCPU guests.
- `crates/guest_runtime/tests/python_image.rs` (`DEPARTURES`).
- `grep -n "Where the timing area's wait start per vCPU goes\|A card that moves a service class between locks"
  BLACKBOARD.md`: the two Decisions.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands", and 300, "Signals that bind".

## signal contract

### Emits

- MEASUREMENT_RECORDED on the committed record, with `vcpus` 2:
  - for each of `heap`, `slots`, `handles` and `sync_objects` as `lock`: `lock_takings`, `lock_hold_ns_p50`,
    `lock_hold_ns_p99`, `lock_wait_ns_p50` and `lock_wait_ns_p99`;
  - `runtime_word_takings`, beside `runtime_word_hold_ns_*` and `runtime_word_wait_ns_*`.

### Invariants

- With timing off, the batch run, the family runs on one vCPU and two, the exit ledger and every run test pass
  unchanged.
- `check_records` over every record from 2026-10-17 prints no new violation.

## artifact contract

### Files modified

- The four locks' take and release in `heap.c`, `thread_storage.c` and `sync_objects.c`, with their timing areas; the
  timing area in `monitor_code.rs`; `w2m-vm.rs`' switch; `run_measurements.rs` and its baseline; the new record under
  `store/runs/`; `DEPARTURES`; and the tests. One concept: the leaf locks' timing, which these files share.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The record's figures for each lock on two vCPUs, quoted.
  - Each guest's lower bound on takings against `lock_takings`.
  - The contended p99 wait above 0.
  - One vCPU's waits at 0.
  - The runtime word's backward waits, 0 of 16,384.
  - With timing off, one take's built code, or the cost bound.
- **Planted faults:**
  - (1) One lock's takings counter not added to. Its `lock_takings` falls below the guest's bound, and the judge fails.
  - (2) A taker's wait start taken from a shared word again. The waits run backward on two vCPUs, and
    `run_measurements` refuses the run.
  - (3) The runtime word's per-vCPU wait start put back to the shared header. Backward waits above 0, refused.
  - (4) A lock's entry written after its release. A hold reads 0 or negative, refused.
  - (5) Timing left on when not asked for. A test with timing off fails, or the cost bound is broken.

## done criteria

A two-vCPU record carries every leaf lock's takings, hold and wait. Its takings meet each guest's own counts, its
contended waits are above 0, and the runtime word's waits are trusted. With timing off, nothing changes.

## self-review checklist

- [x] Scope names one verifiable property: each leaf lock's figures on the record, the runtime word's waits trusted.
- [x] context_files cite the vocabulary, the locks, the timing area, the switch, `run_measurements` and the guests.
- [x] Emits: MEASUREMENT_RECORDED with the five `lock_*` metrics for each lock, and `runtime_word_takings`.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the figures, the bounds, the waits; numbered plants.
- [x] One concept: the leaf locks' timing on the record.

## amendment (2026-11-26)

Builder-358's observation (b), verified by the Supervisor, is a defect in a reported figure, not a gap left for
later. The runtime C's own takings of the runtime word are neither counted nor timed: `w2m_take_runtime_word`
(`runtime.c` 78-84) parks a thread, readies one and serves the host's objects for the eight services of Sprint 356,
and `Site` names four monitor-code sites only. Sprint 356 moved those takings out of the bridges into the C. So its
"after" count, 69,264 on `semaphore_units` on two vCPUs, leaves them out, and the drop it reported (95.6%) is too
large by an unmeasured amount. Its bridge figures, 15,472 to 12 of the first 16,384, count bridges alone and stand.
`runtime_word_takings`' note in 0.14 says "the times the runtime word was taken".

- **Count and time the C's takes.** `w2m_take_runtime_word`'s take and release become a site of the runtime word's
  timing area (a new `Site`, or the area's own count), with the per-vCPU wait start this card built, so
  `runtime_word_takings` counts every taking of the word.
- **Measure Sprint 356's change again.** Run `semaphore_units` on two vCPUs on this card's tree, and on the tree before
  Sprint 356, at 96360d626 (its C and `services.rs` put back by `cp`, this card's measuring kept, restored by `cp` and
  `cmp`). Quote both totals of `runtime_word_takings`, both trees' roots, and the drop. Write the figure into the
  result for the Supervisor to correct in Sprint 356's record of it.
- **Plant (6).** The new site's count removed. `runtime_word_takings` on `semaphore_units` falls below the C's own
  takes, and a judge holds that bound: at least one taking for each park or readying the guest's waits make.

## result

- **The leaf locks' timing** (`tools/guest_runtime/runtime.h`, under `#ifdef W2M_TIME_LOCKS`): `w2m_lock_take` takes
  the word as before (plain loads while held, then compare-and-swap of 0 for 1, acquire) and reads CNTVCT_EL0 (`isb;
  mrs`, as `process.c` 25-27) at the first try that fails, the wait's start, and after the swap, the taken time; both
  go to this vCPU's own monitor data words, `W2M_LOCK_TAKING_OFF` (0x510) + 16 * lock, which no other vCPU writes. An
  untaken lock's take has no wait (its wait start is its taken time), the convention of Linux's lock statistics
  (`Documentation/locking/lockstat.rst`: waittime is timed on contention). `w2m_lock_let_go` reads the released time,
  adds 1 to the lock's takings by `__atomic_fetch_add` (relaxed), counted past the area's end, and while still holding
  the lock writes the entry at the index the add gave: wait start, taken, released, and the lock word as its holder
  reads it (1); then the release store. `hold`/`let_go` (`heap.c`), `hold_slots`/`let_slots_go`
  (`thread_storage.c`), `hold_handles`/`let_handles_go` and `hold_sync`/`let_sync_go` (`sync_objects.c`) call them
  under the define and keep their old bodies in the `#else`. The four areas, 64 KB each, lie at the monitor region's
  end (`monitor_code::LOCK_AREAS` = `MONITOR_SIZE - 4 * LOCK_AREA`, past the runtime word's entries, a compile-time
  assertion), each a 64-byte header and 2,046 entries of 32 bytes; `run::read_locks` reads them at the stop.
- **Switching timing on:** `guest::runtime_defines_for(timed)` adds `W2M_TIME_LOCKS=1` to `runtime_defines()` when
  the run times its runtime word; `w2m-vm`'s `guest_of` builds the image with it, so `--runtime-word` times the runtime
  word and the leaf locks together. `Guest::new` refuses an image whose symbol `w2m_lock_timing_areas` (defined only
  under the define, `heap.c`) disagrees with `Start::time_runtime_word`. The default `runtime_defines()` is unchanged,
  so `python_image.rs`' frozen key is too.
- **Timing off, the built code unchanged:** the image built from this tree without the define is HEAD e865af826's
  image byte for byte: both 3,178,336 bytes, `cmp` silent, sha256 0885084a29a921622ec7e162751a1a96e89353ecd102e79c772fc821e1fa5df5
  (built by `guest_runtime::build` at a scratch worktree of HEAD and at the tree; keys ae251e58... and 71c0db5f...,
  the sources' text differing). The timed image is 3,186,536 bytes. So the batch run, the family runs on one and two
  vCPUs and every untimed test run the image they ran before.
- **The runtime word's wait start per vCPU** (`monitor_code.rs`): `take_runtime` stamps the wait's start and the exit
  count then into `RUNTIME_WAIT_START` (0x500) and `RUNTIME_WAIT_EXITS` (0x508) of the vCPU's own monitor data page
  (`stamp_wait_start`), past the YMM upper halves' 0x400-0x500; `record_taking` reads them there. The header's +0 and
  +16 are no longer written.
- **run_measurements** (`crates/w2m_vm/src/bin/run_measurements.rs`): semaphore_units' two-vCPU waits are trusted and
  a wait or a hold that ran backward fails the run; `runtime_word_takings` for qpc, sleeper and semaphore_units (vcpus
  2); and for each leaf lock on its guest, on two vCPUs and on one, the five `lock_*` metrics with its `lock`
  (`record::lock_measurement_line`, `None` for any other metric, E16). Every timed run's four areas are judged
  (`leaf_locks`): written equal to counted (up to 2,046); no entry whose word is not 1; no wait or hold backward; on
  one vCPU no wait. The driven lock's takings are held to the guest's own counts (`takings_bound`), and on two vCPUs
  its 99th-percentile wait must be above 0.
- **The guests:** semaphore_units (sync objects), handle_values (handles), `probes/thread_slots_race` (slots, binary
  of run 45763850660, its rows held to Windows' but `tls.overlap`), and a new guest for the heap,
  `tests/data/timed_waits/heap_blocks.c` (two threads, a start barrier, 2,000 rounds of 32 HeapFree-then-HeapAlloc
  each, marks checked; built `x86_64-w64-mingw32-gcc -O1 -nostdlib -Wl,-e,start -o heap_blocks.exe heap_blocks.c
  -lkernel32`, GCC 16.2.0), since Sprint 343's heap judge is a monitor-code driver inside `run_loop.rs` that no
  `w2m-vm` child can run; its stop is held in `store/batch_run/first-stop-2026-10-28.tsv` ("ExitProcess { code: 0 }",
  "done\n"). `w2m-vm --runtime-word`'s file adds `locks` and `counts` (the program's `.counts` words).
- **The record** `store/runs/w2m-vm-measurements-1795724698000000000` (substrate exit 0, run alone; `check_records`:
  "E16 applied 40 violations 0", "1 records; 0 violations"; `substrate replay --level 2` exit 0). On two vCPUs (vcpus
  2), ns: sync_objects (semaphore_units) takings 2,436,204, hold p50 0, p99 83, wait p50 0, p99 333; handles
  (handle_values) 4,263,969, hold 42 / 125, wait 0 / 375; heap (heap_blocks) 256,001, hold 42 / 167, wait 0 / 292;
  slots (thread_slots_race) 640,034, hold 167 / 167, wait 125 / 292. On one vCPU: takings 2,440,006, 8,515,855,
  256,001, 640,034; every lock's wait p50 and p99 0; holds p99 42, 42, 83, 42. Each over the run's first 2,046 takings.
- **Takings against the guests' counts:** sync_objects 2,436,204 >= 800,000 (releases 400,000 TRUE + 0 FALSE, waits
  satisfied + timed out 400,000); handles 4,263,969 >= 208,396 (104,064 values claimed + 104,332 closes TRUE + 0
  FALSE); heap 256,001 >= 256,000 (128,000 HeapAlloc + 128,000 HeapFree, and GetProcessHeap's one); slots 640,034 >=
  640,018 (2 x 8 x 20,000 for each of threads A and B, HOLD 8 from `thread_slots_race.c` 22, + FlsAlloc 1 + 16
  FlsSetValue + FlsFree 1).
- **The runtime word:** semaphore_units on two vCPUs, `runtime_word_takings` 69,105, variant "16384 written, 0 of them
  waits run backward"; switch wait p50 42 ns, p99 500 ns over 16,198 takings (Sprint 356: 4,349 and then 146 of
  16,384 started past their taking). qpc 14 takings, sleeper 3,508.
- **Tests:** `guest::tests::the_cs_leaf_lock_areas_are_the_hosts` (runtime.h's area size, taking offset, entry, area
  address and lock order against Rust's), `a_run_times_its_leaf_locks_exactly_when_it_times_its_runtime_word`,
  `measurements_record::each_leaf_locks_takings_hold_and_wait_are_on_the_record` (the newest record: five metrics a
  lock on two vCPUs and one, takings at least the variant's bound, two-vCPU p99 wait above 0, one-vCPU waits 0, the
  runtime word's takings with none backward); `the_two_vcpu_run_is_on_the_record` now takes the lock rows' vcpus by
  their `-one-vcpu` names. `DEPARTURES` names runtime.h and Sprint 358 in heap.c, sync_objects.c and
  thread_storage.c.
- **Plant 1** — "run_measurements: heap_blocks on 2 vCPU(s): the heap lock taken 0 times, below the 256000 its own
  counts of its calls make" (`runtime.h` `w2m_lock_let_go`: the heap's index read from `a->takings`, never added;
  exit 1). Restored by `cp`, `cmp` silent.
- **Plant 2** — "run_measurements: semaphore_units on 2 vCPU(s): the handles lock: of its first 2046 takings, 61 waits
  and 0 holds ran backward" (`w2m_lock_take` writes the wait start to the area's shared `reserved[0]`, the release
  reads it there; exit 1). Restored, `cmp` silent.
- **Plant 3** — "run_measurements: semaphore_units on 2 vCPUs: of the runtime word's first 16384 takings, 122 waits and
  0 holds ran backward" (`take_runtime` back to `stamp(a, 0, 16)`, `record_taking` reading the header's +0 and +16;
  rebuilt; exit 1). Restored, `cmp` silent, rebuilt.
- **Plant 4** — "run_measurements: sleeper: the handles lock: 4 of its first 4 entries written with the lock let go,
  its word not 1" (`w2m_lock_let_go`'s release store moved before the add and the entry; exit 1). The card foresaw "a
  hold reads 0 or negative": each entry's slot is its taker's own, from its add, and its times are its vCPU's own, so a
  late entry's times stay whole; the word read at the entry's write is what shows it. Restored, `cmp` silent.
- **Plant 5** — "the untimed image times its locks" (`runtime_defines_for` adds the define when not timed;
  `a_run_times_its_leaf_locks_exactly_when_it_times_its_runtime_word` fails at guest.rs 1870); an untimed `w2m-vm` run
  of heap_blocks: "w2m-vm: the guest: Layout(\"the runtime image's leaf locks are timed and the run's runtime word
  not timed: build it with guest::runtime_defines_for\")", exit 1. Restored, `cmp` silent, rebuilt.
- **The first gates run**, before the amendment: "exit 0 in 459 s", 2026-11-26T20:34:14Z, root=35ba360d0034a90f948a65cf5026cc5903f99698,
  kept in `store/gates/passes.tsv`.

### The amendment's result

- **The C's takings timed and counted:** `runtime.c`'s `w2m_take_runtime_word` and `w2m_let_runtime_word_go`, under
  `W2M_TIME_LOCKS`, time a taking as the monitor code does: the wait's start and the exit count then to the vCPU's own
  `RUNTIME_WAIT_START` and `RUNTIME_WAIT_EXITS`, the taken time and the exit count then to the header's +8 and +24
  after the take, and at the release, still holding the word, the entry of eight words at the takings' count with the
  site `Site::Runtime` (4, "runtime"), the count at +32 raised and the C's own count at +0 (`LOCK_C_TAKINGS`) too. The
  constants (`runtime.h` `W2M_WORD_TIMING_OFF` 0xA8BB000, `W2M_WORD_WAIT_START_OFF` 0x500, `W2M_WORD_TAKINGS` 16384,
  `W2M_WORD_SITE_RUNTIME` 4) are held to `monitor_code`'s by `guest::tests::the_cs_runtime_word_timing_is_the_monitor_codes`.
  `w2m_unimplemented`'s take is not timed: it ends the run and is never let go. The untimed image is still HEAD's byte
  for byte (sha256 0885084a29a921622ec7e162751a1a96e89353ecd102e79c772fc821e1fa5df5). `DEPARTURES` names runtime.c's
  change.
- **The judge:** `run_measurements` holds semaphore_units' two-vCPU C takings to at least its `.counts` gated (each a
  SetEvent of the manual-reset gate, which readies that event's waiters under the word, `w2m_signal`) plus watched (each
  a 1 ms wait on a thread's handle that timed out, checked under the word, `wait_one_held`), and to at most the
  word's whole count; `measurements_record`'s test holds the record's variant to it.
- **Sprint 356 measured again**, semaphore_units on two vCPUs, `w2m-vm --vcpus 2 --runtime-word`, three runs a tree:
  - before, 96360d626's `sync_objects.c`, `threads.c` and `crates/guest_runtime/src/services.rs` put back by `cp` (this
    card's timing kept; `git diff 96360d626` of the three empty), root=196eac34182e9a82183e1a99d5ac436140895112:
    `runtime_word_takings` 1,576,440, 1,576,728 and 1,576,497, the C's 0, bridges 15,452, 15,470 and 15,475 of the first
    16,384;
  - after, this tree, the three restored by `cp`, `cmp` silent, root=f12d1918f50bc0e414686fafcf09c552527ba67c: 234,454,
    233,654 and 234,465, the C's 165,420, 164,881 and 165,437, bridges 12 of the first 16,384;
  - the record `store/runs/w2m-vm-measurements-1795725897000000000` (substrate exit 0, run alone; `check_records` "E16
    applied 40 violations 0", replay exit 0; the three record tests pass on it): 235,537, of them the runtime C's
    166,136, at least 8,364 from the guest's 6,248 gate releases and 2,116 timed-out 1 ms waits, 0 of 16,384 waits
    backward; the C site's hold p50 125 ns, p99 2,333 ns, wait p50 42 ns, p99 1,708 ns over 11,427 of the first
    16,384.
  - The drop: from 1,576,566 (Sprint 356's before) to 235,537 is 85.1%, not 95.6%; 356's after, 69,264, counts the
    monitor code's sites alone (plant 6's run, the C uncounted, gave 69,042), and its bridge figures stand. This card's
    first record, `store/runs/w2m-vm-measurements-1795724698000000000` (69,105), has the same undercount and is kept as
    the run before the amendment; the newest record is the one the tests read.
- **Plant 6** — "run_measurements: semaphore_units on 2 vCPUs: the runtime C took the word 0 times, of 69042 takings,
  below the 8093 its 6248 gate releases and 1845 timed-out 1 ms waits make" (`runtime.c` `word_released` without its
  two adds; exit 1). Its first run met a fault of the judge's own, "semaphore_units: 20 .counts words, not 17" (the
  section rounds the struct's 17 words up to 20), fixed to read words 15 and 16, and the plant run again. Restored,
  `cmp` silent.
- **Plants 1 to 5 again** on the amended tree, each restored by `cp`, `cmp` silent: 1 "heap_blocks on 2 vCPU(s): the
  heap lock taken 0 times, below the 256000 its own counts of its calls make"; 2 "semaphore_units on 2 vCPU(s): the
  handles lock: of its first 2046 takings, 54 waits and 0 holds ran backward"; 3 "semaphore_units on 2 vCPUs: of the
  runtime word's first 16384 takings, 83 waits and 0 holds ran backward" (the monitor code's wait start to the
  header's +16 and +56, +0 now being the C's count); 4 "sleeper: the handles lock: 4 of its first 4 entries written
  with the lock let go, its word not 1"; 5 "the untimed image times its locks" (guest.rs 1922).
- **The timing's own cost** (reviewer-4's thirtieth review, Low 1), on this card's final tree, after the gates:
  semaphore_units on two vCPUs, five runs untimed (`--output` alone) and five timed (`--runtime-word`), interleaved,
  each the whole `w2m-vm` child's wall time from the shell (zsh `$EPOCHREALTIME`; the run prints no clock figure of
  its own), the machine's load average then 1.44, 4.14, 4.58. Untimed 0.525, 0.529, 0.528, 0.526, 0.523 s, median
  0.526 s. Timed 0.742, 0.737, 0.741, 0.735, 0.735 s, median 0.737 s. Ratio timed/untimed 1.40 (medians). Every
  run's output is "done". Satisfied waits: each timed run 400,000 (its `.counts` word 2, all of the 400,000 waits). An
  untimed `w2m-vm` run writes no counts (only `--runtime-word`'s file carries them). So the untimed counts are those of
  `run_loop.rs`' `four_threads_on_two_vcpus_keep_a_semaphores_units_and_every_wait_returns`, the same image untimed, in
  its own runs: on two vCPUs 400,000 satisfied and 0 timed out in each run printed (runs 1-7 read), and the test
  passed. The 40% covers every timed part at once: the monitor code's runtime-word stamps, the C's runtime-word site,
  and the four leaf locks' clock reads, adds and entries. The record's hold and wait figures were taken under this
  timing and are longer than an untimed run's by an amount no run here separates. G6's bound (busy above 5%, p99 wait
  above 10 µs) reads them as an upper side, as Sprint 247 stated its `--time-compile` probe's cost beside its
  figures.
- **Gates** (after the plants, on the restored tree):

exit 0 in 489 s	default	2026-11-26T20:53:44Z	sprint 358	e865af8264763875d7dd2e755e30028ddc399de3	6af074d8c454290d2458e76ee953c7747e76810c9dee317e3719bb8ce002c66b	root=02375814a44573cbeba1fd9fe83c097ee23914e7	rust=all python=0 samples=1	base=44ef3da2459b98404b24492f76e73382b8a2725f
