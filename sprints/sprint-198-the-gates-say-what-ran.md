# Sprint 198 — The gates say what ran

```yaml
---
id: 198
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 196", reviewer-2's Medium 2 and Low 2 of its review of Sprints 193-196.
`signal_report --check` holds "multi-vCPU: k of 3 passed" to the stage's passed rows inside the cited run, so a run
whose Python gates did not run (Sprint 195: "0 of 3 passed") and a run whose three tests all failed state the same
thing and pass alike; and `scripts/gates.sh` 81-84 chooses the Python stage by paths that leave out `gates.sh` itself,
so a change to the Python or multi-vCPU stage could skip the code it changed.

- **The stages in the pass line:** the pass line gains an eighth column, the stages run (`rust=<all|crates|none>
  python=<0|1> samples=<0|1>`).
- **The claim held to the stages:** from `FIRST_STAGED_SPRINT` (198) a closed entry states "multi-vCPU: not run (Python
  gates not run)" when its run's column says python=0, with no stage row in the run's window; and "multi-vCPU: k of 3
  passed" when python=1, with exactly three stage rows in the window and k of them passed; any other pairing is named.
- **gates.sh runs every stage for itself:** `scripts/gates.sh` and `scripts/gated-tree.sh` join the paths that set
  `all=1`.

## prerequisites

- 197 (closed)

## context_files

- `scripts/gates.sh` 15-110, 220-265; `crates/signal_report/src/main.rs` (`multi_vcpu_count`, `FIRST_WALKED_SPRINT`);
  `crates/signal_report/tests/check.rs`

## signal contract

### Emits

- None: the judges are `signal_report`'s tests.

### Invariants

- No Python file added or changed; `--check` passes over Sprints 108-197 as before.

## artifact contract

### Files modified

- `scripts/gates.sh`, `crates/signal_report/src/main.rs`, `crates/signal_report/tests/check.rs`

### Command exit codes

- `signal_report --check`, its tests and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** this sprint's own pass line carries `python=1` (gates.sh changed sets all=1), its entry states
  "multi-vCPU: 3 of 3 passed", and `--check` passes.
- **Planted faults:** (1) python=0 with "multi-vCPU: 0 of 3 passed": `--check` names the stated count against the
  stages; (2) python=1 with two rows in the window: `--check` names the row count; (3) gates.sh dropped from the all=1
  pattern: a test that runs the pattern over "scripts/gates.sh" names it.

## done criteria

A closed entry's multi-vCPU sentence says whether the stage ran, and the check holds it to the stages the pass line
records.

## self-review checklist

- [x] Scope names one verifiable property: the claim is held to the stages that ran.
- [x] context_files cite what to read first.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the gates say what ran.
