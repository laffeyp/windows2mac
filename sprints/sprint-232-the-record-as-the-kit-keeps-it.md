# Sprint 232 — The record as the kit keeps it

```yaml
---
id: 232
status: done
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

- **Built:** 39 paragraphs, Sprints 183-221, each from its archived scope line and observation pass: what changed and
  the numbers it moved. Sprint 222, rolled at this close, has its paragraph too. The tail archive's header ("each
  sprint's summary also stands in `## Built`") is true again from 183.
- **The check:** `signal_report --check` reads `docs/sprint-tail-archive-2026-10-08.md` and fails a sprint from 183
  (`FIRST_BUILT_SPRINT`) whose entry is there and for which no Built line holds "— Sprint N (". Its unit test,
  `a_rolled_sprint_has_its_built_paragraph`, holds that a Deferred line naming the sprint and "Sprint 1840" do not
  count. The integration tests' scratch trees now carry an archive with no entry.
- **Surfaced:** `blackboard_roll --surfaced 3 docs/surfaced-archive-2026-10-30.md` (new, `crates/signal_report/src/
  bin/blackboard_roll.rs`) moved 29 reviews, 2026-10-27 to 2026-10-29, verbatim, newest first, and left one line.
  - A review is an entry whose text opens "reviewer-" or "The user's pasted code review".
  - The comprehension affirmations stay, which a new session looks for (`sdd-kit-3/AGENTS.md` 27). So do the
    user-only questions and answers, Sprint 178's partial close, the two defect sweeps and the three newest reviews:
    20 entries.
  - The 306 lines that left BLACKBOARD, sorted, equal the archive's entry lines, sorted.
  - Its unit tests: every character kept or moved, nothing moved when no review is older than the kept ones.
- **`ci_held`:** a cited CI run whose sprint has no close commit is named, "its CI run <id> is cited, and no commit
  \"Sprint N closed:\" is found to hold it to", where it passed; the unit test gains the case.
- **A second binary in the crate** broke `cargo run -p signal_report` ("could not determine which binary to run"),
  found by plant 1's first run; `default-run = "signal_report"` restores it. CI and the gates call
  `target/release/signal_report`, which was never affected.
- **WORKING_AGREEMENT** gains the tail roll: ten entries, the archive, the Built paragraph, the check, and the
  Surfaced roll's command.
- **The record's size:** BLACKBOARD from 552,934 bytes to 472,891 at the roll, with the 39 paragraphs added.
- **Plant 1** (Sprint 200's Built paragraph removed): "sprint 200: its entry has left the tail for docs/sprint-tail-archive-2026-10-08.md, and ## Built has no paragraph for it", exit 1.
- **Plant 2** (`is_close` reading "shut:" for "closed:"): "sprint 230: its CI run 41026362512 is cited, and no commit \"Sprint 230 closed:\" is found to hold it to", and the same for 229.
- **Plant 3** (line 5 of the archive copy, "Sprint 224" to "Sprint 225"): the sorted comparison named it, "303c303 < … review at Sprint 224 … > … review at Sprint 225". Its first form matched no text, changed nothing, and was run again by line number.
- Each plant was restored by `cp`, and `cmp` found the three files equal to their copies.
- Departures: `crates/signal_report/src/bin/blackboard_roll.rs`, `crates/signal_report/Cargo.toml`, `crates/signal_report/tests/check.rs`, `WORKING_AGREEMENT.md` and the tail archive (Sprint 222's entry), outside the card's file list.
