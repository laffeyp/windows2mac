# Sprint 219 — The batch run ends where its reference did

```yaml
---
id: 219
status: done
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 214" as annotated in Sprint 218, with reviewer-3's review of Sprint 218 (High 1,
Medium 4, Low 2, Concerns 3 and 5).

- **The problem:**
  - On the Rust host the batch run traces all 10,505 reference rows and 92 more. It then runs on to the VM's step
    budget, 486.7 s in all.
  - The frozen harness stopped its run at 34.9 s with `stop_when` (`run_frontier.py` 284-310), because past the
    reference's last log line "nothing is judged".
  - The game's unity.log reached the reference's 168 lines on the Rust host, equal to them but for timings. No test
    reads it.
- **The stop, by the frozen harness's rule:**
  - First, the game's unity.log on the bridge's drive holds the reference's line count, 168.
  - Then the run waits until each function the Windows main thread calls reaches its Windows count. Only functions
    the w2m main thread calls at all count; the comparison reports one it never calls.
  - Failing that, the run stops `REACH_AFTER_LOG` (20,000,000,000) instructions after the log reached its count.
  - Both stops are killed (`HARNESS_STOPS`).
  - The two main threads are named, where the Python used `min`. On w2m, it is the thread whose start is the
    program's entry. On Windows, it is the thread of the reference trace's first call, excluding the capture's own
    thread.
- **When it is asked:** after each host request the run answers (`run.rs`, the `Served::Answered` arm). Each request
  is a fixed point on the instruction clock, and every write to unity.log is one. The frozen host asks once a
  wall-clock second (`kernel.py` 68). Its stop therefore depends on timing, and that part is not followed.
- **What the host takes:** plain inputs: `--log-lines 168` and `--main-thread-calls <file>`. The file holds
  (image, function, count) rows for the Windows main thread, kept under `store/batch_run/`. It is written once by
  `call_compare`, which owns the reading of the reference and the contract's excluded threads. `w2m-vm` reads no
  contract and no reference run (reviewer-3's Medium 4).
- **The log judged:** `w2m-vm` copies the drive's unity.log beside the stop file. `batch_run.rs` holds it line for line
  to `store/game-runs/38216081440/unity.log`; numbers are the varying cells.
- **The stream's end:**
  - `batch_run.rs` holds the stream's row count to the held stop file.
  - It checks the in-flight file whether or not the stream runs past the frozen stream (Low 2).
  - A step-budget stop of the batch run, no longer an expected end, fails `batch_run.rs` and `batch_record.rs`
    (Concern 3).

## prerequisites

- 218 (closed)

## context_files

- `scripts/run_frontier.py` 70-74, 268-312 (`stop_when`, `REACH_AFTER_LOG`, `HARNESS_STOPS`); `src/w2m/kernel.py` 68,
  395-405
- `crates/w2m_vm/src/bin/w2m-vm.rs` (`game`, `on_one_vcpu`), `crates/w2m_vm/src/run.rs` (`Stop`, the request arm),
  `crates/w2m_vm/src/file_bridge.rs` (the drive), `crates/call_compare/src/lib.rs`
- `store/game-runs/38216081440/` (unity.log, jit_capture.bin.trace.tsv), `contracts/windows.game.calls/0.1.0.json`

## signal contract

### Emits

- EXECUTION_COMPLETED with `exit_status` "killed" and `unobserved` naming the stop: "the log holds the reference's 168
  lines and the main thread its calls", or the instruction bound with the functions short.
- COMPARISON_COMPLETED with `verdict` "conforms" over the stopped stream, and SESSION_COMPLETED with `outcome`
  "completed".

### Invariants

- The stream's rows up to the new stop equal the reference's, every cell but the varying ones.
- Two runs of one tree stop at the same row.

## artifact contract

### Files modified

- `crates/w2m_vm/` (`src/bin/w2m-vm.rs`, `src/run.rs`, `src/record.rs` if the stop needs its own kind,
  `tests/batch_run.rs`, `tests/batch_record.rs`), `crates/call_compare/` (the main-thread calls file),
  `harness/w2m-vm-game.toml`, `store/batch_run/` (the held stop file, the main-thread calls, the held log),
  `docs/step-4-census-2026-10-28.md`

### Command exit codes

- `cargo nextest run -p w2m_vm -p call_compare`, the batch run under substrate with `--strict`, `check_records` and
  `scripts/gates.sh` return 0

## observation contract

- **Expected values:**
  - The run stops on the log and the main thread's calls, or on the instruction bound with the functions short named.
  - Its rows are the reference's, and its seconds are of the frozen run's order, not 486.7 s.
  - The log equals the reference's 168 lines but for numbers.
  - Two runs give the same stop file.
- **Planted faults:**
  1. The line count taken as 167: the run stops a line early, and the held stop file names it.
  2. The main thread's counts read from another thread: the stop moves, named.
  3. A line of the held log changed: `batch_run.rs` names the line.
  4. The instruction bound removed: a run whose main thread falls short goes to the step budget, and `batch_run.rs`
     names the step-budget stop.

## done criteria

The batch run on the Rust host ends where the reference's run ended, by the same rule, at the same row on every run.
Its unity.log is held line for line to the reference's.

## self-review checklist

- [x] Scope names one verifiable property: the batch run's end, by the reference's rule, and its log.
- [x] context_files cite the frozen harness's lines and the Rust code.
- [x] Emits names the tags and values.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and four plants.
- [x] One concept: where the batch run ends, and what it has written by then.

## result

- **The stop:** `w2m_vm::batch_stop` ports `stop_when` (`run_frontier.py` 284-310) as `decide`: the log's line count
  first, then the main thread's calls, else `REACH_AFTER_LOG` instructions after the log. It is asked after each host
  request the run answers (`run.rs`, before the `Served::Answered` arm). The log's lines are counted again only when
  its size changes, as Python's `splitlines` counts them. The main thread's returned calls are counted from the trace
  record by record; a record still in flight is kept and read again. The main thread is the run's first thread
  (`guest.start.tid`). `Stop::ReferenceEnd`, `StopKind::ReferenceEnd` and `RunEnd::ReferenceEnd` record it as killed.
- **The inputs:** `w2m-vm --game` takes `--log-lines 168 --main-thread-calls
  store/batch_run/main-thread-calls-2026-10-29.tsv` and reads no contract or reference run. `call_compare
  main-thread-calls` derives the file from game run 38216081440: the thread of its first call, the capture's thread
  excluded, 217 functions and 2,999 calls. `tests/call_compare.rs` holds the stored file equal to that output.
- **The run:** it stops on "the log holds the reference's 168 lines and the main thread its calls" after 92.0 s, then
  93.4 s on a second run of the same tree. Both runs give the same stop file. The stream is 10,505 calls, every
  reference row and none past it; no call is in flight and none was dropped. The comparison against Windows conforms.
  The log, copied beside the stop file, equals the reference's 168 lines once numbers are stripped.
- **The judges:**
  - `batch_run.rs` holds the log line for line, numbers being the varying cells.
  - The in-flight file is checked on both branches.
  - A held stop of the step budget fails `batch_run.rs`, and a step-budget record fails `batch_record.rs`.
  - `batch_stop`'s unit tests hold the line count, the decision and the calls file's reading.
- **Plant 1** (`--log-lines 167` in the harness): the run stopped at 10,505 calls, its main thread not yet done at
  167 lines, and `batch_run.rs` named it: "the batch run's first stop or a guest's stop moved from
  store/batch_run/first-stop-2026-10-28.tsv". Restored byte for byte.
- **Plant 2** (`guest.start.tid + 1` as the main thread): no counted function is short, so the run stopped at 10,394
  calls. `batch_run.rs` named the stop moved, and `batch_record.rs` named "vm_exits {\"hvc 1\": 1067.0, …} where the
  held stop file holds {\"hvc 1\": 1091.0, …}". Restored byte for byte.
- **Plant 3** (line 20 of the run's log changed): "line 20 of the batch run's unity.log differs from the reference's".
  Restored byte for byte.
- **Plant 4** (the instruction bound removed from `decide`): the unit test failed, "left: None right: Some(\"the log
  holds the reference's 168 lines; the main thread short of 1 functions' calls 20000000000 instructions later\")".
  Restored byte for byte.
- **Departures:**
  1. **The lookup.** On the user's word that discovering by running what the code already says is a failure ("no
     searching for already knowable answers that you can look up at once"), one lookup compared every dispatch line of
     the frozen host (vm_backend.py, kernel.py, file_bridge.py) with the Rust host's arms. It found three requests with
     no arm: HOST_CLOCK (3), HOST_UNIMPLEMENTED (4) and THREAD_WAKE (thread request 10). All three are served as
     vm_backend 3838-3839, 3878-3883 and 3196-3199 serve them. HOST_UNIMPLEMENTED's name comes from
     `guest_runtime::services::api_names`, which `api_header` now uses too.
  2. **The gate.** `crates/w2m_vm/tests/host_requests.rs` reads those dispatch lines (56 requests) and fails on any with
     no Rust arm. A plant that removed THREAD_WAKE's arm was named: "THREAD_WAKE (src/w2m/vm_backend.py, not in
     crates/w2m_vm/src/host_request.rs)".
  3. **Files outside the list:** `crates/w2m_vm/src/guest.rs` (`trace_count`, `trace_record`, the `batch_stop` field;
     `traced_calls` reads through them), `crates/w2m_vm/src/host_request.rs`, `crates/guest_runtime/src/services.rs`,
     and the new `crates/w2m_vm/src/batch_stop.rs` and `tests/host_requests.rs`.
  4. **The run's length.** The run takes 92-93 s against the frozen host's 34.9 s; the Rust host's speed is outside
     this card.
  5. **HOST_UNIMPLEMENTED's record.** The stop's text named the service, but the record's SERVICE_UNIMPLEMENTED named one
     only when a traced image's call was in flight (`RunEnd::of`). The frozen host keeps the name and the return address,
     read from the saved RSP, in `self.unimplemented` (vm_backend 3880-3882), whatever image called. `Guest::unimplemented`
     now holds them, and `w2m-vm --game` writes SERVICE_UNIMPLEMENTED from it. A unit test holds `api_names`' order,
     id n the n-th name. The gates run then going was stopped with no pass line and run again.
- **After reviewer-3's review of Sprint 219** (the gates run then going was stopped with no pass line, and run again):
  - **Medium 1:** `host_requests.rs` reads the guest's own enums (runtime.h 69-73, 298-330) as the requests it can
    ask, and the frozen host's dispatch as its answer. It matches a name as a whole word outside comments, so `CLOSE`
    is not met by `FILE_CLOSE`. It names its one deliberate refusal, `THREAD_SIGNAL_RETIRED`. 56 requests pass; an arm
    that refuses still passes, which its doc says. The THREAD_WAKE plant, run again, was named from both sources:
    "THREAD_WAKE (runtime.h W2M_THREAD_)", "THREAD_WAKE (src/w2m/vm_backend.py)".
  - **Medium 2:** `host_request::unimplemented` holds HOST_UNIMPLEMENTED's rule. An RSP that is not mapped is a
    failure, not a return address of 0. The unit test `requests_three_and_four_answer_as_the_frozen_host` holds the
    name, the form, the return address from a planted stack word, the unmapped case, and HOST_CLOCK's count.
  - **Low 2:** `trace_record` refuses an index at or past the count.
  - **Low 3:** `batch_stop`'s doc says the bound is measured only at a request.
  - **Why the lookup rode in this card** (reviewer-3's Medium 6 of its project review; WORKING_AGREEMENT 293, a card
    over 10 files names why it is not two sprints): the user ruled mid-sprint that finding by a run what the code
    states is a failure, to be fixed then, "ok, so fix that. that is failure". The three unserved requests were defects
    of the host this card's run exercises, and the gate keeps the next run from finding the next one. It is a second
    concept all the same, about 20 files with the stop's; had it been carded apart, it would have been its own sprint
    between 219 and 220.
  - **High 1, Medium 3, Low 1:** draft card 220 is rewritten to the index's 68 families and 273 runs, each run's
    contract version, a fuller lookup, a per-run budget, and a like-for-like instructions-a-second line.
