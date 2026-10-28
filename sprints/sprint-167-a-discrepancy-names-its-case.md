# Sprint 167 — A case runner's discrepancy names its case, after a reduction of none

```yaml
---
id: 167
status: done
phase: 6
pass_kind: functional
---
```

## scope

reviewer-2's Medium 4 of 2026-10-27 (Decision 2026-10-28's order): the Rust case runner (`crates/w2m_vm/src/bin/
case_runs.rs` 344-384) reports a class's discrepancy with no REDUCTION_COMPLETED before it, which rule S5 refuses, and
with `minimized_program_sha256` and `initial_state_sha256` both the empty hash, which rule E1 refuses: the faults the
Python's class runs had until Sprint 112 (BLACKBOARD Deferred 2026-10-17, Sprint 109). The model's port (step 4) will
report the first discrepancies the Rust records hold, so the form is set first.

As `scripts/class_run.py` 188-212 does since Sprint 112: before each DISCREPANCY_REPORTED, a REDUCTION_COMPLETED for
the class's first failing case, one instruction, so `steps` 0, `minimized` equal to `source`, `preserved_mismatch`
true (the vocabulary's note: "If nothing could be removed, minimized equals source and steps=0"); the report names that
case's program (sha256 of its instruction bytes) and initial state (sha256 of the canonical JSON of every other field
it sets, as `w2m.record.state_sha256`: sorted keys, no spaces, bytes as hex). The Python's `Case`
(`scripts/semantics_differential.py` 103-116, less `code` and `label`) gives the keys `data`, `edge`, `kind`, `mxcsr`,
`regs`, `rflags`, `stack`, `x87` (null, or `cw`, `st`, `sw`, `tags`), `xmm` and `ymm_hi`; `cpu_cases::Case` holds the
same fields. The 128-bit `xmm` and `ymm_hi` lanes and the 80-bit `st` values are JSON integers of any size in the
Python, so the Rust writes them as decimal digits itself rather than through a 64-bit number type.

Where the first failing case comes from: `cases::Tally::note` (`crates/w2m_vm/src/cases.rs` 125-131) keeps a
difference's count and its first example's text, and no case; it keeps the first case's index too, and `case_runs`
reads that case from the run's set (`case_runs.rs` 225) for the hashes. The record to judge comes from `case_runs`'s
own plant mode (`plant`, 227-231: every result's rflags bit 0 flipped in the first run), which reports discrepancies
on a record outside `store/runs/`.

## prerequisites

- 166 (closed)

## context_files

- `crates/w2m_vm/src/bin/case_runs.rs`, `crates/w2m_vm/src/cases.rs` (the case's fields)
- `scripts/class_run.py` 118-125, 188-212; `src/w2m/record.py` 69-90 (`state_sha256`)
- `signals/0.11.json` REDUCTION_COMPLETED, DISCREPANCY_REPORTED, rules S5 and E1; `scripts/check_records.py`

## signal contract

### Emits

- On the four `case-runner-rust` records: SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED,
  SESSION_COMPLETED, as before (no discrepancy expected).
- On a planted run's record (a lowering made wrong on purpose): CANDIDATE_REJECTED, REDUCTION_COMPLETED,
  DISCREPANCY_REPORTED, DISCREPANCY_ROUTED, SESSION_COMPLETED (completed_with_discrepancies).

### Consumes

The case runner's fixture and the four profiles' Windows results.

### Invariants

- No Python file added or changed; the hashes equal the Python's for the same case (a test computes both for one case
  through the frozen Python, run, not written).

## artifact contract

### Files modified

- `crates/w2m_vm/src/bin/case_runs.rs`, `WORKING_AGREEMENT.md`; four records under `store/runs/`

### Command exit codes

- the four `case-runner-rust` runs and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the planted run's record passes `check_records` with no S5 or E1 violation; its program hash
  equals sha256 of the first failing case's bytes, and its state hash equals the frozen Python's `state_sha256` of the
  same case.
- **Planted fault:** the REDUCTION_COMPLETED emitted after the report: `check_records` names S5.

## done criteria

A Rust case runner's discrepancy is reported as the Python's class runs report it since Sprint 112.

## self-review checklist

- [x] Scope names one verifiable property: S5 and E1 hold on a Rust discrepancy.
- [x] context_files cite the Rust and the Python it follows.
- [x] Emits: both records' tags, by name.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the hashes against the Python's, and a plant check_records names.
- [x] One concept: the discrepancy's form.

## result

- **The hashes:** `cpu_cases::case_hashes` writes the Python's canonical JSON itself. On each committed set's first
  case (13 sets, all format 2) it gives the frozen Python's two hashes (`crates/cpu_cases/tests/case_hashes.rs`, the
  values computed 2026-10-28 by running `semantics_differential.read_cases` and `class_run.case_hashes`). On the built,
  uncommitted sets it was checked once: `semantics_049`'s first case (format 3, YMM upper halves set) and
  `semantics_065`'s (format 4, an x87 register of 1199942704056667479188510) both equal the Python's; that check is
  not kept, since a test that skips where its file is absent is the fault KIT_DIARY #65 names.
- **The record's form:** `cases::Tally` keeps each difference's first case (`first_case`), `case_runs` hashes it, and
  emits REDUCTION_COMPLETED (steps 0, minimized equal to source, `preserved_mismatch` true) before each
  DISCREPANCY_REPORTED, which names the hashes; the empty-hash constant is gone.
- **The planted run** (`case_runs --plant-flip-cf`, a topology in the scratchpad, the record outside `store/runs/`):
  205 classes, 32 rejected, each with REDUCTION_COMPLETED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED;
  `check_records` applied S5 and E1 with 0 violations.
- **The plant** (REDUCTION_COMPLETED moved after the report): `check_records` named S5, 32 violations; the file was
  restored from its saved copy.
- **The case named:** the planted record's first report (ADD_LOCK, "run 38029980267: case 0") names program
  `d266341d…` and state `caf08742…`; the frozen Python's `case_hashes` of case 0 of that run's set
  (`probes/semantics_041/cases.bin`, found by the run's `cases.sha256`) gives the same two.
