# Sprint 368 — G3's runnable part: the game's batch run on two vCPUs, conforming as on one

```yaml
---
id: 368
status: done
phase: 6
pass_kind: functional
---
```

## scope

G3 (`docs/step-6-census-2026-10-30.md` 20): "the batch run on the wall clock on two vCPUs conforming, in several runs".
The Decision "Step 6's order after G2, weighed again at Sprint 366" puts it here, before C2's build cards, so that the
game itself judges every change after it; G3 by Sprint 370. Sprint 367 made the batch run run on two vCPUs on the wall
clock and stop by the reference's rule, in two runs.

- **The judge.** "Conforming as on one vCPU" is `call_compare`'s verdict (`harness/w2m-vm-game.toml`, the game's
  calls against the Windows reference's) under the user's Decision of 2026-10-13: a VALUE divergence (the same call on
  the paired thread with the same inputs answered differently) is a defect; a presence or count divergence on a
  thread whose earlier calls all conform is SCHEDULING, recorded and not chased, unless an earlier value divergence
  explains it; the contract's `undetermined` entries (`contracts/windows.game.calls/0.1.0.json`) stand as they are. Any
  new `undetermined` entry a two-vCPU run calls for names its determinant with its evidence and is a contract change,
  recorded as one, never a widening of the judge to let a run pass.
- **Several runs.** Five batch runs on two vCPUs through the harness (`scripts/batch-run-record.sh` with the run's vCPU
  count), each record held by `batch_record.rs` and `check_records`, each stop at the reference's row, and each
  `call_compare` verdict read under the rule above; the verdicts quoted, every divergence sorted as value or
  scheduling with its thread and call.
- **G3 in the gates.** `scripts/gates.sh` gains one batch run on two vCPUs beside today's one-vCPU run, its record and
  `call_compare` verdict held the same way, so every later card's gates judge the game on two vCPUs.
- **Sprint 367's open item.** After a run, `bin/w2m-vm.rs` 1247 `current_tid` and 1262 `called_service` read
  `guest.served.vcpu`, the last vCPU served, not the stopper's, so an Unserved stop on two vCPUs may name another
  thread's call: they read the stopping vCPU's.
- **Every failure found is fixed here**, under the user's Decision of 2026-10-18 where it can stop a run or leave a
  thread past its deadline; report each to main at once with its evidence.
- **The class first.** List every reader of the batch run's record, verdict and stop that assumes one vCPU (the
  harness file, `batch-run-record.sh`, `batch_record.rs`, `check_records`, `call_compare`, the gates' batch step), with
  file:line, and what each does on two vCPUs after this card.

## prerequisites

- 367 (closed)

## context_files

- `docs/step-6-census-2026-10-30.md` (G3's row), `harness/w2m-vm-game.toml`, `scripts/batch-run-record.sh`,
  `scripts/gates.sh` 155-220.
- `crates/w2m_vm/tests/batch_run.rs`, `crates/w2m_vm/tests/batch_record.rs`, the `call_compare` and `check_records`
  binaries' sources, `contracts/windows.game.calls/0.1.0.json`.
- `crates/w2m_vm/src/bin/w2m-vm.rs` 1080-1280 (the game's run and its stop file), `crates/w2m_vm/src/batch_stop.rs`.
- `grep -n "2026-10-13 user\|Step 6's order after G2, weighed again at Sprint 366\|The batch run's stop on several vCPUs" BLACKBOARD.md`:
  the Decisions.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None: the game's batch run on two vCPUs through the existing harness and its Producers; no emit site changes. The
  one-vCPU run's records are unchanged; a two-vCPU run's record carries `vcpus` 2 (Sprint 367).

### Invariants

- One vCPU unchanged: the batch run stopping at its held row and its `call_compare` verdict as today; the 273 family
  runs on one vCPU and two as held; the two-process guests; the exit ledger.

## artifact contract

### Files modified

- `scripts/gates.sh` (the two-vCPU batch run), `crates/w2m_vm/src/bin/w2m-vm.rs` (the stopper's thread), and the
  harness or tests only where the class list requires, reported to main when found. One concept: the game judged on
  two vCPUs.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The class list.
  - Five two-vCPU batch runs: each stop line and time, each record's `vcpus`, each `call_compare` verdict quoted, and
    each divergence sorted as value or scheduling under the user's Decision of 2026-10-13, with its thread and call.
  - The gates' two-vCPU batch run and its held verdict.
  - The one-vCPU batch run unchanged.
- **Planted faults:**
  - (1) A value fault the game meets on two vCPUs: one service answering a value that differs from the reference's
    (named in the result, chosen from the calls the runs make); the two-vCPU run's `call_compare` verdict names it as
    a value divergence, and the gates fail.
  - (2) The stopper's thread read from the last vCPU served, today's form; a test of an Unserved stop on two vCPUs
    names the wrong thread and fails.

## done criteria

The game's batch run conforms on two vCPUs in five runs, by `call_compare` under the user's Decision of 2026-10-13,
and the gates judge it on two vCPUs at every card from here; one vCPU is unchanged.

## self-review checklist

- [x] Scope names one verifiable property: the game conforming on two vCPUs in several runs.
- [x] context_files cite G3's row, the harness, the judge, the contract and the Decisions.
- [x] Emits: none, with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: five runs sorted under the rule, the gates' run; numbered plants.
- [x] One concept: the game judged on two vCPUs.

## result

**Gates:** `exit 0 in 717 s	all	2026-11-27T18:36:25Z	sprint 368	f6d228be8eff4b217edcb3690e502d3fa4999c82	d78e1fc8c0cc19acfd7e7514cd4e7e2bed750e51cd7032ddf0fbb21b2f893aa6	root=1551baddedf92b7b23c26fe30fb5e801ba2e6328	rust=all python=0 samples=1	base=00f40a6c463c5e402e0519516e2ebd75c22c12ba`; steps 712 s of the run's 717 s, unplaced 5 s; the one-vCPU batch run 148 s, the two-vCPU batch run 209 s, each `check_records` and `substrate replay` 0 s, the guests 5 s, nextest 196 s (`store/gates/steps.tsv`, the run of 2026-11-27T18:24:28Z).

**The class**, every reader of the batch run's record, verdict and stop that assumed one vCPU, and what each does on two
after this card:

- `harness/w2m-vm-game.toml` 22 and 29: the game's command passes `--exit-ledger` and `--judge-blocks`, which `w2m-vm`
  refuses above one vCPU ("2 vCPUs refused: --exit-ledger reads every exit in order", `game_clock`'s test), and names
  `build/batch_run/first-stop.tsv` and `stdout.txt`; 15 says "on one vCPU". Unchanged: `scripts/batch-run-record.sh
  --vcpus 2` (41, 47-50) writes the run file without the two flags (90-91), with `--vcpus 2`, the files renamed
  `first-stop-2-vcpus.*` and `stdout-2-vcpus.txt` for both Producers (95-98), "on 2 vCPUs on the wall clock" (99) and a
  `vcpus` baseline row (110), and refuses the run file if any of that did not take (124-128).
- `scripts/batch-run-record.sh`: took no vCPU count; `--vcpus <n>` above, refused beside `--time-compile`,
  `--instructions-at-compile` and `--cache` (55-58).
- `crates/w2m_vm/tests/batch_record.rs` 425: the newest `record-*` by name, which `record-2-vcpus-<ns>` would win (`2`
  after `1795…`), judging a two-vCPU record against the one-vCPU stop file; now `newest_record`, the prefix and digits
  alone. 367: `facts` read `first-stop.in-flight.tsv` and counted exits to the fixture or the held one-vCPU file; it
  takes its in-flight file, and `the_two_vcpu_batch_runs_record_is_held_as_the_one_vcpu_runs_is` (491) holds
  `record-2-vcpus-<ns>` in form, end, `vcpus` 2, rate, conditions, compiles and verdict, its exits to its own stop
  file's; `CONTRACT_VERSION` (31) is 0.2.0.
- `crates/w2m_vm/tests/batch_run.rs` 57: `stamped_for` knew `first-stop.tsv`'s companions alone; both stops now. 124
  (the stop file whole) and 208 (the Python stream row for row) hold one vCPU's order and counts and stay one-vCPU;
  `the_two_vcpu_batch_run_stops_at_the_references_row_with_its_log` (318) holds the two-vCPU stop line and images to
  the held file's, `vcpus` 2, and the unity.log line for line (`assert_log_is_the_references`, 344).
- `crates/call_compare/tests/call_compare.rs` 92: the Rust stream's divergences equal to the frozen host's same rows,
  an order only one vCPU keeps; one-vCPU only. `the_rust_hosts_stream_on_two_vcpus_has_no_value_divergence` (172)
  judges `first-stop-2-vcpus.*` under the user's Decision of 2026-10-13: a defect fails, each scheduling and past-cut
  divergence is printed with its w2m thread and call (`SetDivergence::w2m_thread` and `is_value`,
  `crates/call_compare/src/lib.rs` 900 and 906).
- `call_compare`'s verdict (`lib.rs` 1014 `by_cause`) and `main.rs`, `check_records`
  (`crates/w2m_vm/src/bin/check_records.rs`, its rules read per record, no vCPU count): no one-vCPU assumption,
  unchanged.
- `scripts/gates.sh` 165-189: one batch run; 190-207 add the two-vCPU run, its `check_records` and `substrate replay`.
- `crates/w2m_vm/src/bin/w2m-vm.rs` 1248 and 1264 read `guest.served.vcpu`, the vCPU served last; now
  `Guest::stopping_thread(&out)` (`guest.rs` 1388), the vCPU whose stop is the run's: `run.rs` `RunStop::first` (1433) keeps that
  vCPU's pages with its Stop (1503, 1637; the time limit and the link thread's stop give none), `Outcome::stopped_on`
  (403, 1327); `threads::current_tid_on` (262) and `Guest::called_service_on` (1360) read the pages given, the
  latter each address as that vCPU reaches it.
- `crates/w2m_vm/tests/block_judge.rs` and `exit_ledger.rs` read the one-vCPU run's files only (`whole-blocks.tsv`,
  `build/exit_ledger/rust.tsv`), which the two-vCPU run does not write: unchanged.

**A run-stopping defect, fixed here** (the user's Decision of 2026-10-18). A two-vCPU trial run stopped "import
KERNELBASE.dll!SleepConditionVariableCS" at 88.1 s, its log 64 of 168 lines
(`build/batch_run/record-2-vcpus-1795761590000000000`): a thread the second vCPU ran called a function the game
resolves by GetProcAddress and the runtime lacked. `tools/guest_runtime/slim_locks.c` 166-215 serves it on the SRW
form's phases, generations and wakes; `critical_section.c` 114-128 gives its two halves,
`w2m_critical_section_leave_once` (the section left by an owner that entered it once, else the run stops as a call
outside the rows) and `w2m_critical_section_enter`, declared in `runtime.h` 241-242;
`crates/guest_runtime/tests/python_image.rs`' DEPARTURES names `critical_section.c` and `slim_locks.c`. The guest
`tests/data/timed_waits/condition_cs.c` (`x86_64-w64-mingw32-gcc` 16.2.0, `-O2 -nostdlib -e start -s -lkernel32`)
holds five checks, each quoting Microsoft Learn's SleepConditionVariableCS page,
https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-sleepconditionvariablecs: a 0 ms sleep no
thread wakes answers zero with ERROR_TIMEOUT and the caller owning the section again; a worker that sets `waiting`
inside the section before it sleeps is woken by the main thread only once the main thread finds `waiting` under the
section ("releases the specified critical section as an atomic operation"), so the wake reaches a sleeping thread by
the page's own rule and not by a 20 ms delay, as the first form had it; and the section is free at the end. It ends
ExitProcess 0 in 3 of 3 runs on one vCPU and 3 of 3 on two; `store/batch_run/first-stop-2026-10-28.tsv` 41 holds its
stop in the gates' guests.

**The class that stop names**: KERNEL32 imports UnityPlayer.dll or mono-2.0-bdwgc.dll declares that the Windows batch
run called 0 times (`store/game-runs/38006665419/jit_capture.bin.imports.tsv`) and that no service in
`tools/guest_runtime` serves, 108 functions in 140 rows, each listed with its importer, reference calls and kind in
`docs/unserved-declared-imports-2026-11-27.md` with the rule and the grep: 25 waits and threads, 31 files, 15 other,
10 memory, 9 console, 9 locale, 9 unwind. None is served here. The next cards serve 26 first, since a two-vCPU
schedule reaches waits the reference's did not: mono-2.0-bdwgc.dll's CreateMutexW, OpenMutexW, OpenEventW,
OpenSemaphoreW, SetThreadContext, DisableThreadLibraryCalls, GetNumberOfConsoleInputEvents (a console function by
kind) and QueueUserAPC; UnityPlayer.dll's CreateTimerQueue, CreateTimerQueueTimer, ChangeTimerQueueTimer,
DeleteTimerQueueTimer, CreateWaitableTimerA, SetWaitableTimer, OpenEventA, Thread32First, Thread32Next,
RegisterWaitForSingleObject, UnregisterWait, UnregisterWaitEx, CreateIoCompletionPort, GetQueuedCompletionStatus,
CreateToolhelp32Snapshot (which Thread32First and Thread32Next take) and QueryDepthSList; FreeLibraryAndExitThread and
SignalObjectAndWait from both. Of these, 17 were the review's list; the rule found the other nine and the 82 outside
waits.

**A contract change.** Trial run `record-2-vcpus-1795758688000000000` (05:51:28-05:54:23 UTC), judged under 0.1.0, got
"discrepancy": "UnityPlayer.dll!HeapReAlloc call 2 a3: Windows 0x2017, w2m 0x817: 11 of 11 reference executions
disagree (run 38216081440 seq 3267)" and "call 3 a3: Windows 0x817, w2m 0x2017: 9 of 11 reference executions disagree,
2 do not make the call (run 38216081440 seq 3268)", on w2m thread 88, named BatchDeleteObjects. Windows runs disagree on
that order on their BatchDeleteObjects threads: 0x817 0x1017 0x2017 0x817 in run 38216081440 (seq 3265-3268), 0x817
0x1017 0x817 0x2017 in lap-20261014T012302244Z-cb542225-04 (seq 3298-3301) and lap-20261014T023701004Z-3f1cf08d-04 (seq
3285-3288), the second 0x817 another block (a2); seq 3268 is the call the user's Decision of 2026-10-13 names
scheduling. `contracts/windows.game.calls/0.2.0.json` is 0.1.0 with one `undetermined` entry scoped to a thread,
"UnityPlayer.dll!HeapReAlloc@BatchDeleteObjects a3" (89), its determinant and those runs, and a `history` (91) naming
the nine commits that edited 0.1.0 in place, a2b27d07f to 91f6e329f; 0.1.0.json stays as 91f6e329f left it
(`the_game_calls_contract_0_1_0_is_unchanged`, `call_compare.rs` 290), a minor bump with the old file kept as the
Decision of 2026-10-07 has it. `call_compare` reads 0.2.0 (`lib.rs` 1190) and leaves a scoped key's field uncompared
only where the Windows call's thread and the w2m call's thread both carry that SetThreadDescription name (767-773,
`undetermined_key` 1278); the main thread's HeapReAlloc sizes, 0x100 to 0x800, stay judged
(`a_size_left_out_on_batchdeleteobjects_is_still_judged_on_the_main_thread`, `call_compare.rs` 255). The frozen host's
stream still gives "conforms".

**The five runs**, `scripts/batch-run-record.sh build/batch_run/record-2-vcpus-<ns> --vcpus 2` on the final tree (the
id test's pin, added after them, changes a test alone), each record copied to `store/runs/w2m-vm-game-<ns>` (held by
`every_stored_rust_batch_record_holds_its_rate`), each stop "ReferenceEnd: the log holds the reference's 168 lines and
the main thread its calls", `vcpus 2` on its stop file and on EXECUTION_STARTED and every MEASUREMENT_RECORDED,
`check_records` "1 records; 0 violations", and the four two-vCPU tests (the three above and the stored records' rate)
4 passed on each:

| record ns | UTC | seconds | calls | verdict | defects | past the cut | scheduling | Windows varies | BatchDeleteObjects a3 |
|---|---|---|---|---|---|---|---|---|---|
| 1795802342000000000 | 17:59:02-18:02:56 | 233.4 | 10497 | conforms | 0 | 0 | 0 | 23 | 817 1017 2017 817 |
| 1795802591000000000 | 18:03:11-18:06:08 | 176.6 | 10488 | conforms | 0 | 0 | 0 | 23 | 817 1017 817 2017 |
| 1795802779000000000 | 18:06:19-18:09:16 | 176.6 | 10483 | conforms | 0 | 0 | 0 | 24 | 817 1017 2017 |
| 1795802967000000000 | 18:09:27-18:12:23 | 175.2 | 10574 | conforms | 0 | 0 | 0 | 23 | 817 1017 2017 817 |
| 1795803156000000000 | 18:12:36-18:15:36 | 179.2 | 10479 | conforms | 0 | 0 | 0 | 24 | 817 1017 2017 |

No divergence is left to sort as value or scheduling in any of the five. Both BatchDeleteObjects orders occur (w2m
thread 88, named so in each), the first in two runs and the second in one, and two runs make three reallocations, as
laptop laps c8814547-02 and -07 do. Each unity.log is the reference's 168 lines line for line. The first run's 233.4
s includes building the runtime image under its new ids. The trial runs, kept out of `store/runs/`, their records
under `plants/368/trial-runs/`: every two-vCPU run judged under 0.1.0 or on the runtime without
SleepConditionVariableCS; 1795762175000000000 (06:49:35 UTC, conforms, its call files overwritten by the next run
before its divergences were listed); 1795762358000000000 (started 06:52:38 UTC, its events stamped 10:56:34 UTC against
the run's own 207.8 s: the host slept about four hours inside it, a condition its baseline does not name); and
1795798719000000000, 1795799207000000000, 1795799391000000000, 1795799577000000000 and 1795799762000000000 (16:58-17:19
UTC, each conforms), taken while the API ids were still numbered in name order, before the fix below.

**A second defect the gates found, fixed here.** The first gates run (17:34-17:44 UTC) failed
`exit_ledger::each_process_parts_from_the_frozen_host_where_held` ("where the Rust host parts from the frozen host
moved from store/exit_ledger/differences-2026-10-29.tsv"): 1,130 new cells, every one the `process` column (the
process area's sha256), main rows 41-1022 and child 4100's rows 41-189, the 29 held lines all still found. With the
three runtime files at HEAD and every other change of the sprint in place, the ledger matched the held file. The W2M_API
ids were numbered in name order (`crates/guest_runtime/src/services.rs`, `api_names` before this card), so the new
SleepConditionVariableCS, 231st by name, renumbered the 69 names after it. `w2m_logged` (`tools/guest_runtime/runtime.c`
148-158) writes the id into the call ring and bumps `per_api[id]`, and the ring lies at process offset 0x5000
(`crates/guest_layout/src/lib.rs` 100), inside the hashed area, so from the first logged call of a renumbered API, main
row 41's WideCharToMultiByte (thunk 0x7ffe00003370), every hash moved. The readers of the ids' order: `api_header`
writing `api_ids.h` for every build (`crates/guest_runtime/src/lib.rs` 143), `runtime.h` 83, 92 and 227
(`W2M_RING_APIS` 512, `per_api`, the include), `w2m_logged` and `w2m_unimplemented` (`runtime.c` 137-158),
HOST_UNIMPLEMENTED naming its service by id (`crates/w2m_vm/src/host_request.rs` 432-445), and the tests that read
ids by position (`threads.rs` 2136, `run_loop.rs` 1518-1530 and 2653-2656). The call trace keys on thunk indices and the
record writes no per-API counts. The ids are now fixed: `crates/guest_runtime/src/api_ids.txt`, "id name" a line,
space-separated, gives ids 1-299 as HEAD's, the names in name order, equal one for one to the frozen tag's 299
(measured by the crate's own `api_names` over both trees), and 300 to SleepConditionVariableCS. `api_names` and
`api_header` (`services.rs` 274-340) read it through `include_str!`. A service it lacks fails the build naming the id
it takes next ("add \"301 Name\" to crates/guest_runtime/src/api_ids.txt"), and a table that skips or repeats an id, or
reaches 512, is refused. `the_api_ids_at_head_are_unchanged_and_a_new_service_takes_the_next` (722) pins rows 1-299
whole by the sha256 of their names one a line, ab180912c4dd52e895e00044b2fbeedf8446a21d5d64fa4d21e15c818d0dd512
(HEAD's list's and the frozen tag's alike), so no row is renamed or reused in place and a new name takes an id above
299. It also holds them to name order and eight to frozen numbers written into the test, and holds `api_header` over
the runtime's own C to every row. A duplicate id is refused as not the next ("api_ids.txt: Exception has id 2, where 3
is next", the toy test). The frozen image is unchanged: `the_image_rust_builds_is_the_python_builds_byte_for_byte`
passes on images built anew, the old build cache moved aside to the scratchpad. On the new ids a one-vCPU batch run
stopped at ReferenceEnd and the exit-ledger test passes on the differences file as HEAD holds it. `cache_key` does not
hash the header and cannot without moving the frozen key; that stands because the test lets the table only grow and a
new service always changes the C, which the key hashes.

**The handles design's clause** (`docs/design-handles-and-sync-objects-2026-11-26.md` 422-423: "A trace shows the game
calling EnterCriticalSection, LeaveCriticalSection, the SRW locks or the condition variables from several vCPUs while
they stay on the word"). In each of the five traces the game calls EnterCriticalSection and TryEnterCriticalSection
1,082 to 1,087 times on 28 threads, LeaveCriticalSection 954 to 959 times on 28, the SRW locks 328 times on 3 and the
condition variables 64 times on 2, floors, since the trace keeps a function's first 32 calls on a thread. A trace
names each call's thread and no vCPU, and the record no thread's vCPUs, so the vCPUs were measured once outside the
five: a counter of exits by thread on each vCPU in `run.rs`' served-exit path, two runs on two vCPUs, the change
restored (cmp 0, live 17:25:09-17:31:42 UTC) and the release binaries rebuilt. Of the 28 threads that call those
functions, 22 had exits served on both vCPUs in each run (2,102 of 2,436 traced calls, and 2,096 of 2,424); in the
second, four ran only on vCPU 1 (threads 60, 68, 80 and 84, 192 calls) and two only on vCPU 0 (36 and 40, 136 calls),
so the calls come from both vCPUs, each under the runtime word. The clause fired: their own design, keyed by guest
address, enters step 6's order, its place set by card 369's measurement.

**A proposal for the next vocabulary card**: COMPARISON_COMPLETED's `contract_sha256`, the sha256 of the contract file
the comparator read (PAYLOAD_FIELD_PROPOSED). A version alone did not pin the file: 0.1.0 was edited in place by nine
commits, and a record that names 0.1.0 does not say which of the nine judged it.

**Slips in this sprint.** One line of `crates/w2m_vm/tests/batch_record.rs` was appended by `sed` with an escaped
newline, against the rule that a multi-line edit goes through the Edit tool; its text is right. And
`tests/data/timed_waits/condition_cs.c` was rewritten whole with the Write tool where the Edit tool was due; its text is
the one intended. A scratch program that listed the API ids wrote `api-names.txt` into the repository root and into
the frozen export under `build/`; both files were moved to the scratchpad before any gate ran. The five two-vCPU runs
and plant 1 were first taken before the ids were fixed, and were taken again after.

- **Plant 1** "value divergence, w2m thread 4: UnityPlayer.dll!SetConsoleCtrlHandler call 0 ret: Windows 0x1, w2m 0x2:
  11 of 11 reference executions disagree (run 38216081440 seq 419)", `cargo nextest run` of the gates' two-vCPU tests
  under `W2M_REQUIRE_CASE_RUNS=1` exit 100; live 18:15:59-18:19:13 UTC:
  - `tools/guest_runtime/processes.c` 213, SetConsoleCtrlHandler's add, answering 2 for TRUE's 1 (a BOOL, judged in
    its 4 bytes), backed up to `plants/368/processes.c.backup-c`; `grep` showed the line.
  - `scripts/batch-run-record.sh build/batch_run/record-2-vcpus-1795803369000000000 --vcpus 2`: stop ReferenceEnd,
    COMPARISON_COMPLETED "discrepancy", its first divergence the line above; `batch_record.rs`'s two-vCPU test failed
    too, "COMPARISON_COMPLETED verdict "discrepancy"". The same plant on the ids in name order (17:19:27-17:22:45 UTC)
    failed the same way.
  - Restored by `cp`, `cmp` 0. A first form, `k32_GetACP` answering 0x4e5 (builder-368, 06:13:50-06:14:22 UTC), stopped
    the run at "HostRequest op 4: IsValidCodePage argument form 0x4e5" before the call returned, and `call_compare` said
    "conforms" over the stopped stream; `batch_record.rs` caught it by the stop. Kept as an observation, replaced.
- **Plant 2** "the stopping vCPU's thread and call, not those of vCPU 0, the vCPU served last (thread 4) left: (4,
  Some((0, None))) right: (8, Some((5368713222, Some("KERNEL32.dll!ExitProcess"))))", `cargo nextest run -p w2m_vm
  --lib -E 'test(an_unserved_stop_on_two_vcpus)'` exit 100; live 17:22:54-17:23:10 UTC:
  - `crates/w2m_vm/src/guest.rs` 1389, `stopping_thread`'s pages `self.served.vcpu`, the form before; backed up to
    `plants/368/guest.rs.backup-b`; `grep` showed the line.
  - Restored by `cp`, `cmp` 0.
- **Plant 3**, beyond the card's two, the scope: "a main-thread HeapReAlloc size changed is a value divergence: []",
  `cargo nextest run -p call_compare -E 'test(a_size_left_out_on_batchdeleteobjects_is_still_judged_on_the_main_thread)'`
  exit 100; live 17:23:31-17:23:42 UTC:
  - `crates/call_compare/src/lib.rs` 767, the undetermined key matched without its thread, the wide entry's form;
    backed up to `plants/368/call_compare-lib.rs.backup-b`; `grep` showed the line.
  - Restored by `cp`, `cmp` 0.
- **Plant 4**, beyond the card's two, the ids' numbering: "SleepConditionVariableSRW's W2M_API_ id moved from 231, its
  id at HEAD (api_ids.txt) left: Some(232) right: Some(231)", `cargo nextest run -p guest_runtime --lib -E
  'test(the_api_ids_at_head_are_unchanged_and_a_new_service_takes_the_next)'` exit 100; live 17:54:28-17:54:37 UTC:
  - `crates/guest_runtime/src/services.rs` `api_names_in` answering the names in name order, the numbering before this
    card, backed up to `plants/368/services.rs.backup`; `grep` showed the line. The exit-ledger test caught the same
    numbering in the gates run of 17:34 UTC, above.
  - Restored by `cp`, `cmp` 0.
- **Plant 5**, beyond the card's two, a reused id: "api_ids.txt's rows 1-299 are not HEAD's left:
  "84dbf2813d0f99975e2b746f01b22f284cf28b09c6804f24cca57a5021bbbbc3" right:
  "ab180912c4dd52e895e00044b2fbeedf8446a21d5d64fa4d21e15c818d0dd512"", the same test exit 100; live 18:21:51-18:22:00
  UTC:
  - `crates/guest_runtime/src/api_ids.txt` row 150, IsDebuggerPresent renamed in place to IsDebuggerAttached, a name
    that keeps the rows in name order, backed up to `plants/368/api_ids.txt.backup`; `grep` showed the line.
  - Restored by `cp`, `cmp` 0.

**Gates runs.** 17:34-17:44 UTC failed at the exit ledger, above, and found the second defect. A run started at
18:19:32 UTC was stopped in its batch-run step at about 18:21 UTC, its process group killed and no gates, nextest or
w2m-vm process left, because the pin was still to be added to the gated tree. The row at the head of this result is
the run on the finished tree.

Plant backups, logs, the five runs' files and the trial runs:
`/private/tmp/claude-501/-Users-peterlaffey-port/18efb1a6-9978-416c-904b-05e124030429/scratchpad/plants/368/`.
