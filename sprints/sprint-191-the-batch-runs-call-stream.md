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
