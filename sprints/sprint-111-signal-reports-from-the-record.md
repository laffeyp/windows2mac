# Sprint 111 — Signal Reports from the record

```yaml
---
id: 111
status: done
phase: 5
pass_kind: functional
---
```

## scope

H3 of the SDD hardening order (docs/plan.md; Decision 2026-10-17; `docs/review-2026-10-17-sdd-hardening.md` items 1,
2, 3 and 6 of its "Incorrect" list and items 5 and 6 of its order). One concept: a sprint's Signal Report is made from
its records, and its card's Emits is checked against what fired.

The review found Signal Reports stopped after Sprint 048 (logged as a bypass, WORKING_AGREEMENT), 14 of the last 15
cards' Emits "None new", and Rubber Duck Passes whose narration is prose. `scripts/signal_report.py <NNN>`:

- finds the sprint's records: those whose RunStarted baseline names (`w2m_commit`) a commit whose subject begins
  "Sprint NNN" (substrate's `read_first_envelope`, one line a record);
- writes the signal trace: for each kind of record (class, family, frontier, ...), the count and substrate's
  `narration_summary` totals (finalised, producers failed, application events by kind); for a sprint's frontier and
  other single runs, substrate's `narrate` of their application events in seq order, with each event's `t` from the
  run's start;
- reads the card's `### Emits` and names each tag of the vocabulary it mentions; each is reported fired (with its
  count) or not fired; a tag that fired and the card does not name is listed too; an Emits of "None" with records
  that fired tags is reported as a card that names nothing its code fired;
- writes `signal-reports/sprint-NNN-report.md` in the kit's template (sdd-kit-3/templates/SIGNAL_REPORT.md): its
  scope from the card, its work from the sprint's commits, its trace and its Emits check as above; the self-grade and
  the Rubber Duck observations stay the BLACKBOARD tail's, which the report cites.

WORKING_AGREEMENT's sprint cadence gains the rule the review asks for: a card's Emits names the tags its code path
fires, with the payload values expected; "None new" is not an Emits for code that runs; each close writes its report
with this script; the Rubber Duck Pass's narration is the report's trace, not prose.

## prerequisites

- 110.

## context_files

- sdd-kit-3/templates/SIGNAL_REPORT.md; signal-reports/sprint-048-report.md; substrate docs/api.md (narrate,
  narration_summary, read_first_envelope); src/w2m/record.py (the baseline's `w2m_commit`); WORKING_AGREEMENT.md
  (sprint cadence policy, hand-author log).

## signal contract

### Emits

- None: the sprint reads records. Its check is of other sprints' Emits.

### Invariants

- Vocabulary 0.9 unchanged.

## artifact contract

- `scripts/signal_report.py`; `tests/test_signal_report.py`; `signal-reports/sprint-108-report.md` to
  `sprint-111-report.md`; WORKING_AGREEMENT's rule; `scripts/gates.sh` exit 0; CI `success` on the closing commit.

## observation contract

- **Judge:** the reports for Sprints 108, 109 and 110 made by the script. Expected: 108's names MEASUREMENT_RECORDED
  fired 107 times on its batch run and 526 class and family records; 109's says its card names no tag and its
  commits made no record; 110's narrates its batch run with EXECUTION_STARTED at 0.207 s and EXECUTION_COMPLETED at
  33.965 s, and its scratch plant is outside `store/runs` and so not in the report.
- **Plant:** a card whose Emits names a tag no record of the sprint fired (the test's card names TRACE_REJECTED):
  the report lists it as not fired.

## done criteria

Each close from Sprint 111 on writes its Signal Report from the record; CI green.

## observed
- **Made:** `scripts/signal_report.py` (a sprint's records found by their baseline's `w2m_commit` through
  `read_first_envelope`, 32,755 scanned in about 14 s; the trace from substrate's `narration_summary` and `narrate`;
  the Emits check); `tests/test_signal_report.py` (4 tests, the plant among them); the rule in WORKING_AGREEMENT's
  sprint cadence; the resumption in its hand-author log.
- **Judge:** the reports for Sprints 108, 109 and 110, as expected. 108's: 527 records (458 class, 68 family, the
  batch run), MEASUREMENT_RECORDED fired 107 times, and the tags its card left unnamed listed (SESSION_INIT 527,
  COMPARISON_COMPLETED 1,190, CANDIDATE_ACCEPTED 524, DISCREPANCY_REPORTED 8 among them); its narration shows the
  old stamping, EXECUTION_STARTED at 41.012 s. 109's: its card names no tag and the sprint made no record. 110's:
  EXECUTION_STARTED at 0.207 s, EXECUTION_COMPLETED at 33.965 s; its scratch plant, in runs/, is not in it.
- **Plant:** a card naming TRACE_REJECTED with no record firing it: "NOT FIRED"; `tests/test_signal_report.py`.
