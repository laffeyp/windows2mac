# Sprint 194 — The stream judge's fixes

```yaml
---
id: 194
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 193", reviewer-2's Mediums 2-3 and Lows 1 and 3 of its review of Sprints 190-192.
`batch_run.rs` holds every cell of the Rust stream equal to the Python's, but three repeat Python runs of the same code
differ from the reference at rows 426 and 438 (GetFileAttributesExW, columns a4-a7), so a correct Rust host would fail
there; the call in flight at the stop is dropped and never compared; and the build digest's inputs miss
`specs/x86_decode.json`, which `crates/x86_decoder/build.rs` 98 reads.

- **The varying cells:** `store/batch_run/stream-varying-2026-10-28.tsv`, the (row, column) pairs at which the four
  Python runs (the reference and `-1792269281973223000`, `-1792273674526334000`, `-1792300672353112000`) differ, derived
  by a test from the four files and compared with the committed list; `batch_run.rs` skips those cells and holds every
  other cell exactly.
- **The call in flight:** `w2m-vm` writes the records whose ready word is 0 to `<stop>.in-flight.tsv` (sequence, thread,
  label, dll, function, arguments); `batch_run.rs` holds the first to the Python's row at `ours.len()`, all but the
  return and error columns.
- **The build's inputs:** `INPUTS` gains `specs/x86_decode.json`; a unit test reads every `build.rs` under `crates/` and
  fails on a path outside `crates/` that `INPUTS` does not name.
- **The trace port:** `traced_calls` names a thunk with no name as an error in the stop file, not "?!?"; `traced_proc`'s
  caller read on an unmapped RSP is a named error, not 0 (Low 1).
- **The constants:** `TRACE_RECORDS_OFF` and `TRACE_RECORD` defined once, in `layout.rs` (Low 3).

## prerequisites

- 193 (closed)

## context_files

- `crates/w2m_vm/tests/batch_run.rs`, `crates/w2m_vm/build.rs`, `crates/w2m_vm/src/built_from.rs`,
  `crates/x86_decoder/build.rs` 95-100, `crates/w2m_vm/src/guest.rs` (`traced_calls`, `traced_proc`),
  `crates/w2m_vm/src/process.rs` (the TRACE constants), `store/frontier-calls/` (the four runs)

## signal contract

### Emits

- None: no producer runs; the judges are `batch_run.rs` and unit tests.

### Invariants

- No Python file added or changed; the stream's rows 0-15 still equal; the guests' stops as before.

## artifact contract

### Files modified

- `crates/w2m_vm/tests/batch_run.rs`, `crates/w2m_vm/build.rs`, `crates/w2m_vm/src/built_from.rs`,
  `crates/w2m_vm/src/bin/w2m-vm.rs`, `crates/w2m_vm/src/guest.rs`, `crates/w2m_vm/src/process.rs`,
  `store/batch_run/stream-varying-2026-10-28.tsv` (new)

### Command exit codes

- `batch_run.rs`, the `built_from` unit test and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the varying list holds rows 426 and 438, columns a4-a7, and nothing else; the in-flight record
  at the stop is CreateProcessW with arguments 0xa 0xfffefe64 0x60000003e, equal to the Python's row 16.
- **Planted faults:** (1) a varying pair dropped from the committed list: the derivation test names the pair; (2) the
  in-flight record's first argument changed: the stream test names row 16; (3) `specs/x86_decode.json` dropped from
  `INPUTS`: the build-inputs test names the path.

## done criteria

The stream judge passes on a host equal to the Python's at every cell the Python's own runs agree on, holds the call in
flight, and the build digest covers every file a build script reads.

## self-review checklist

- [x] Scope names one verifiable property: the stream judge holds what the Python holds fixed and no more.
- [x] context_files cite what to read first.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the stream judge's inputs.

## result

- **The varying cells:** `store/batch_run/stream-varying-2026-10-28.tsv` holds rows 426 and 438, columns 9-12 (a4-a7),
  and `the_varying_cells_are_where_the_python_runs_differ` derives the same eight cells from the reference and the three
  repeat runs; the stream test skips those cells and holds every other cell exactly.
- **The call in flight:** `traced_calls` returns the records not yet ready; `w2m-vm` writes them to
  `<stop>.in-flight.tsv` and counts them in the stop file ("calls in flight 1", held). The record, CreateProcessW with
  a0-a7 0, fffefe64, 0, 0, 1, 1000000020, 0, 0, equals the Python's row 16 in columns 0-12.
- **The build's inputs:** `INPUTS` names `specs/x86_decode.json` in `build.rs` and `built_from.rs`;
  `every_file_a_build_script_reads_is_an_input` reads each crate's `build.rs` and holds the two lists equal.
- **The trace port:** a thunk with no name and an unmapped saved RSP are named errors, as the Python's lookups raise.
- **The constants:** `TRACE_RECORDS_OFF` and `TRACE_RECORD` come from `guest_runtime::layout` only.
- **Plant 1** (pair 438, 12 dropped from the list): the derivation test failed, "store/batch_run/stream-varying-
  2026-10-28.tsv lacks row 438 column 12". Restored byte for byte.
- **Plant 2** (the in-flight record's a0 plus 1): the stream test failed, "row 16, the call in flight at the stop,
  differs from the Python batch run's". Restored byte for byte, rebuilt, the batch run run again, 4 of 4 tests pass.
- **Plant 3** (`specs/x86_decode.json` dropped from both lists): the build-inputs test failed, "a build script reads
  ../../specs/x86_decode.json, which INPUTS does not name". Restored byte for byte.
- **Departures:** the card's expected arguments were the host request's words (0xa 0xfffefe64 0x60000003e); the judge
  compares the call's a0-a7. The result section was first appended by a shell heredoc, then restored from HEAD and
  written with the Edit tool.
