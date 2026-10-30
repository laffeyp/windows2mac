# Sprint 255 — A design change in a close

```yaml
---
id: 255
status: done
phase: 6
pass_kind: functional
---
```

## scope

CI failed on Sprint 254's close (run 41134940668): "sprint 254: its close commit 8d18ece8a changes
docs/design-translation-cache-2026-10-30.md, outside the bookkeeping paths". The close amended the design's choice D
to the key as built. `signal_report`'s `is_bookkeeping` (`main.rs` 967-979) excludes the plans and designs under
`docs/` from a close from Sprint 195 on, so that a design changes only in a commit the gates test; the close read the
`BOOKKEEPING` list (1214-1222), which names `docs`, and not the function that applies it. The commit is pushed, and
pushed history is not rewritten, so every CI run fails until the record names it.

- **Where it stands, by lookup (2026-10-30):** `BETWEEN_SPRINTS` (Sprint 247) is the form for a pushed commit the
  record's rules refuse: the commit by its hash, with why, in a constant the check reads, a test holding it. Searched
  for `signal_report`, `close_moved`, `is_bookkeeping` in Deferred, Surfaced and `docs/surfaced-archive-2026-10-30.md`:
  none bears.
- **The readers:** `close_moved`, the gates' and CI's record check.
- **The work:**
  - `CLOSE_DESIGN_CHANGES`, beside `BETWEEN_SPRINTS`: 8d18ece8a and the design file it changed, with why; `close_moved`
    passes that one path in that one commit, nothing else; a test holds that the exception names one commit and one
    path, and that another design path in that commit is still refused.
  - WORKING_AGREEMENT: `signal_report --check` runs after the close commit and before the push, so a rule that reads
    the close commit itself judges it before CI does.
- **Not here:** the probe (card 256).

## prerequisites

- 254 (closed)

## context_files

- `crates/signal_report/src/main.rs` 909-979, 1214-1222, `BETWEEN_SPRINTS`; CI run 41134940668

## signal contract

### Emits

- None; a rule of the record.

### Invariants

- Every other close still held to the bookkeeping paths.

## artifact contract

### Files modified

- `crates/signal_report/src/main.rs`, `WORKING_AGREEMENT.md`, `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0; `signal_report 254 --check` returns 0

## observation contract

- **Expected values:** `signal_report --check` passing on the tree that holds 8d18ece8a.
- **Planted faults:**
  - (1) The exception keyed by the wrong hash: `--check` names 8d18ece8a again.
  - (2) The exception passing every design path of the commit: the test names a second path let through.

## done criteria

8d18ece8a stands by name with its reason, and no other close may change a design.

## self-review checklist

- [x] Scope names one verifiable property: one pushed commit named, the rule otherwise whole.
- [x] context_files cite the check, its list and the run.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: a design change in a close.

## result

- **The fix:** `CLOSE_DESIGN_CHANGES` and `named_design_change` (dc878ce88): 8d18ece8a's one path passes, with why;
  `a_named_design_change_passes_that_path_in_that_commit_alone` holds a second design path in that commit, and that path
  in another commit, refused. `signal_report 254 --check` passes on the tree.
- **The rule:** WORKING_AGREEMENT, the record check runs on the close commit before the push, and a rule is read in the
  function that applies it.
- **A finding beside the card:** the research read for the probe (card 4 of the translation cache's design) found the
  design cannot be built as it stands; the Decision "The translation cache's shape" records it, and card 256 revises the
  design first.
- **The gates:** `scripts/gates.sh` exit 0 in 236 s on dc878ce88, copied from `store/gates/passes.tsv`; the timing stage 695 ns an exit at load 8.13, its cause not read.
- **Plants,** run on dc878ce88, each restored by `cp` and checked by `cmp`.
- **Plant 1,** `signal_report 254 --check` named "sprint 254: its close commit 8d18ece8a changes
  docs/design-translation-cache-2026-10-30.md, outside the bookkeeping paths": the exception keyed by `8d18ece8b`.
- **Plant 2,** `a_named_design_change_passes_that_path_in_that_commit_alone` failed, "assertion failed:
  !named_design_change(commit, "docs/design-rust-host-2026-10-24.md")": every path of the commit passed.
