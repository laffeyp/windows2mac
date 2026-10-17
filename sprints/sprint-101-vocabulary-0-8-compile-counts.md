# Sprint 101 — vocabulary 0.8: the VM's compile counts on the record

```yaml
---
id: 101
status: active
phase: 5
pass_kind: functional
---
```

## scope

The third of the five entries deferred that should not have been (user decision 2026-10-16; Deferred 2026-10-16,
Sprint 084, carried by 085 and 086, large review M3). The batch run prints the VM compiler's counts (its compiles,
and its refusals and cut blocks by cause, `VMC_COUNT_NAMES`), and its record holds none of them: MEASUREMENT_RECORDED's
`metric` is an enum of 12 values with no block count. Decision 2026-10-15 reads the host-compile share from signals.

Proposal P19 (ENUM_EXTENDED, layer 2): `metric += vm_compile_blocks`. One value, not one per cause: the cause is the
`variant` MEASUREMENT_RECORDED already carries for "the configuration measured when one metric compares
alternatives", and the 23 causes change as the compiler does (Sprint 096 added `pause`), which an enum value per
cause would make a vocabulary bump each time. Unit `blocks`. An enum value bumps the minor version (PRINCIPLES.md's
proposal taxonomy), so 0.8.

## prerequisites

- 100.

## context_files

- signals/0.7.json, 0.7-rationale.md, proposals.json, current.json; scripts/gen_events.py; src/w2m/__init__.py;
  scripts/run_frontier.py; src/w2m/vm_backend.py (`vm_compile_counts`, `VMC_COUNT_NAMES`).

## signal contract

### Emits

- MEASUREMENT_RECORDED (metric `vm_compile_blocks`, unit `blocks`, phase `discovery`, backend `managed_vm`,
  artifact_state `cold` with no translation-cache hits, else `warm`, variant the cause) from the batch run's runner,
  one per count, after EXECUTION_COMPLETED's inputs are gathered.

### Invariants

- Every 0.7 tag and enum value unchanged; guest-plane traces of 0.1-0.7 ingest under 0.8.

## artifact contract

- signals/0.8.json, 0.8-rationale.md, P19 ratified, current.json 0.8, events.py regenerated (`--check` 0), the
  runner's emits; `scripts/gates.sh` exit 0; CI `success` on the closing commit.

## observation contract

- The batch run's record holds one MEASUREMENT_RECORDED per count, whose values equal the printed counts; the record
  reads back through substrate (`read_record`).

## done criteria

The VM's compile counts are on the batch run's record under a ratified vocabulary; CI green.

## observed

- **Made** (94eead46): signals/0.8.json and its rationale, P19 ratified, current.json 0.8, events.py regenerated; the
  runner emits the counts. The gates' mypy stopped the first gate run on `artifact_state` typed as `str`; it was
  made the Literal before the commit.
- **Judge:** the batch run's record (frontier-643a6283510c-1792220209039481000, read through substrate's
  `read_record`) holds 23 MEASUREMENT_RECORDED events, one a cause, `artifact_state` warm (the translation cache hit),
  each value equal to the printed count: compiled 274,939; refused for protected pages 17,092; not a code page 366;
  decode 304; guarded 280; no stencil 123; page needs a split 15; pause 2; host only 1; cut by decode 243, by no
  stencil 142, by guarded 210; ten causes 0.
