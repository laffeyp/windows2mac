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

## result

- **The producer:** `w2m_vm::record::execution_lines` writes `run_frontier.py`'s session from an `Execution` whose end
  is one `RunEnd` (exited with a code, timed out, a service not implemented with its return address, faulted);
  `w2m-vm --game` writes it on standard output after the stop file, and `harness/w2m-vm-game.toml` runs it as a
  Producer, its record at build/batch_run/record-<ns>. The game runs under GAME_SECONDS, 10,800 s, the bound
  EXECUTION_STARTED states; `Outcome::executed` carries the monitor's count at the stop. A request the host does not
  serve stops as `Stop::HostRequest`, a failure in serving one as `Stop::HostFailure` (a fault, not a service not
  implemented); `Guest::called_service` reads the return address at the guest's saved RSP and, through the `call
  [rip+disp32]` before it, the Windows function called.
- **The record:** SESSION_INIT; EXECUTION_STARTED equal to the 10-21 record's but `execution_id`; seven measurements,
  `instructions_retired` and `instructions_per_second` process, `run_seconds` vm (0.087 s, the VM's time alone),
  `vm_exits` hvc 1 227, hvc 4 9, hvc 8 779, hvc 11 9; SERVICE_UNIMPLEMENTED "KERNEL32.dll!MapViewOfFile" at return
  address 0x11800272bf and 3,541,969 instructions; EXECUTION_COMPLETED killed, `guest_trace` false with the empty
  trace and 0 lines (P22), the call stream's 925 rows and sha256 in `unobserved`; SESSION_COMPLETED. `check_records`
  (E1, T1, T2, T3 applied, 0 violations) and `substrate replay --level 2` pass; gates.sh runs both.
- **Where the frozen host is not followed:** it names an unserved request's service by its refusal's words
  (vm_backend 3802-3805), where vocabulary 0.12 names "DLL!Function"; the record names the function, the words kept
  only when the call does not resolve.
- **The judges:** `tests/batch_record.rs` holds the record's kinds, EXECUTION_STARTED's fields, the measurements'
  metrics, units and conventions to the 10-21 record; `vm_exits` and the stop's instructions to the frozen host's own
  run to the same stop (the exit ledger fixture); the end to the stop's kind; the service to the Windows call in
  flight at the stop; the trace fields to P22. `a_moved_field_is_named` runs in CI, as does
  `record::tests::a_run_ends_as_the_frozen_host_names_it`, which holds the four ends, the rate and the measurement ids
  (a plant, the faulted branch answering killed, failed it).
  `modules::tests::getprocaddress_keeps_its_thunks_apart_from_the_imports` holds GetProcAddress's split on the Mac.
- **Plant 1** (EXECUTION_COMPLETED left out): `check_records` exited 1, "held: T3 record-1793299278000000000:
  frontier-643a6283510c-1793299280526627000: 0 completions". Restored byte for byte.
- **Plant 2** (the service named by the refusal's words): the record test failed, "SERVICE_UNIMPLEMENTED \"host
  request 29: not served by the Rust host yet\" where the calls in flight are [\"KERNEL32.dll!MapViewOfFile\"]".
  Restored byte for byte.
- **Plant 3** (`kill_grace_seconds` dropped): the record test failed, "EXECUTION_STARTED kill_grace_seconds: null where
  the reference has 60". Restored byte for byte.
- **Plant 4** (`proc` answering a name not imported through `import_thunk`): the unit test failed at modules.rs:635,
  "Sleep's thunk is GetProcAddress's own". Restored byte for byte.
- **Closed partial:** the done criteria name every guest run, and a guest run writes no record (BLACKBOARD Deferred
  "(Sprint 210, partial)").
- **Departures:** a guest run writes no execution record: its standard output carries the skeleton's translation
  record, which `skeleton_gate.rs` holds to the Python's, and the Python's `run_guest` writes none; census row 14 says
  so. The measurements the frozen host takes by thread, by the host's time per exit, of switches, wakes, load and the
  VM compiler's causes are not written, not written as 0. Plant 3 drops `kill_grace_seconds`, as the observation
  contract names; the record test names it, not the vocabulary's validation (the field is optional).
