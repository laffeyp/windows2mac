# Sprint 267 — The guard over both standing rules

```yaml
---
id: 267
status: done
phase: 6
pass_kind: functional
---
```

## scope

reviewer-3's Medium 1 at 266: the user's rule "No Python runs" was broken by a Python start in Sprints 238, 240, 250
and 266, and the rule that multi-line edits use the Edit tool in 262, 263 and 266, each breach recorded and the rate
unchanged. `crates/edit_guard`, the PreToolUse hook program built at Sprint 195, refuses a heredoc written to a project
file and a multi-line `sed -i`, and refuses neither an in-place `perl` or `awk` edit nor a Python start. This card
gives it both, so the hook, once the user installs it, stops both rules' breaches before they run.

- **Where it stands, by lookup (2026-10-30):**
  - `edit_guard::refusal` (`src/lib.rs` 193-267) reads the command's words (`words`, heredoc bodies removed) and refuses
    a heredoc redirected or piped by `tee` to a path outside `FREE`, and `sed -i` with `a\`, `i\`, `c\`, a range, a
    newline or a `\n` escape. Sprint 266's `cat >> … <<'EOF'` is refused by it; its `perl -0pi -e …` and `python3 -`
    are not.
  - WORKING_AGREEMENT 290: the hook is built, not running; installing it is the user's step.
  - The Drift watchlist (`BLACKBOARD.md` 680-686) holds an entry for shell edits and none for Python starts.
- **The work:**
  - `refusal` names a simple command whose program is `python`, `python3`, `python3.<n>`, `pip`, `pip3`, or `uv run`
    or `uvx` followed by one of them, by the command's words, so a word inside a quoted message or a heredoc body is not
    a start; and `perl` or `awk` with an in-place option (`-i`, `-pi`, `-0pi`, `-i.bak`; `awk -i inplace`).
  - Tests for each refused form and for the allowed ones: `grep python`, a commit message naming Python, `uv run
    substrate`, which the record script runs, `perl -ne` reading a file.
  - The Drift watchlist: the edits entry annotated with 262, 263 and 266; an entry for the Python starts.
- **Not here:** installing the hook (the user's); step 5's close-out (card 268).

## prerequisites

- 266 (closed)

## context_files

- `crates/edit_guard/src/lib.rs`, `src/main.rs`; WORKING_AGREEMENT 290; `BLACKBOARD.md` Drift watchlist

## signal contract

### Emits

- None; a hook's refusals, held by tests.

### Invariants

- Every command the tests allowed before is allowed after.

## artifact contract

### Files modified

- `crates/edit_guard/src/lib.rs`, `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** Sprint 266's three breaching commands, as recorded, each refused with its reason; Sprint 266's
  allowed commands (`uv run substrate`, the one-line `sed`s) allowed.
- **Planted faults:**
  - (1) `python3` left out of the interpreters: the test names `python3 -`.
  - (2) the in-place check reading only `-i` exactly: the test names `perl -0pi`.

## done criteria

`edit_guard` refuses a Python start and an in-place `perl` or `awk` edit, by the command's words, and allows every
command it allowed before; the Drift watchlist counts both rules' breaches.

## self-review checklist

- [x] Scope names one verifiable property: the guard refuses both rules' breaches.
- [x] context_files cite the guard, the agreement and the watchlist.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: the guard over both rules.

## result

- **The guard** (d9c467292): `refusal` reads each simple command's program, after `NAME=value` words and inside `$(` or
  a backtick, and refuses `python`, `python2`, `python3`, `python3.<n>`, `pip` and `pip3`, directly or by `uv run`
  (its value-taking options skipped) or `uvx`; `perl` with an option cluster holding `i` (not `-I`); `awk` or `gawk`
  with `-i inplace`, `-iinplace` or `--inplace`. A word that only names Python, in `grep`, a message or a heredoc body,
  is allowed, as are `uv run substrate`, `perl -ne` and `perl -I`.
- **The tests:** ten Python starts refused, Sprint 266's `python3 - 2>/dev/null; perl -0pi …` among them; five in-place
  edits refused; eight commands allowed. Two first drafts were caught by the tests: `uv run --with x python3` took `x`
  for the program, and `x=$(python3 -c 1)` read `x=$(python3` as one.
- **The watchlist:** the edits entry annotated with 262, 263 and 266; an entry added for the Python starts, 238, 240,
  250 and 266, to close when the hook is installed.
- **The gates:** `scripts/gates.sh` exit 0 in 305 s on d9c467292, copied from `store/gates/passes.tsv`; the timing stage 737 ns an exit at load 11.68.
- **Plant 1,** restored by `cp` and checked by `cmp`: `"python3"` left out of the interpreters; the test failed,
  `"python3 - 2>/dev/null; perl -0pi -e 's/a/b/' f.rs": perl editing in place: a multi-line edit goes through the Edit
  tool`, the command refused for its `perl` alone and its Python start passed.
- **Plant 2,** restored by `cp` and checked by `cmp`: the in-place check reading `-i` exactly; the test failed,
  `"perl -0pi -e 's/(a\\n)/$1b/' crates/w2m_vm/src/run.rs" allowed`.
