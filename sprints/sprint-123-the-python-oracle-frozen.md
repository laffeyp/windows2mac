# Sprint 123 — the Python oracle frozen

```yaml
---
id: 123
status: done (commit and CI pending)
phase: 6
pass_kind: architecture
---
```

## scope

Step 0 of `docs/rust-port-plan-2026-10-21.md`. Every later step of the Rust port is judged by giving the verdicts the Python
w2m gives today, so those verdicts are written down once, as data, before any Rust exists. `scripts/golden.py --write`
reads the records under `store/runs/` whose baseline names 4b36b5ad (code identical to f2d071af, Sprint 122's
close) and writes `specs/golden/python-final.json`: per class run its iclass, case count, contract, comparisons with
their relation and first divergence, and its outcome; per family run the same, keyed by family and vCPU count; for
the batch run its line count, thread count, exit status, first divergence and the sha256 of the guest trace's body
(lines 2 on; line 1 names the run, BLACKBOARD Decision 2026-10-22). `scripts/golden.py --check <commit>` reads the
records a commit made and reports each key whose verdict differs from the golden file, exit 1 on any. The local tag
`python-final` marks f2d071af.

## prerequisites

- 122 (closed; its records are the golden set)

## context_files

- `sdd-kit-3/AGENTS.md`; `BLACKBOARD.md` Decisions 2026-10-21 (user) and 2026-10-22; `WORKING_AGREEMENT.md`
- `docs/rust-port-plan-2026-10-21.md` (step 0)
- `scripts/signal_report.py` (records by baseline commit, `read_first_envelope`)
- `scripts/call_compare.py` (`guest_trace`: line 1 is GUEST_TRACE_OPENED with `execution_id`)
- the records: `store/runs/class-*`, `family-*`, `frontier-643a6283510c-1792623423346201000`

## signal contract

### Emits

None: the script writes no record and fires no tag; it reads the tags below from stored records. The vocabulary
(0.9) does not change.

### Consumes

- SESSION_COMPLETED (`outcome`, `accepted_count`, `discrepancy_count`), COMPARISON_COMPLETED (`comparison_id`,
  `relation`, `first_divergence`), CANDIDATE_GENERATED (`contract_id`, `contract_version`), EXECUTION_COMPLETED
  (`exit_status`, `guest_line_count`), TRACE_INGESTED (`thread_count`, `trace_sha256`), from the 530 records at 4b36b5ad.

### Invariants

- No file under `store/` is written. `src/w2m` is not touched.
- The golden file holds 458 class keys, 71 family keys and 1 batch-run key.

## artifact contract

### Files created

- `scripts/golden.py`
- `specs/golden/python-final.json`

### Content assertions

- `specs/golden/python-final.json` parses; `commit` is 4b36b5adc7d2a62a6f31c76caa30ac961c3f6d5e; 530 keys.
- The batch-run entry's `trace_body_sha256` is f927ac28fb3681ab38c0952517a11fb9a32bdc7e27bbcf2985673cf474da6fc1.
- Class keys with a discrepancy: 2 (FCOS, FSIN; Sprint 122's sweep, 456 of 458 accepted).

### Command exit codes

- `uv run python scripts/golden.py --write` returns 0
- `uv run python scripts/golden.py --check 4b36b5ad` returns 0 (the golden set against itself)
- `uv run python scripts/golden.py --check f1838b96` returns 0 or 1, its differing keys reported (older code; a
  measurement, not a gate)
- `uv run ruff check scripts/golden.py` and `uv run mypy scripts/golden.py` return 0

## observation contract

- **Planted fault:** `--check` against a copy of the golden file in which the ADC class entry's first comparison has
  `first_divergence` set to 1 must report exactly the key `class:ADC` and exit 1. Reach: one key of 530; the plant
  changes one field that the real ADC record holds as null.
- **Expected value:** the batch-run body hash at f1838b96 equals the one at 4b36b5ad (both f927ac28…, measured
  2026-10-22), so `--check f1838b96` reports no `frontier` key.

## done criteria

The golden file exists, checks clean against its own commit, catches the plant, and the tag `python-final` names
f2d071af.

## notes

- Records from the run name the commit checked out when they were made; 4b36b5ad's tree differs from f2d071af only in
  stored records, so the golden set is f2d071af's verdicts.
- The tag is local; it is pushed with the sprint's commit.
- Found while executing: a COMPARISON_COMPLETED's `comparison_id` embeds the candidate id, which hashes w2m's code
  (ADC: rule-adc-4ffabefab879 at 4b36b5ad, rule-adc-8fe1853bac85 at f1838b96). Held as is, the first `--check f1838b96`
  reported 529 of 530 keys differing on verdicts that were the same. The golden file names a comparison by what it
  compared against (`model`, `managed_vm`, a reference run id); the second check reported 0. A Rust w2m changes every
  candidate id, so this is the form the port is judged in.

## self-review checklist

- [x] Scope names one verifiable property: the golden file and its check.
- [x] context_files cover the record format and the trace header.
- [x] Emits: none, stated with the tags read.
- [x] Artifact contract gradable by command.
- [x] Observation contract: a planted fault with its reach, and an expected value.
- [x] Two files, one concept.
