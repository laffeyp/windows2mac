# Sprint 127 — Signal Reports in Rust

```yaml
---
id: 127
status: active
phase: 6
pass_kind: architecture
---
```

## scope

Step 2 of `docs/rust-port-plan-2026-10-21.md`, one of its judges. A Rust binary, `signal_report`, writes a sprint's
Signal Report from its records as `scripts/signal_report.py` did (Sprint 111): the card's scope, the sprint's
commits, the signal trace (each kind of record counted; each single record narrated by `substrate narrate`, with each
event's time from the run's start), and the card's Emits checked tag by tag against what fired. It reads the record
files itself (`events-*.jsonl`, one JSON frame a line) for the first frame's baseline and the count of each kind; the
frames' integrity stays `substrate replay`'s. A record belongs to a sprint when its baseline's `w2m_commit` is one of
the sprint's commits, or, for a record whose baseline names no commit (`substrate run --file`, Sprints 125 and 126),
when one of the sprint's commits added it.

With `--check`, it fails when a sprint from 108 on whose card says `done` has no `signal-reports/sprint-NNN-report.md`;
the gates and CI run it, so a fourth silent stop cannot happen (the reports stopped after 048, around 108, and after
122). With `--backfill`, the report says it was written after the sprint's close. Sprints 123 to 126 are written
with it.

## prerequisites

- 126 (closed)

## context_files

- `sdd-kit-3/templates/SIGNAL_REPORT.md`; `scripts/signal_report.py` (read, not run); `signal-reports/sprint-111-report.md`
  to `sprint-122-report.md` (the judge's data)
- substrate `projections/narrate.py` (`narrate`, `narration_summary`), `record/record.py` (`read_first_envelope`,
  segments)
- `WORKING_AGREEMENT.md` hand-author log 2026-10-24 (the stop)

## signal contract

### Emits

None. The binary reads records and writes Markdown; it puts nothing on a record.

### Consumes

Every tag on the records of the sprint reported, counted by kind.

### Invariants

- No Python file is added to w2m; substrate's `narrate` runs as the harness's own command.
- The reports for 111 to 122, written again by the Rust binary from the same records, equal the committed ones but for
  the line that names the program that made them.

## artifact contract

### Files created

- `crates/signal_report/` (the binary); `signal-reports/sprint-123-report.md` to `sprint-126-report.md`

### Content assertions

- Each of the four new reports says it was written after the sprint's close, on 2026-10-24.
- Sprint 126's report finds the two records its commits added and names SESSION_INIT and SESSION_COMPLETED fired.

### Command exit codes

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` return 0
- `target/release/signal_report --check` returns 0

## observation contract

- **Expected value:** the twelve committed reports, 111 to 122, written again from their records, equal byte for byte
  below their "Made by" line.
- **Planted fault 1:** a card set to `done` with no report: `--check` exits 1 and names the sprint.
- **Planted fault 2:** a card whose Emits names a tag no record fired: the report says `NOT FIRED` for it.

## done criteria

Every closed sprint from 108 on has a Signal Report, and a gate keeps it so.

## self-review checklist

- [x] Scope names one verifiable property: a sprint's report from its records, and none missing.
- [x] context_files name the template, the Python replaced, the reports that judge it, and substrate's readers.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: twelve reports as the expected value; two planted faults.
- [x] One concept: the Signal Report.

## result

- `crates/signal_report` (no dependency but `serde_json`): the report, `--write`, `--backfill DATE`, `--check`. It
  reads the first frame of each of the 38,580 records under `store/runs/` and the frames of the sprint's own; each
  single record's prose is `uv run substrate narrate`'s.
- Expected value: the reports for 111 to 122, written again from their records, equal the committed ones line for line
  below the "Made by" line but for one line each: the commit that added the report itself, which did not exist when
  the Python wrote it (checked for all twelve against `git log --diff-filter=A`). No other line differs: commits,
  counts, narrated times and Emits checks.
- Reports 123 to 126 written with `--backfill 2026-10-24`. 123 and 124 made no record. 125: one record, SESSION_INIT
  and SESSION_COMPLETED. 126: two records, both tags fired 3 times, and 50 tags fired that its card's Emits did not
  name (the samples run): its card under-declared what it emitted.
- Planted faults: (1) a card marked done with no report: `--check` exit 1 naming sprint 200 (test with a scratch
  root); (2) a tag named in Emits and not fired: `NOT FIRED` (unit test).
- Tests: 6 (`signal_report`: 4 unit, 2 of `--check`); the workspace 25.
