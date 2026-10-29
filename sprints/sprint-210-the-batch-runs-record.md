# Sprint 210 — The batch run's record

```yaml
---
id: 210
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 208" (Sprint 210, the batch run's record) and the Decision "Sequence after Sprint
205" (reviewer-2's Medium 3 of its review of Sprints 199-205; census row 14): every sprint of the batch-run line has
declared "signal — none", and `w2m-vm` writes a record only for its translations (record.rs). The frozen host's
frontier writes, per run (scripts/run_frontier.py 262-266, 380-407): EXECUTION_STARTED, MEASUREMENT_RECORDED per
exit kind and the instructions a second, SERVICE_UNIMPLEMENTED when the guest stops at a service w2m lacks, and one
EXECUTION_COMPLETED (exit status exited, timed_out, killed or faulted; elapsed seconds; the trace's sha256 and line
count). Sprint 209's tail, Observation 7, adds a unit test of GetProcAddress's split between `Guest::imports` and
`Guest::proc_thunks`, which only the macOS exit ledger judges now.

- **The producer:** `w2m-vm --game` and a guest run write that session through substrate's line protocol, as
  record.rs writes the translations': SESSION_INIT, EXECUTION_STARTED, the measurements, SERVICE_UNIMPLEMENTED at a
  named stop, EXECUTION_COMPLETED, SESSION_COMPLETED; a stop included.
- **The judge, form:** EXECUTION_STARTED's fields are held against the recorded run of 2026-10-21
  (store/runs/frontier-643a6283510c-1792623423346201000/events-000001.open.jsonl): `execution_id`,
  `initial_state_sha256`, `injected_inputs_sha256`, `kill_grace_seconds` 60, `profile_id` w2m-vm, `profile_kind`
  managed_vm, `subject_kind` application, `subject_sha256` (sha256:643a6283…), `timeout_seconds` 10800; the
  measurements' names are a subset of that record's nine (`vm_exits`, `vm_compile_blocks`, `instructions_retired`,
  `instructions_per_second`, `run_seconds`, `thread_switches`, `thread_wakes`, `pause_instructions`,
  `load_average`) with the same units. The record's kinds are that record's six harness kinds less TRACE_INGESTED and
  COMPARISON_COMPLETED (the Rust host writes no guest-plane trace, census row 14), plus SERVICE_UNIMPLEMENTED at a
  stop.
- **The judge, counts:** the `vm_exits` measurements equal the exit counts in
  `store/batch_run/first-stop-2026-10-28.tsv` (hvc 1 227, hvc 4 9, hvc 8 779, hvc 11 9); SERVICE_UNIMPLEMENTED names
  the held stop; EXECUTION_COMPLETED carries the call stream's sha256 and row count (925) and says in `unobserved`
  that no guest-plane trace was written; `check_records` (T3: one completion per start) and
  `substrate replay --level 2` pass over it.
- **The thunks' test:** a unit test in guest.rs or modules.rs: GetProcAddress of an imported name answers the import
  thunk; of a name not imported, a new thunk, the same one on a second call, and `import_thunk` of that name later
  makes its own.

## prerequisites

- 209 (closed)

## context_files

- `scripts/run_frontier.py` 212-410; `src/w2m/events.py` (the tags' fields); `src/w2m/modules.py` 160-190;
  `crates/w2m_vm/src/record.rs`; `crates/events` (vocabulary 0.12); `crates/w2m_vm/src/{guest.rs, modules.rs}`;
  store/runs/frontier-643a6283510c-1792623423346201000

## signal contract

### Emits

- SESSION_INIT, EXECUTION_STARTED, MEASUREMENT_RECORDED, SERVICE_UNIMPLEMENTED, EXECUTION_COMPLETED,
  SESSION_COMPLETED: one session per `w2m-vm --game` or guest run.

### Invariants

- No vocabulary change; the tags are vocabulary 0.12's. No Python file added or changed; the stream still equals the
  reference through row 924; the exit ledger's held file unchanged.

## artifact contract

### Files modified

- `crates/w2m_vm/src/record.rs`, `crates/w2m_vm/src/bin/w2m-vm.rs`, `crates/w2m_vm/src/{guest.rs, modules.rs}`
  (the test), `scripts/gates.sh`, a test under `crates/w2m_vm/tests/`, `docs/step-4-census-2026-10-28.md` (row 14)

### Command exit codes

- the record test, the thunks' unit test, `target/release/check_records` over the new records, and
  `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the batch run's record holds `vm_exits` hvc 1 227, hvc 4 9, hvc 8 779, hvc 11 9 and ends
  EXECUTION_COMPLETED with exit status killed after SERVICE_UNIMPLEMENTED naming host request 29; EXECUTION_STARTED's
  fields equal the 10-21 record's but `execution_id`; a guest's record ends exited with its exit code; T3 holds.
- **Planted faults:** (1) EXECUTION_COMPLETED left out at a stop: `check_records`' T3 names the record; (2)
  SERVICE_UNIMPLEMENTED's service misnamed: the record test names the field; (3) `kill_grace_seconds` dropped from
  EXECUTION_STARTED: the record test names the field against the 10-21 record; (4) `proc` made to answer a non-imported
  name through `import_thunk`: the thunks' unit test fails.

## done criteria

Every `w2m-vm` batch and guest run writes its execution record, a stop included, held in form against the frozen
host's record of 2026-10-21 and in count against the run's own stop file; GetProcAddress's split has a unit test.

## self-review checklist

- [x] Scope names one verifiable property: the record's kinds and fields against the frozen host's.
- [x] context_files cite the Python to read first.
- [x] Emits: named.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and four plants, each reaching a named judge.
- [x] One concept: the batch run's record (the thunks' test is Sprint 209's owed judge, carried here by its tail).
