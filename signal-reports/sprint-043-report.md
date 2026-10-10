# SIGNAL_REPORT — Sprint 043

## 1. Observed

### scope_confirmation

> Every differential now writes a record in which each step causes the next: a unit's candidate causes its comparison, the comparison its acceptance or report, a report its route, and an acceptance resolves what an earlier run left open. Nine records hold that chain; the discrepancy ledger is derived from them.

### work_performed

- `src/w2m/differential.py`: the topology, `rebuild_ledger`, `by_unit`; example bytes stored under a report's `minimized_program_sha256`.
- Moved onto it: `check_guest_state.py`, `semantics_differential.py --judge`, `vm_oracle.py` (refuses `--record` with `--limit`), `generated_differential.py`, `game_code_differential.py`; `decoder_differential.py` records for the first time.
- `tests/test_differential.py`: resolution across runs, causal chains and multiplicities on every stored differential record, the ledger against the records.

### signal_trace

```
store/runs/guest-state-1791619112894954000: 7 CANDIDATE_GENERATED, 7 COMPARISON_COMPLETED, 7 CANDIDATE_ACCEPTED
store/runs/semantics-039-1791619244140788000, vm-oracle-039-1791619257172125000: 107 units each, all accepted
store/runs/semantics-041-*, vm-oracle-041-*: 32 units; semantics-042-*, vm-oracle-042-*: 2 units; all accepted
store/runs/decoder-generated-1791619278582619000: 5 units, 3 accepted, 2 rejected/reported/routed (contract_revision)
store/runs/decoder-game-hail-to-the-rainbow-1791619500387707000: 4 units, ours_gap reported/routed
store/runs/decoder-corpus-hail-to-the-rainbow-native-1791619443034053000: 7 files, all accepted
tests (tmp): planted CONTEXT.Rip -> REPORTED, ROUTED; next run -> CANDIDATE_GENERATED(discrepancy_id), RESOLVED(contract_revised)
```

## 2. Expected

> The card expected causal chains on every differential record, resolution across runs, verdicts unchanged, and replay green. All held.

## 3. Delta

### dual_contract_self_grade

**Signal contract:** pass (vocabulary 0.7, no new tags; multiplicities tested). **Artifact contract:** gates 0 (312 tests; 451 records replay) and CI on the closing commit (WORKING_AGREEMENT, from this sprint). **Observation contract:** causal chain pass; resolution across runs pass; verdicts unchanged pass; replay pass.

**overall:** pass.

## 4. Hypothesis

### rubber_duck_observations

**Sequence narration:** each differential computes its comparison per unit; the topology turns each unit's candidate into a comparison event, each comparison into a decision, each report into a route, and each acceptance of a unit with an open ledger entry into a resolution; the ledger is a fold over the records.

- *Observation 1 (the ledger is a function of the records):* nothing writes the ledger but the derivation. Disposition: tested.
- *Observation 2 (units have no field):* the unit travels in the first-divergence prefix. Disposition: recorded; a vocabulary unit field when one is needed.
- *Observation 3 (CI was red under this sprint):* the review found six red pushes. Disposition: resolved before the close; KIT_DIARY #28; a sprint now closes on its commit's green CI.

### status_and_blockers

`status: complete`
