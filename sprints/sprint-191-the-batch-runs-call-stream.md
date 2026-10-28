# Sprint 191 — The batch run's call stream

```yaml
---
id: 191
status: active
phase: 6
pass_kind: functional
---
```

## scope

reviewer-2's High 1 and Medium 4 of its review of Sprints 185-189 (the Decision "Sequence after Sprint 190"): the
batch run on the Rust host is held by its stop and not by its answers, while the Python's answers for the same run
are on disk, `store/frontier-calls/frontier-643a6283510c-1792267359516238000.tsv` (10,516 calls, rows 0-10515, in
`scripts/call_compare.py`'s `write_tsv` form: sequence, thread, image, DLL, function, eight arguments, return value,
last error).

- **The call trace in Rust:** `vm_backend.py`'s call tracing carried into the Rust guest: the trace header at
  `TRACE_OFF` (on, seq, count, capacity, dropped; the traced images' ranges at 40), `_trace_ranges` (4097-4120: each
  traced image's import slots on thunks of its own, their owner bytes at `TRACE_OWNER_OFF`), the bridge's traced form
  (3754-3760: `w2m_traced` with the service in x5 and the thunk index in x6, `TRACE_WIDE_ARGS` and `UNTRACED`), the
  trace opening at the first compile of `call_trace_from` (2481-2487, 4589-4590), `_traced_proc` (4062-4095: the quiet
  and dynamic GetProcAddress thunks) and `traced_calls` (4133-4147). `w2m-vm --game` starts with `call_trace`
  `UnityPlayer.dll` and `mono-2.0-bdwgc.dll` from `UnityPlayer.dll!UnityMain`, as `run_frontier.py` 271-272 does, and
  writes the stream at its stop in `write_tsv`'s form.
- **The comparison:** `batch_run.rs` holds the Rust rows equal to the Python's from row 0 up to the stop, column by
  column, but for a named list of the columns that differ between runs for a reason the card states (found and quoted
  in the result, not assumed); the held file records the row the stream conforms to.
- **The guests' output** (Medium 4): `w2m-vm` writes `--stop` for a guest as for the game, `gates.sh` drops its `sed`
  and keeps each guest's stderr under build/batch_run/, and `batch_run.rs` compares each ExitProcess guest's output
  with the held one (qpc and qpc2 print a clock's reading, so theirs is held by form).

## prerequisites

- 190 (closed)

## context_files

- `src/w2m/vm_backend.py` 247-258, 2342-2348, 2478-2487, 2859, 3733-3794, 4062-4147, 4580-4590;
  `tools/guest_runtime/call_trace.c`; `scripts/run_frontier.py` 255-340; `scripts/call_compare.py` 91-93;
  `crates/w2m_vm/src/guest.rs` (`bridge`, `import_thunk`), `process.rs` (`Start::call_trace`, `TRACE_OFF`),
  `run.rs` (the compile at a dispatch miss), `src/bin/w2m-vm.rs`; `crates/w2m_vm/tests/batch_run.rs`

## signal contract

### Emits

- None: no producer runs; the judge is `batch_run.rs`.

### Invariants

- No Python file added or changed; the stored stream read, not rewritten; the guests' and the run-loop judges' results
  as before.

## artifact contract

### Files modified

- `crates/w2m_vm/src/guest.rs`, `process.rs`, `run.rs` or `compile.rs`, `modules.rs` (`proc`'s traced thunks),
  `src/bin/w2m-vm.rs`, `tests/batch_run.rs`, `store/batch_run/first-stop-2026-10-28.tsv`, `scripts/gates.sh`,
  `docs/step-4-census-2026-10-28.md` (rows 7, 8), `WORKING_AGREEMENT.md`

### Command exit codes

- `batch_run.rs`, the run-loop judges and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the Rust stream's rows equal the Python's up to the row before the stop, the differing columns
  named; whether the Rust run passed row 16 (UnityPlayer's CreateProcessW) settled by the stream; each ExitProcess
  guest's output equal to the Python host's.
- **Planted faults:** (1) the reader's return value taken from the wrong field: the comparison names row 0's return
  column; (2) the trace opened at the program's entry instead of UnityMain: the comparison names the first row; (3) a
  guest's held output changed by one character: `batch_run.rs` names the guest.

## done criteria

The batch run on the Rust host is held by the prefix of the Python's call stream it answers, and every tests/data
guest that ends by its output.

## self-review checklist

- [x] Scope names one verifiable property: the Rust stream equal to the Python's prefix.
- [x] context_files cite the Python to read first.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the batch run's call stream.

## result

- **The call trace in Rust:** `Start::call_trace` is the list of traced images and `call_trace_from` the export the
  trace opens at; `Guest` gains `make_thunk` (always a new thunk, as vm_backend 2920-2924), `trace_ranges` (each traced
  image's slots on thunks of their own, owner 1 + the image), `traced_proc` (GetProcAddress's quiet and dynamic
  thunks), `traced_calls` (the records in `write_tsv`'s form) and `with_trace` (the ranges, the trace closed until
  UnityMain's block, its address at `VMC_EXIT + 16`); the bridge takes a traced form (`w2m_traced`, the service in x5,
  the thunk index in x6, `UNTRACED` and `TRACE_WIDE_ARGS`); the host compiler opens the trace at that block; the layout
  gains the trace's offsets. `w2m-vm --game` traces UnityPlayer.dll and mono-2.0-bdwgc.dll from UnityMain and writes
  `<stop>.calls.tsv`.
- **The comparison:** the Rust stream's 4 rows equal the Python's rows 0-3 in all 15 columns (VirtualAlloc,
  VirtualProtect, InitializeCriticalSectionAndSpinCount, GetCurrentProcessId), no column named as varying; the run stops
  at row 4, CreateEventW(0, 0, 0, a name at 0xfffef760), KERNEL 33's EVENT_OPEN. The held file records 4 calls traced.
- **The guests' output:** `w2m-vm` writes a guest's stop with `--stop`, `gates.sh` keeps each guest's stderr and drops
  its `sed`, and `each_guest_that_ends_prints_what_the_python_host_printed` compares the 6 that end (model_steps,
  readied, sleeper, twoready exactly; qpc and qpc2 by the clock's form).
- **Plant 1** (`traced_calls` taking the return value from the last-error field): `the_batch_runs_calls_are_the_python_
  runs_up_to_the_stop` failed, "row 0 of the call stream differs from the Python batch run's", the return column 0.
  Restored byte for byte.
- **Plant 2** (the trace open from the start): the same test failed, "row 0 of the call stream differs from the Python
  batch run's", its row 0 "GetSystemTimeAsFileTime". Restored byte for byte.
- **Plant 3** (readied's held output "dene\n"): `each_guest_that_ends_prints_what_the_python_host_printed` failed,
  "readied's output differs from the Python host's". Restored byte for byte.
- **A race in `c_words`:** CI failed on 7839e3bba (run 40763197676), "libvmc-5124.so: file too short": the two tests
  built the C emitter at one path on two threads. `load` builds it once a process; three runs passed.
- **Departures:** the review's question of row 16 is settled by the stream (the Rust host had not reached it); one
  multi-line `sed` and one inline Python read, both recorded; every other multi-line edit used the Edit tool.
