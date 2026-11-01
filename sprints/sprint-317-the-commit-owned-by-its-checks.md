# Sprint 317 — The commit owned by its checks

```yaml
---
id: 317
status: active
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-3's Low 1 at 316 (BLACKBOARD Surfaced) and the Drift watchlist's entry "A commit joined to its checks by
`;`": seven commits from Sprint 312 to 315 ran `scripts/pre-commit.sh` and `git commit` joined by `;`, one over a
failed format check (44b4730bc), the form Sprint 281 committed and WORKING_AGREEMENT's Sprint 281 amendment forbids.
The rule rests on a separator typed right at each commit; a script that owns the commit holds it once.

- **The work:** `scripts/commit.sh`, `/bin/sh` with `set -eu` as `scripts/pre-commit.sh` is: it refuses, by name, a
  tracked file whose working copy differs from the index (`git diff --quiet`), since the checks read the working tree
  and the commit takes the index; it records the index's tree (`git write-tree`), runs `scripts/pre-commit.sh`, and
  runs `git commit` with its own arguments only when the checks passed and the index's tree is the one recorded,
  exiting with the failing step's status, naming it;
  WORKING_AGREEMENT's "Before a work commit" opening with it, the amendments kept after it as the list's history.
- **Not here:** a hook; a change to the checks themselves.

## prerequisites

- 316 (closed)

## context_files

- `scripts/pre-commit.sh`; WORKING_AGREEMENT.md 253; the Drift watchlist's entry; reviewer-3's Low 1 at 316

## signal contract

### Emits

- None; a procedure made a script.

### Invariants

- No crate changes.

## artifact contract

### Files modified

- `scripts/commit.sh` new; `WORKING_AGREEMENT.md` ("Before a work commit")

### Command exit codes

- `scripts/gates.sh` returns 0; `dash -n scripts/commit.sh` returns 0

## observation contract

- **Expected values:** this card's work committed through `scripts/commit.sh`, the checks' output read whole and the
  commit made; `dash -n` passing.
- **Planted faults:** (1) a line of a Rust file staged unformatted: `scripts/commit.sh` exits non-zero naming the
  format check, `git log -1` unchanged; (2) a file staged after the tree is recorded, by a check that stages it (the
  script's tree compare tested by `git add` inside a stand-in check): no commit, the script naming the changed index;
  (3) a tracked file changed and left unstaged: no commit, the script naming the file.

## done criteria

No commit is made unless the checks passed on the index committed, whatever separator is typed.

## self-review checklist

- [x] Scope names one verifiable property: a commit only after passing checks on its own index.
- [x] context_files cite the script, the rule and the record.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each changing what its judge reads.
- [x] One concept: one script that owns the commit.

## result

