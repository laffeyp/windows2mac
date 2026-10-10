# Sprint 043 — causality in every record

```yaml
---
id: 043
status: open
phase: 4
pass_kind: functional
---
```

## scope

Six differentials write their records by replaying finished results through one producer (review 2026-10-09 progress, S1): `check_guest_state.py`, `semantics_differential.py --judge`, `vm_oracle.py`, `generated_differential.py` and `game_code_differential.py`; `decoder_differential.py` writes no record at all. Each record shows that a list was written, not that a comparison caused a discrepancy or that a discrepancy caused a route. Only the candidate loop (Sprint 031) and the item runs (041, 042) have causal records, and only the candidate loop ever emits DISCREPANCY_RESOLVED.

1. **One differential topology** (`src/w2m/differential.py`). A differential names what it judges (the implementation and its sha256), the reference, and its units (an instruction class, a guest-state section, a PE file). One run per differential:
   - SESSION_INIT
   - → CANDIDATE_GENERATED per unit (the implementation as judged on that unit; `discrepancy_id` set when the ledger holds one open for the unit, making the run a repair attempt)
   - → COMPARISON_COMPLETED for that unit, caused by its candidate
   - → CANDIDATE_ACCEPTED, or CANDIDATE_REJECTED with DISCREPANCY_REPORTED, caused by the comparison
   - → DISCREPANCY_ROUTED, caused by the report
   - → DISCREPANCY_RESOLVED, caused by an acceptance, for each discrepancy the ledger holds open on that unit from an earlier run
   - → SESSION_COMPLETED once every unit is decided and every route and resolution is written
2. **The discrepancy ledger** (`store/runs/discrepancy-ledger.json`). It is rebuilt from the records: a REPORTED opens an entry and a RESOLVED closes it. A test rebuilds it from the records and compares.
3. **The six differentials move onto it**, each with its units named. The decoder differential gets a record for the first time.
4. **The four measurements stay single-step runs** (`probe_capabilities.py`, `ordering_cost.py`, `page_geometry.py`, `litmus_mp.py`). They measure the Mac; nothing in them causes anything else. `record.py` already says that is what a one-step run is for. The card names them so the choice is on the record, not an omission.

## prerequisites

- 041 and 042 (item runs, `Once()` decisions), 031 (the candidate loop's resolution), substrate `./substrate` branch `w2m`

## context_files

- `scripts/run_candidate_loop.py` (reducer, DISCREPANCY_RESOLVED), `scripts/class_run.py`, `scripts/file_run.py`, `src/w2m/record.py`, the six scripts, `docs/review-2026-10-09-progress.md` S1, substrate `api.trace_ancestry`, `api.explain_producer`, `api.replay`

## signal contract

### Emits

- Per differential run: the chain above. Multiplicities: one SESSION_INIT; per unit one CANDIDATE_GENERATED, one COMPARISON_COMPLETED, and one decision (ACCEPTED or REJECTED); one ROUTED per REPORTED; one RESOLVED per ledger entry open on an accepted unit; one SESSION_COMPLETED (KIT_DIARY #26: multiplicities stated and tested).

### Invariants

- Vocabulary 0.7; no new tags.

## artifact contract

- created: `src/w2m/differential.py`, `tests/test_differential.py`, `store/runs/discrepancy-ledger.json`, new records for the six differentials
- modified: the six scripts and their tests
- `scripts/gates.sh` → 0; every record replays

## observation contract

- **Causal chain:** on each new record, `trace_ancestry` from every decision reaches its comparison, its unit's candidate and SESSION_INIT; from every ROUTED it reaches its REPORTED and its comparison; from every RESOLVED it reaches the acceptance that caused it.
- **Resolution across runs:** a planted fault in a guest-state layout row (reach: the `layout` unit, one field) makes one run report and route a discrepancy. The next run, without the plant, accepts the unit, and its RESOLVED names the first run's discrepancy_id. The ledger shows it open after the first run and closed after the second.
- **Unchanged verdicts:** each differential's verdicts per unit equal what its one-producer version reported, on the same inputs.
- **Replay:** every record, old and new, replays at level 2.

## done criteria

Every differential's record holds the chain from candidate to comparison to decision to route, and a discrepancy opened in one run is closed on the record of the run that fixed it.
