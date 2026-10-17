# Sprint 109 — every record checked against the vocabulary's rules

```yaml
---
id: 109
status: done
phase: 5
pass_kind: functional
---
```

## scope

H2 of the SDD hardening order (docs/plan.md; Decision 2026-10-17; `docs/review-2026-10-17-sdd-hardening.md` item 4).
One concept: the vocabulary's ordering rules are checked over every record. `src/w2m/trace.py` enforces T5, T6 and
S14 on guest traces at ingest; `scripts/check_replay.py` checks each record's stream integrity (substrate level 2)
and nothing about its events; no code checks T1 to T4, T7 or S1 to S13 against the 32,755 records in `store/runs/`.

`scripts/check_records.py` reads each record through substrate's `read_record` and checks, from the declarations in
`signals/current.json`:

- T1 (SESSION_INIT first), T2 (SESSION_COMPLETED last), T3 (each EXECUTION_STARTED closed by one
  EXECUTION_COMPLETED or EXECUTION_FAILED within timeout_seconds + kill_grace_seconds on the record's clock; counted
  as unchecked where EXECUTION_STARTED omits timeout_seconds, as T3 says), T4 (each EXECUTION_COMPLETED followed by
  one TRACE_INGESTED or TRACE_REJECTED with its trace_sha256), T7 (CONTRACT_REVISED followed by
  DEPENDENTS_INVALIDATED);
- S1 to S13 where their tags occur: the `pairing_ordering`, `forbidden_after`, `forced_next` and `allowed_set`
  kinds over the payload fields each names; a rule whose match is prose (S3, S7, S10, S13) checked for order alone,
  and said so;
- E1 (every sha256-typed field `^sha256:[0-9a-f]{64}$`) and E6 (first_divergence present on a discrepancy).

It prints, per rule, the records and events checked, the violations, and the unchecked; with `--since` it fails
(exit 1) on a violation in a record made at or after a date, so the gate holds new records to the rules while the
old ones are reported, not rewritten (records are never edited).

## prerequisites

- 108.

## context_files

- signals/0.9.json (`temporal_invariants`, `state_transitions`, `evidence_constraints`); scripts/check_replay.py;
  scripts/gates.sh; substrate `read_record` (substrate docs/api.md); src/w2m/trace.py.

## signal contract

### Emits

- None: the sprint reads records and emits nothing. Its signals are the events already on the 32,755 records; the
  checker's report names each rule's count.

### Invariants

- Vocabulary 0.9 unchanged.

## artifact contract

- `scripts/check_records.py`; `tests/test_check_records.py` (a record per rule kind with its violation planted);
  `scripts/gates.sh` runs it with `--since` the closing date; `scripts/gates.sh` exit 0; CI `success` on the closing
  commit.

## observation contract

- **Judge:** the report over every record in `store/runs/`: per rule, checked, violations and unchecked, each count
  stated in the observed section; every violation in a record made from 2026-10-17 on is fixed or recorded with its
  cause.
- **Expected finding:** the frontier's records omit `timeout_seconds` on EXECUTION_STARTED, so T3 is unchecked on
  every one of them (the review's "payload anomaly in every frontier record").
- **Plant:** each rule kind's test record with its violation (SESSION_INIT second; an EXECUTION_COMPLETED with no
  TRACE_INGESTED; a sha256 of 63 hex digits) must be reported.

## done criteria

Every record is checked against T1 to T4, T7, S1 to S13, E1 and E6; new records are held to them by the gate; CI
green.

## observed
- **Made:** `scripts/check_records.py` (T1 to T4, T7, S1 to S13 by kind from `signals/current.json`, E1 with the zero
  placeholder named, E6; `field=value` in a rule's match a filter on its first event, prose matches checked for order
  alone and counted unchecked); `tests/test_check_records.py` (a clean record; eight plants, each named); in
  `scripts/gates.sh` with `--since 2026-10-17`. It reads the 32,755 records in 27 s.
- **Judge** (every record in `store/runs/`, at the close): 1,962 violations, none held from 2026-10-17 on.

  | rule | applied | violations | unchecked | where the violations are |
  |---|---|---|---|---|
  | T1 | 32,753 | 2 | 0 | two model sessions (kimi-k2.7) begin with SessionStarted |
  | T2 | 32,753 | 12 | 0 | records that end before SESSION_COMPLETED: candidates, class-add_lock, six of three families (files_more, process_queries, processor_topology), two frontiers, two model sessions |
  | T3 | 566 | 43 | 452 | 43 frontier runs with no completion; 452 frontier runs whose EXECUTION_STARTED omits timeout_seconds |
  | T4 | 523 | 433 | 0 | 394 laptop probe runs, 33 laptop and 5 runner game runs, 1 frontier: no trace ingested after the execution |
  | T7 | 3 | 0 | 0 | |
  | S1, S4, S6, S8, S11 | 1 to 140 | 0 | 0 | |
  | S2, S3 | 32,254 | 0 | 32,254 | prose matches: order alone |
  | S5 | 140 | 676 | 0 | class and family runs report with no REDUCTION_COMPLETED (known: Deferred) |
  | S7, S10 | 4, 1 | 0 | 4, 1 | prose matches |
  | E1 | 36,525 | 789 | 0 | every one the zero placeholder in DISCREPANCY_REPORTED: 744 in class, 29 in family, 16 in file records (known for class and family: Deferred) |
  | E6 | 339 | 7 | 0 | 2 gamediff, 1 gendiff, 4 semantics records: a discrepancy with no first divergence |

- **Expected finding, met:** T3 cannot be checked on 452 frontier records, which omit timeout_seconds. A further
  finding: the frontier's runner emits EXECUTION_STARTED after the run, with the rest (its `t` in the latest record
  41 s after the runner started), so the record cannot place the execution in time either.
- **Held from now on:** a new record breaking any rule fails the gate, but for the two known class- and family-run
  defects, which the report prints with their counts (57 and 113 from 2026-10-17) and a Deferred entry.
- **Correction (2026-10-17, Sprint 110):** T3's 452 unchecked are 394 laptop probe runs, 46 frontier runs and 12
  judge records, not 452 frontier runs as the table and the Deferred entry say (87 frontier records exist); its 43
  violations are all frontier runs.
