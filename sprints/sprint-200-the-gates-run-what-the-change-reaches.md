# Sprint 200 — The gates run what the change reaches

```yaml
---
id: 200
status: active
phase: 6
pass_kind: functional
---
```

## scope

The user's word of 2026-10-28: "running 15 minutes of tests per sprint is fucking insane", "need to test more
efficiently". Sprints 197-199's gates took 943, 969 and 984 s. Measured on 2026-10-28 on this Mac, idle: the Rust tests
under `cargo test` 347 s, of which `crates/w2m_vm/tests/case_runner.rs` 271 s (its two passes take turns behind a lock,
one VM a process); the same tests under `cargo nextest`, a process a test, 188 s (the two passes side by side, 146 and
149 s); the Python gates' pytest 427 s (2,550 tests of the frozen Python host, the slowest 144 s). The Python gates ran in
every sprint because `store/` is in their trigger, and every sprint changes `store/gates`, `store/batch_run` or
`store/family_runs`, none of which the frozen Python reads; the Rust tests ran all for the same paths.

- **nextest for every Rust run:** `rust=all` runs `cargo nextest run` and the doc tests, as `rust=crates` already does.
- **The triggers the readers set:** `store/gates/`, `store/batch_run/` and `store/family_runs/` leave the Python
  trigger and the `rust=all` trigger; a change to them runs the crates that read them (`signal_report` for
  `store/gates/`, `w2m_vm` for the other two).
- **The selection's proof:** `gates.sh --plan` prints the selection; the card's plants run it with `GATES_CHANGED` set.

## prerequisites

- 199 (closed)

## context_files

- `scripts/gates.sh` 20-130 (the selection), 135-160 (the Rust stage); the measurements above

## signal contract

### Emits

- None: the judge is `gates.sh --plan` and the gates' own run.

### Invariants

- No Python file added or changed; every judge still runs when a path it reads changes.

## artifact contract

### Files modified

- `scripts/gates.sh`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** with `GATES_CHANGED` set to a sprint's usual change (crates/w2m_vm and store/batch_run), the
  plan says Rust tests for w2m_vm's dependants and "Python gates: not run"; with `src/w2m/kernel.py` it says Python gates;
  the gates' wall time for this sprint stated beside 984 s.
- **Planted faults:** (1) `store/batch_run/` put back in the Python trigger: the plan for the usual change says "Python
  gates", named; (2) `rust=all` back on `cargo test`: the measured time states it; (3) `store/case_runner/` left out of
  every trigger: the plan for a change to it names no judge, named.

## done criteria

A sprint's gates run the judges whose inputs its change reaches, the Rust tests in parallel processes, and the time is
stated.

## self-review checklist

- [x] Scope names one verifiable property: the selection for a usual change leaves out the frozen Python.
- [x] context_files cite what to read first.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the gates run what the change reaches.
