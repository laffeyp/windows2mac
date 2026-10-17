# Sprint 112 — discrepancy reports that name their program and state

```yaml
---
id: 112
status: active
phase: 5
pass_kind: functional
---
```

## scope

In H4's place in the SDD hardening order (Decision 2026-10-17; Deferred 2026-10-17, Sprint 109). One concept: a
DISCREPANCY_REPORTED says which program failed from which state, as v0.2 11.3 asks ("everything the generation system
needs to reproduce the failure"). Today the class run reports `minimized_program_sha256` and `initial_state_sha256` as
`sha256:` and 64 zeros, the family run the state so, and file_run.py both (789 such fields on the record); and no
reduction precedes a class or family report (S5, 676 times).

- **Class runs:** each difference class's first failing case, from the model's judge and the VM's (`vm_oracle`
  examples, "case {i}: ...", read back by `vm_oracle.case_of`; a compile refusal's first refused case);
  `minimized_program_sha256` the case's instruction bytes, `initial_state_sha256` the rest of the case (registers,
  flags, memory, MXCSR, vectors, x87) through `w2m.record.state_sha256`, canonical JSON; a REDUCTION_COMPLETED with
  steps 0 and the program unchanged before each report: a case is one instruction, and the tag's note says "If
  nothing could be removed, minimized equals source and steps=0".
- **Family and file runs:** the program is the probe binary (its file name is its sha256); the state is what the
  VM run starts from (`family_run.vm_inputs`, which `run_in_vm` now reads, so the hash and the run cannot part; for
  file_run.py, the VM's defaults on a fresh root). No reducer cuts a compiled probe, so no REDUCTION_COMPLETED is
  claimed; S5 stays a known defect for family runs, named as such in `check_records.py`.

## prerequisites

- 111.

## context_files

- scripts/class_run.py (`judge_model`, `decide`), scripts/vm_oracle.py (`run_cases`, `note`), scripts/family_run.py
  (`run_in_vm`, `decide`), scripts/file_run.py; src/w2m/record.py; signals/0.9.json (REDUCTION_COMPLETED,
  DISCREPANCY_REPORTED, S4, S5, E1); scripts/check_records.py (`KNOWN`).

## signal contract

### Emits

- From a class run with a discrepancy (FCOS, FSIN at each sweep): per difference class and judge,
  REDUCTION_COMPLETED (`steps` 0, `preserved_mismatch` true, source equal to minimized, a case's instruction-byte
  sha256) then DISCREPANCY_REPORTED with that program and a state hash other than the zero placeholder, then
  DISCREPANCY_ROUTED; SESSION_INIT, CANDIDATE_GENERATED, two COMPARISON_COMPLETED and SESSION_COMPLETED as before.
- From a family or file run with a discrepancy: DISCREPANCY_REPORTED with the probe binary's sha256 and the run's
  start's state hash.

### Invariants

- Vocabulary 0.9 unchanged; S4 (a reduction only after a discrepancy comparison) holds.

## artifact contract

- The four scripts and `src/w2m/record.py`; `tests/test_discrepancy_reports.py`; `check_records.py`'s known list down
  to the family runs' S5; `scripts/gates.sh` exit 0; CI `success` on the closing commit.

## observation contract

- **Judge:** the sweep's FCOS and FSIN records pass `check_records.py` with 0 violations, S5 and E1 applied; their
  reports' hashes differ from the placeholder; the sweep's 456 of 458 classes and 68 families as before.
- **Plant:** the reduction left out (a scratch class run of FCOS with `REDUCTION_COMPLETED` not emitted): S5 names
  it; `tests/test_check_records.py` holds the same plant on a synthetic record.

## done criteria

Class, family and file runs report the program and state they failed from; the class runs' reports follow a
reduction; CI green.

## observed
