# Sprint 043 — causality in every record

```yaml
---
id: 043
status: closed
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

## observed

- **One topology** (`src/w2m/differential.py`): candidate per unit → comparison → acceptance or rejection with report → route; an acceptance resolves what the ledger holds open on its unit, as `contract_revised` or `implementation_repaired` by the discrepancy's route. The ledger (`store/runs/discrepancy-ledger.json`) is derived from the records in 0.5 s over 451 records; a test compares it with the file.
- **Moved onto it, verdicts unchanged:** guest state (7 units: runs, CONTEXT, XMM_SAVE_AREA32, EXCEPTION_RECORD, TEB, PEB, defaults; all accepted); semantics and VM oracle on sets 039, 041, 042 (107, 32, 2 classes, each candidate its rule's sha256; all accepted); generated decoder differential (5 units; iclass_differs and ours_accepts open, as the last one-producer record had them); game-code decoder differential (4 units over 6,192,478 positions; ours_gap open, 2 positions, as before); decoder differential over the game's 7 native files, its first record (5,801,517 positions, all accepted).
- **Resolution across runs:** a planted CONTEXT.Rip offset (reach: the CONTEXT unit, one field; the card said "layout unit" before the units were named) rejected CONTEXT and routed contract_revision; the next run, a repair attempt on CONTEXT (`discrepancy_id` on its candidate), accepted it and resolved that discrepancy by id; a third run resolved nothing.
- **Causal chain:** on all 9 differential records in the store, every decision traces to comparator → implementation → harness, every route to its report, every resolution to an acceptance; multiplicities as the signal contract states (tested).
- **Measurements:** `probe_capabilities.py`, `ordering_cost.py`, `page_geometry.py`, `litmus_mp.py` stay single-step runs, by design.
- **Known limit:** a ledger entry's unit is read from the report's first-divergence prefix; vocabulary 0.7 has no unit field. Recorded, not changed.
- **During the sprint:** the deep-dive review (2026-10-10) found CI red for six pushes; fixed with provenance in every record before this close (BLACKBOARD 2026-10-10).
- Gates: `scripts/gates.sh` 0 (312 tests; 451 records replay); CI: see the closing commit's run.
