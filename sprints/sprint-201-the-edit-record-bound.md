# Sprint 201 — The edit record bound

```yaml
---
id: 201
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 198", reviewer-2's Medium 4 of its review of Sprints 196-198. The rule that a
multi-line change to a tracked file goes through the Edit tool is enforced by each sprint's own account, and the
account misclassified Sprint 196's `PathBuf` import; `crates/edit_guard`, the check Sprint 195 built, does not run
(installing it as a hook was refused as self-modification, and the user has been asked once). Nothing in a commit's
diff shows which tool made it, so the record is what can be bound: from Sprint 199 each tail entry quotes every `sed`
it ran.

- **The quoted commands checked:** from `FIRST_QUOTED_EDIT_SPRINT` (199) `signal_report --check` reads each closed tail
  entry's quoted `sed` commands (the backticked spans that begin `sed -i`) and runs `edit_guard::refusal` over each;
  a quoted command the classifier refuses fails the entry, named, unless the entry names it as a multi-line edit
  outside the Edit tool.
- **The library shared:** `signal_report` depends on `edit_guard`'s library, so the classifier that would run as the
  hook is the one that judges the record.

## prerequisites

- 200 (closed)

## context_files

- `crates/edit_guard/src/lib.rs`; `crates/signal_report/src/main.rs` (`closed_too_early`); `crates/signal_report/tests/check.rs`;
  BLACKBOARD Sprint 199's tail entry (its quoted `sed`s)

## signal contract

### Emits

- None: the judges are `signal_report`'s and `edit_guard`'s tests.

### Invariants

- No Python file added or changed; `--check` passes over Sprints 108-199.

## artifact contract

### Files modified

- `crates/signal_report/` (Cargo.toml, src/main.rs, tests/check.rs), `crates/edit_guard/src/lib.rs`

### Command exit codes

- `signal_report --check`, its tests and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** Sprints 199's and 200's quoted `sed`s pass the classifier; Sprint 201's own entry quotes its `sed`s and
  passes.
- **Planted faults:** (1) an entry quoting `sed -i '' '5a\ x' f` as one-line: `--check` names the command; (2) the same
  command named as a multi-line edit: `--check` passes; (3) the classifier's `a\` rule dropped: the test names the
  quoted command it no longer refuses.

## done criteria

A sprint's record of its shell edits is held to what the commands do, by the classifier the hook would run.

## self-review checklist

- [x] Scope names one verifiable property: a quoted multi-line `sed` called one-line fails `--check`.
- [x] context_files cite what to read first.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the edit record bound.
