# Sprint 165 — KIT_DIARY for Sprints 150-164, and step 3's synthesis

```yaml
---
id: 165
status: done
phase: 6
pass_kind: functional
---
```

## scope

KIT_DIARY's entry for Sprints 150-164 (reviewer-2's Medium 3 of 2026-10-27 and Medium 1 of 2026-10-28; Decision
2026-10-28), with step 3's synthesis. KIT_DIARY has had no entry since Sprint 149 (6b01ec3d4, 2026-10-25); step 3 of
`docs/rust-port-plan-2026-10-24.md` met its gate at Sprint 158 and its measurements at Sprint 164.

- **Step 3's synthesis:** did the step meet its text, and when: the skeleton's gate (Sprint 158, `sleeper` to
  ExitProcess under substrate, 40 events equal to the Python's), link 5's end (157) against the groups that ran on to
  161, and the three measurements (163, 164) against the design's budgets and revisit conditions, each with its record.
- **Findings, numbered from #69,** each from a tail entry or a review in BLACKBOARD, with the sprints it names:
  - Emits that under-declared a whole plane, a third time (126, 131, 162);
  - zsh expansion that ran or counted nothing (149: a command in a variable, not word-split, exited 127; 159: a glob
    in an assignment, not expanded);
  - plants that could not fail or did not run (149, 155, 156, 162, 163);
  - an instrument sized by guess (164: the 48-entry ring lost 2,914 of 3,002 takings; the histograms that followed
    mixed four sites);
  - an order changed in a Surfaced disposition, not a Decision (2026-10-28).
- **Hypothesis H5:** a lesson reaches new code only when it is a check in code; a lesson held in prose (BLACKBOARD,
  KIT_DIARY, a card's text) recurs. Evidence for and against, with its verdict: held in prose and recurring, the
  Emits rule (126, 131, 162), multi-line edits outside the Edit tool (128, 131, 142, 143, 153, 157, 158), a judge
  that skips (#65 at 149, again at 154); made a check and not recurring since, Signal Reports (stopped at 122; a gate
  fails a closed sprint without one from 127) and the planted-sample checks (the gates read the ProducerFailed event
  from 162).
- **The kit change** each finding suggests, named for the next kit version (`sdd-kit-3` is read-only; changes land as a
  new folder).

## prerequisites

- 164 (closed)

## context_files

- `KIT_DIARY.md` (its last entry, 2026-10-25; the hypothesis table)
- `BLACKBOARD.md` `## Sprint tail` (155-164), `## Surfaced for review` (reviewer-2's reviews of 2026-10-27 and
  2026-10-28; the code review of 2026-10-28), `docs/sprint-tail-archive-2026-10-08.md` (126, 131, 149-154)
- `docs/rust-port-plan-2026-10-24.md` step 3; `docs/design-rust-host-2026-10-24.md` 131-145
- `sdd-kit-3/templates/KIT_DIARY.md`

## signal contract

### Emits

None: no producer runs.

### Consumes

The records step 3's synthesis cites: `store/runs/w2m-vm-sleeper-1793149355000000000`,
`store/runs/w2m-vm-measurements-1793169234000000000`, `store/runs/w2m-vm-measurements-1793171817000000000`.

### Invariants

- No code changes; KIT_DIARY's earlier entries unchanged.

## artifact contract

### Files modified

- `KIT_DIARY.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** every number in the entry is in a record, a tail entry or a review the entry names, and each
  sprint it cites says what the entry says it does (read back against BLACKBOARD and the archive before the close).
- **Planted fault:** none runs; the check is the read-back, and the close names each number checked.

## done criteria

KIT_DIARY holds the entry, the synthesis and H5 with a verdict.

## self-review checklist

- [x] Scope names one verifiable property: the entry and the synthesis, every number sourced.
- [x] context_files cite the sources.
- [x] Emits: none, said.
- [x] Artifact contract gradable by command.
- [x] Observation contract: a read-back of every number.
- [x] One concept: the diary's entry for step 3.
