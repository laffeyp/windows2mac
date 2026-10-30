# Sprint 232 — The record as the kit keeps it

```yaml
---
id: 232
status: active
phase: 6
pass_kind: functional
---
```

## scope

The bookkeeping card in the Decision "Sequence after Sprint 214" (reviewer-2's Medium 5 and reviewer-3's Medium 4 on
the record's size; reviewer-3's Low 3 of its review of 212-214, Deferred to this card).

- **Where it stands, by lookup (2026-10-30):**
  - BLACKBOARD is 552,934 bytes: Surfaced 115,800, Decisions 240,592, Built 102,987, Deferred 30,871, Sprint tail
    50,246.
  - The kit rolls a tail entry older than the last ten "into `## Built` as compressed paragraphs"
    (`sdd-kit-3/templates/BLACKBOARD.md` 58, `sdd-kit-3/AGENTS.md` 92).
  - This project moves each rolled entry verbatim to `docs/sprint-tail-archive-2026-10-08.md`, whose header says
    "each sprint's summary also stands in `## Built`". Built's last paragraph is Sprint 182's. The 39 sprints from
    183 to 221 rolled with no paragraph, and nothing checks it.
  - Surfaced holds every review since the project began. Reviewer-2's Medium 5 ruled that reviews older than the
    last three roll to a dated archive.
  - `signal_report`'s `ci_held` passes silently when it finds no close commit: `holds_close` answers None and nothing
    is held.
- **The work:**
  - Built gains one compressed paragraph for each of Sprints 183-221, read from its archived entry and card: what
    changed, the gate numbers it moved, its departures.
  - `signal_report --check` fails a sprint, from 183, whose entry has left the tail and has no Built paragraph.
  - Surfaced keeps its last three reviews. The older ones move verbatim to
    `docs/surfaced-archive-2026-10-30.md`, newest first, with a pointer left in their place.
  - `ci_held` names a sprint whose close commit it cannot find, from `FIRST_CI_HELD_SPRINT`.
- **Not here:** Decisions, which is the audit trail of rulings and stays whole; the census rows' histories.

## prerequisites

- 231 (closed)

## context_files

- `BLACKBOARD.md` (Surfaced, Built, Sprint tail), `docs/sprint-tail-archive-2026-10-08.md`
- `sdd-kit-3/templates/BLACKBOARD.md`, `sdd-kit-3/AGENTS.md` 92
- `crates/signal_report/src/main.rs` (`ci_held`, the tail checks)

## signal contract

### Emits

- None: the record and its checks.

### Invariants

- No entry is lost. Each archived review and each Built paragraph's sprint can still be found by its date and number.

## artifact contract

### Files modified

- `BLACKBOARD.md`, `docs/surfaced-archive-2026-10-30.md` (new), `crates/signal_report/src/main.rs`

### Command exit codes

- `scripts/gates.sh` returns 0; `signal_report --check` returns 0

## observation contract

- **Expected values:** 39 Built paragraphs, 183-221; Surfaced's three reviews kept, and the rest archived verbatim
  (a `diff` of the moved text against the old file's lines); BLACKBOARD's new size.
- **Planted faults:**
  1. One Built paragraph removed: `--check` names the sprint.
  2. A sprint's close commit renamed in a scratch history: `ci_held` names it.
  3. A Surfaced review's line changed in the archive copy: the `diff` names the line.

## done criteria

The record keeps what the kit says it keeps, and a check holds the rolled summaries and the CI tie to their sprints.

## self-review checklist

- [x] Scope names one verifiable property: every rolled sprint summarized in Built, and the CI tie held.
- [x] context_files cite the record, the kit's rule and the checks.
- [x] Emits: none, with the reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the record kept as the kit keeps it.

## result
