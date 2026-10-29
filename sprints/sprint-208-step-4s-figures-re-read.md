# Sprint 208 — Step 4's figures re-read

```yaml
---
id: 208
status: active
phase: 6
pass_kind: bookkeeping
---
```

## scope

The Decision "Step 4's reference run" (reviewer-2's follow-up on Sprint 207's stale reference). Step 4's gate figures
(docs/rust-port-plan-2026-10-24.md 135-138: "168 lines, 0 divergences", "against Sprint 108's 2,095,145,119") and
census rows 4, 5, 5a and 10 (docs/step-4-census-2026-10-28.md) are read from Sprint 108's batch run of 2026-10-17,
which predates Sprints 119-122. They are read again from the frozen host's recorded run of 2026-10-21,
store/runs/frontier-643a6283510c-1792623423346201000, each figure citing the record and the field it came from.

- **The plan:** an amendment to step 4 dated 2026-10-28: the comparison's line count and divergences, and the
  instructions a second, from that record's COMPARISON_COMPLETED and MEASUREMENT_RECORDED.
- **The census:** row 4's model exits by class, row 5's exit kinds, row 5a's host requests by op, row 10's
  instructions a second, each from the record; where the record does not hold a count the 10-17 log held, the row
  says so and names the source that would.
- **The order:** the batch run's order ranked by row 5a's counts is re-checked against the new counts; a change is
  recorded as a Decision.

## prerequisites

- 207 (closed)

## context_files

- store/runs/frontier-643a6283510c-1792623423346201000 (its events); docs/rust-port-plan-2026-10-24.md 128-145;
  docs/step-4-census-2026-10-28.md rows 4, 5, 5a, 10; build/sprint-108-run.log 120, 170-177

## signal contract

### Emits

- None: a bookkeeping card; it reads a record.

### Invariants

- No code changes; the plan's text changes only by a dated amendment.

## artifact contract

### Files modified

- `docs/rust-port-plan-2026-10-24.md` (an amendment), `docs/step-4-census-2026-10-28.md`

### Command exit codes

- `target/release/design_check`, `target/release/signal_report --check` and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** each re-read figure equals the record's field, read by `substrate replay` or the record's
  JSONL; the plan's "168" is replaced or confirmed by the record's line count.
- **Planted faults:** (1) a census figure copied from the 10-17 log instead of the record: the comparison of the two
  readings names it; (2) the amendment without its date: `design_check` names the plan.

## done criteria

Step 4's gate figures and census rows 4, 5, 5a and 10 cite the recorded run of 2026-10-21.

## self-review checklist

- [x] Scope names one verifiable property: the figures equal the record's fields.
- [x] context_files cite the record to read.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: step 4's figures re-read.
