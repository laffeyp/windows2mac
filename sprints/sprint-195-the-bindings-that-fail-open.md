# Sprint 195 — The bindings that fail open

```yaml
---
id: 195
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 193", reviewer-2's Medium 4 and Lows 2 and 4 of its review of Sprints 190-192, and
Sprint 194's Observation 3. `signal_report --check` finds a sprint's work commit by its subject and the close commit by
"Sprint N closed:", so a commit under any other subject between the card and the close is checked by nothing; its
multi-vCPU rule matches the phrase "multi-vCPU tests passed" and passes a close that says nothing; its test of the gated
tree copies `gates.sh`'s steps rather than running them; `docs/`, which `design_check` reads, is a bookkeeping path; and
`vex_store_reach` lifts nine encodings picked by hand. Sprint 194 made the third multi-line edit outside the Edit tool
within three sprints (189, 191, 194), so the rule becomes a check.

- **Every commit walked:** from `FIRST_WALKED_SPRINT` (195) `--check` walks each commit between a sprint's card commit
  and the next card's; each is the card alone, a work commit whose tree matches a pass line, or a close commit touching
  only bookkeeping paths, and any other names the commit.
- **The multi-vCPU claim stated:** the tail entry's artifact line carries "multi-vCPU: k of 3 passed", and `--check`
  holds k to the rows of `store/gates/multi-vcpu.tsv` for that sprint whose time falls in the cited pass's run.
- **The tree step shared:** `scripts/gated-tree.sh` prints the gated root tree; `gates.sh` and `signal_report`'s test
  both call it.
- **design_check's documents:** the designs and plan `design_check` reads leave BOOKKEEPING; a close that changes one
  fails.
- **vex_store_reach from the table:** the test enumerates every VEX.256 store form in the decoder's table over
  `VEX_CLASSES` and holds each to native or refused by name.
- **The edit check:** `crates/edit_guard`, a PreToolUse hook on Bash in `.claude/settings.local.json`, refuses a
  command that writes a multi-line change to a file by heredoc redirection or by `sed`'s `a\`, `i\`, `c\` or an embedded
  newline.

## prerequisites

- 194 (closed)

## context_files

- `crates/signal_report/src/main.rs` 528-960, `crates/signal_report/tests/check.rs`, `scripts/gates.sh` 20-40,
  `crates/design_check/src/main.rs`, `crates/translator/tests/vex_store_reach.rs`, `crates/x86_decoder/build.rs`

## signal contract

### Emits

- None: no producer runs; the judges are `signal_report`'s tests, `vex_store_reach` and `edit_guard`'s tests.

### Invariants

- No Python file added or changed; `--check` passes over Sprints 108-194 as before.

## artifact contract

### Files modified

- `crates/signal_report/` (src/main.rs, tests/check.rs), `scripts/gated-tree.sh` (new), `scripts/gates.sh`,
  `crates/translator/tests/vex_store_reach.rs`, `crates/edit_guard/` (new), `Cargo.toml`

### Command exit codes

- `signal_report --check`, `design_check`, the tests named and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** `--check` walks Sprint 195's own commits and passes; the claim "multi-vCPU: 3 of 3 passed"
  matches three rows; every VEX.256 store form the table holds is listed with its route.
- **Planted faults:** (1) a commit "tidy" between card and close changing `crates/`: `--check` names it; (2) a close
  claiming 3 of 3 with two rows: `--check` names the count; (3) `sed` with `a\` on a tracked file: `edit_guard` refuses
  it by name.

## done criteria

No commit between a card and its close goes unchecked, the multi-vCPU claim is a count held to its rows, and a
multi-line shell edit is refused before it runs.

## self-review checklist

- [x] Scope names one verifiable property: each binding fails on the defect it names.
- [x] context_files cite what to read first.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the bindings that fail open.

## result

- **Every commit walked:** `commits_walked` takes the commits from "Sprint N card:" (or "Sprint N's card:") to the next
  card's and names any that is neither "Sprint N: " nor "Sprint N closed:"; from `FIRST_WALKED_SPRINT` 195.
- **The multi-vCPU claim stated:** `multi_vcpu_count` requires "multi-vCPU: k of 3 passed" and holds k to the stage's
  passed rows for the sprint timed from the pass line's end less its seconds to its end.
- **The tree step shared:** `scripts/gated-tree.sh` prints the gated tree; `gates.sh` and `check.rs`'s
  `gates_root_tree` run it.
- **design_check's documents:** `DESIGN_DOCUMENTS` (design-, solution-, recentering-, architecture-, *plan-) stay in the
  gated tree from 195, and `is_bookkeeping` fails a close that changes one; ten documents match today.
- **vex_store_reach from every encoding:** the test lifts 12,288 three-byte VEX.256 encodings with a memory operand; the
  stores lifted are MOVAPD, MOVAPS, MOVDQA, MOVDQU and MOVUPS, all native, Sprint 192's five.
- **The edit check:** `crates/edit_guard` refuses a heredoc written or `tee`d to a path outside the temporary
  directories and `sed -i` with `a\`, `i\`, `c\`, an address range or a newline; a heredoc's body is not read as shell.
  The hook's settings were refused by the session's permission classifier; the program waits on the user's settings.
- **Plant 1** (the walk's result dropped): the walk test failed, its expected line "commit … \"tidy\" stands between its
  card and the next card, and is neither its work nor its close" missing. Restored byte for byte.
- **Plant 2** (the count without the run's start): the walk test failed, its expected line "store/gates/multi-vcpu.tsv
  holds 2 passed in its gates run" missing. Restored byte for byte.
- **Plant 3** (`a\` dropped from the check) reached no judge: the one `a\` case also held a newline. Its replacement 3b,
  the same plant against a new one-line `$a\ last line` case, panicked "called `Option::unwrap()` on a `None` value" at
  `lib.rs:280`. Restored byte for byte, rebuilt.
- **Departures:** the hook not installed (above); the store test enumerates encodings through the lifter rather than
  the decoder's rows, which covers every row the lifter takes.
