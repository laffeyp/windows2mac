# Sprint 150 — The case runner on a record: its verdicts as the differential's events

```yaml
---
id: 150
status: done
phase: 6
pass_kind: functional
---
```

## scope

The link-3 review's Medium 5: four functional sprints whose code runs 290,548 cases put nothing on a record, and each
Signal Report reads "(no record)"; WORKING_AGREEMENT's "Signals that bind" says "None new" is not an Emits for code
that runs. The judge the case runner replaces, `scripts/vm_oracle.py --record`, put its verdicts on a substrate record
through `src/w2m/differential.py` (285 lines, read in full): one unit per instruction class, the Intel CPU under
Windows the reference.

A Rust binary, `case_runs` (in `crates/w2m_vm`, macOS only), runs as a substrate Producer from a run file,
`harness/case-runs.toml`, and writes `differential.run`'s sequence in the vocabulary's types (`crates/events`):

- `SESSION_INIT` (purpose `conformance`, profile the Intel runs' CPU);
- per Intel run, `EXECUTION_STARTED` and `EXECUTION_COMPLETED` (subject `candidate_runtime`, profile `managed_vm`, the
  run's case file sha256 the subject, its result file sha256 the injected inputs);
- per instruction class judged (the unit), `CANDIDATE_GENERATED` (the class's rule module's sha256 from the frozen
  registry map; the whole registry otherwise), `COMPARISON_COMPLETED` (contract `w2m.guest_state.fault_context` 0.2.0,
  relation `exact`, as vm_oracle's record), then `CANDIDATE_ACCEPTED`, or `CANDIDATE_REJECTED`, `DISCREPANCY_REPORTED`
  and `DISCREPANCY_ROUTED` (`implementation_repair`);
- `DISCREPANCY_RESOLVED` for each discrepancy the ledger (`store/runs/discrepancy-ledger.json`, read as data) holds open
  on an accepted unit of this differential;
- `SESSION_COMPLETED`.

**A difference from the Python, by necessity.** `differential.py` started each step from the event before it through
substrate triggers with predicates. A run file's triggers take no predicate (`substrate/adapters/topology_file.py`:
`predicate=lambda _ctx: True`), and adding one would be writing Python in substrate. So one Producer writes the whole
sequence in order: each event's cause on the record is the Producer's start, not the event before it. The vocabulary's
rules check order and payload matches (`scripts/check_records.py`), which the sequence keeps.

The record's baseline names the differential (`differential = "vm-oracle"`), the fixture's sha256, and the runs, so
`differential.rebuild_ledger` reads it.

## prerequisites

- 149 (closed)

## context_files

- `src/w2m/differential.py` (read in full); `scripts/vm_oracle.py` 261-334; `substrate/docs/process-producer.md` (read
  in full); `substrate/src/substrate/adapters/topology_file.py`
- `crates/events`, `crates/session_events/src/main.rs`, `harness/session-check.toml`; `crates/w2m_vm/src/cases.rs`

## signal contract

### Emits

- `SESSION_INIT` {vocab_version 0.10, profile the CPU of the Intel runs, purpose conformance}
- `EXECUTION_STARTED` / `EXECUTION_COMPLETED` {execution_id `case-run-<run id>`, exit_status exited} per run, 22
- `CANDIDATE_GENERATED`, `COMPARISON_COMPLETED` {verdict conforms}, `CANDIDATE_ACCEPTED` per judged class, 387 each
  on the current tree
- `CANDIDATE_REJECTED`, `DISCREPANCY_REPORTED`, `DISCREPANCY_ROUTED` per class with a difference: 0 on the current tree
- `SESSION_COMPLETED` {outcome completed, accepted_count 387, discrepancy_count 0}

### Consumes

The case files, the downloaded Windows results, the fixture (Sprint 149's inputs).

### Invariants

- No Python file added or changed.

## artifact contract

### Files created

- `crates/w2m_vm/src/bin/case_runs.rs`, `crates/w2m_vm/src/case_inputs.rs`, `harness/case-runs.toml`, a record under
  `store/runs/`, its 22 traces under `store/case_runner/traces/`

### Files modified

- `crates/w2m_vm/` (`Cargo.toml`; `tests/case_runner.rs` may share its input loading through the crate),
  `WORKING_AGREEMENT.md`, `BLACKBOARD.md` (Sprint 149's CI run, in the tail entry)

### Content assertions

- Each function cites the Python lines it carries.

### Command exit codes

- `uv run substrate run --file harness/case-runs.toml --strict` and `uv run substrate replay <root> --level 2` return 0
- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the record holds the counts above; `scripts/check_records.py` passes it; the Signal Report for
  Sprint 150 reads the record's trace, not "(no record)".
- **Planted fault:** the binary run with the CF and ZF registers swapped (`--plant-swap-flags`): the record holds
  `DISCREPANCY_REPORTED` events whose first divergence names a CF class, and `SESSION_COMPLETED` says
  `completed_with_discrepancies`. The planted record is not committed.

## done criteria

The case runner's verdicts stand on a substrate record in the vocabulary's events, and its Signal Report shows them.

## self-review checklist

- [x] Scope names one verifiable property: the verdicts on a record.
- [x] context_files cite the Python carried and the substrate surface used.
- [x] Emits: each tag with its expected counts.
- [x] Artifact contract gradable by command.
- [x] Observation contract: expected values and a planted fault.
- [x] One concept: the runner's record.

## result

- `crates/w2m_vm/src/bin/case_runs.rs` writes the sequence; `crates/w2m_vm/src/case_inputs.rs` holds the inputs the
  test and the binary share (the runs, the pinned fixture, the expected counts, the registry map, the case VM);
  `harness/case-runs.toml` runs it. The run file calls the binary through `scripts/sign_and_run.sh`: the first run
  failed at `hv_vm_create` (-85377017, HV_DENIED), since substrate starts the binary unsigned.
- Amended during the sprint, by the record check: rule T4 holds every `EXECUTION_COMPLETED` to exactly one
  `TRACE_INGESTED` or `TRACE_REJECTED` of its `trace_sha256` before the session ends. The first record named the empty
  hash for all 22 runs and ingested none, so all 22 were held. Each run now has a trace: one JSON line per class judged
  (the run, the class, its count, its difference lines), kept as `store/case_runner/traces/<sha256>.jsonl`; its
  `EXECUTION_COMPLETED` names it, says in `unobserved` that it is the runner's verdict and not guest output, and one
  `TRACE_INGESTED` follows.
- The record (`store/runs/case-runner-<ns>`): SESSION_INIT 1, EXECUTION_STARTED 22, EXECUTION_COMPLETED 22,
  TRACE_INGESTED 22, CANDIDATE_GENERATED 387, COMPARISON_COMPLETED 387 (conforms), CANDIDATE_ACCEPTED 387,
  SESSION_COMPLETED 1 (completed; accepted 387, discrepancies 0; 64.8 s). `substrate replay --level 2` exits 0;
  `scripts/check_records.py --since 2026-10-25` holds none of its events.
- Planted fault, changed from the card: in place of swapping the VM's flag registers (which needs a hook in the
  runner), `--plant-flip-cf` flips CF in every completed case of the first run's Windows results, so the real judge
  finds real differences. The planted record (in the scratchpad, not committed) holds 107 classes rejected, reported
  and routed, the first divergence "ADD: ADD CF: 1212 cases; run 38011858047: case 0: … vm 1, cpu 0", and
  SESSION_COMPLETED `completed_with_discrepancies` (accepted 280, discrepancies 107). Its one trace written to the
  store was removed.
- Amended during the sprint, by the gates: the first gates run failed five pytest tests. Three read every
  `store/traces/*.jsonl` as a guest trace (`KeyError: 'execution_id'`), where the runner's traces then lived; they
  moved to `store/case_runner/traces/`. One, `tests/test_differential.py`, holds every record whose baseline names
  `differential` to `differential.py`'s causal chain (`KeyError: 'units'`); the run file's baseline now names
  `judge = "case-runner"`, and the record was made again. The fifth, `test_timed_waits`'s retried context read, hunts
  a race its own comment records as 0 hits in 10 runs under the gates' load; it passed alone in 0.23 s.
- CI does not run `case_runs` (Linux: it prints that it needs Hypervisor.framework and exits 2); it builds it.
