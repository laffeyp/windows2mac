# Sprint 219 — The batch run ends where its reference did

```yaml
---
id: 219
status: active
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
