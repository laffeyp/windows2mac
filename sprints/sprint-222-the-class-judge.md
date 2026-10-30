# Sprint 222 — The class judge

```yaml
---
id: 222
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 214" as annotated at Sprint 221 (reviewer-3's Medium 3 of its refreshed project
review), and plan step 4's gate item "456 of 458 classes" (census row 3, "absent" on Rust).

- **What the frozen judge did:** `scripts/class_run.py` judged each of the 458 classes in its own substrate run and
  wrote `store/runs/class-runs.json`: per class its record, its rule's sha256, its case count, two verdicts (`model`,
  the Python rules interpreter; `managed_vm`, the VM) and its event counts. 456 conform in both; FCOS and FSIN are
  discrepancies in both.
- **What the Rust side holds, by lookup (2026-10-29):**
  - The 21 reference runs `class_run.py` names (`REFERENCES`, 54) are all under `build/probe-runs` with
    `result-1.bin`.
  - They span the four profiles census row 3 counts: 8573C on Windows Server 2025 (10 runs) and 2022 (9), 8370C (1)
    and 6973P-C (1).
  - `case_runs` (`crates/w2m_vm/src/bin/case_runs.rs`) judges one profile a session, over every downloaded run on that
    profile, not the class's own sets (`case_sets_of`, 85-91). It writes no class index.
- **The judge:**
  - For each class in the index, the cases of the reference sets that cover it, oldest first, judged by the VM through
    the Rust case runner.
  - The verdict is `managed_vm`'s. The `model` executor is the Python rules interpreter, which moves under `oracle/`
    with the rest of the frozen code. It has no Rust counterpart to port, and the card does not make one.
  - One record per class in `class_run.py`'s event order. The Rust index, `store/case_runner/class-runs-<date>.json`,
    is in the frozen index's form less `model`.
- **The gate:** a test holds the Rust index to the frozen one, class by class: the same 458 classes, the same case
  count, and the same `managed_vm` verdict. Where a class's verdict differs, the test names it and the class's first
  divergence.
- **Not here:** fixing FCOS and FSIN; the oracle move.

## prerequisites

- 221 (closed)

## context_files

- `scripts/class_run.py` (whole); `scripts/semantics_differential.py` (`judge`, `undefined_memory`, `CASE_SETS`);
  `scripts/vm_oracle.py` (`run_cases`)
- `crates/w2m_vm/src/bin/case_runs.rs`, `crates/w2m_vm/src/case_inputs.rs`, `crates/w2m_vm/src/cases.rs`,
  `crates/cpu_cases/src/lib.rs`
- `store/runs/class-runs.json`; `store/probe-runs/<run>/` for the 21 reference runs

## signal contract

### Emits

- Per class: SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED (`managed_vm`), then CANDIDATE_ACCEPTED, or
  REDUCTION_COMPLETED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED; SESSION_COMPLETED. All are in vocabulary 0.12.

### Invariants

- The 512 verdicts `case_runs` gives today are unchanged.
- No class judged on a run outside its reference sets.

## artifact contract

### Files modified

- `crates/w2m_vm` (the class judge and its test), `scripts/gates.sh`, `store/case_runner/`,
  `docs/step-4-census-2026-10-28.md` (row 3)

### Command exit codes

- `cargo nextest run -p w2m_vm` and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** classes judged, of 458; conforming, against 456; the discrepancies by name, against FCOS and
  FSIN; case counts equal per class.
- **Planted faults:**
  1. One reference result's CF flipped: the class it covers turns discrepancy, and the test names it.
  2. A class judged on every run of its profile rather than its own sets: the case count differs, and the test names
     the class.
  3. A class dropped from the Rust index: the test names it.

## done criteria

Each of the 458 classes the frozen index judged is judged on Rust, over the cases of its own reference sets. A gate
holds the Rust index to the frozen one class by class.

## self-review checklist

- [ ] Scope names one verifiable property: the 458 classes' verdicts on Rust, equal to the frozen index.
- [ ] context_files cite the frozen judge, the Rust runner and the reference runs.
- [ ] Emits: the frozen sequence less the model's comparison, in vocabulary 0.12.
- [ ] Artifact contract gradable by command.
- [ ] Observation contract: values and three plants.
- [ ] One concept: the class judge.

## result
