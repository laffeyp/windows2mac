# Sprint 277 — `edit_guard` refuses every `sed -i` that changes the line count

```yaml
---
id: 277
status: active
phase: 6
pass_kind: functional
---
```

## scope

Sprint 276 broke the edit rule (WORKING_AGREEMENT 291) four times, and its tail roll inserted Sprint 266's entry into the
archive by `sed -i '343r file'`. Fed by hand to `edit_guard` on 2026-10-31, the guard refused the `\n` replacement, the
range deletion and the Python start, and passed the `r` insertion (exit 0). The guard's `sed` check
(`crates/edit_guard/src/lib.rs` 140-193) knows `a\`, `i\`, `c\`, an address range, a newline and a `\n` escape. Under
`-i` sed writes its whole output back to the file, so every command that prints, reads, joins or ends early changes the
line count. The forms the check passes:

- `r` and `R` (a file's lines read in); `a`, `i` and `c` without the backslash (GNU's one-line form);
- `G` and `N` (a line added or two joined), `D` (part of a pattern deleted);
- `p`, `P`, `=`, `l` and GNU's `F`, and `s///p` (a line printed into the file); `q` and `Q` (the file cut after a
  line); `!` (a command on every line but the address's, as `/x/!d`);
- `w` and `W`, `s///w` (another file written), `e` and `s///e` (a command run);
- `-n` with `-i` (the file reduced to the printed lines); `-f` (a script the guard cannot read);
- commands inside `{ }` (the check reads the first command after an address only).

This card makes the check refuse each, so the guard, once installed, refuses the whole class, not the forms seen so far.

- **The work:** `multi_line_script` reads each command of the script, inside braces too, past its address, and
  refuses the commands above and an `s` command whose flags hold `p`, `w` or `e`; `refusal` refuses `sed -i` with `-n`
  or `-f`. Tests: each form refused; Sprint 276's `343r` among them; the allowed forms (`s` replacing one line with
  one, a one-line `d`, `y`, reading `sed -n`) still allowed.
- **Not here:** installing the hook, which WORKING_AGREEMENT 291 leaves to the user's own settings.

## prerequisites

- 276 (closed)

## context_files

- `crates/edit_guard/src/lib.rs` 137-193 (`multi_line_script`), 314-362 (`refusal`'s `sed -i` branch), 367-478 (tests)
- `WORKING_AGREEMENT.md` 291; BLACKBOARD Drift watchlist (the edits entry)
- sed's commands: macOS `man sed` (BSD) and GNU sed's manual, https://www.gnu.org/software/sed/manual/sed.html

## signal contract

### Emits

- None; a refusal, held by tests.

### Invariants

- Every command the guard allowed and its tests name stays allowed.

## artifact contract

### Files modified

- `crates/edit_guard/src/lib.rs`, `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** each form above refused with a reason naming it; Sprint 276's `sed -i '' '343r …'` refused; the
  allowed forms allowed.
- **Planted faults:**
  - (1) `r` left out of the refused commands: the test names `343r`.
  - (2) the brace stripping removed: the test names `/x/{N;s/a/b/}`.

## done criteria

`edit_guard` refuses every `sed -i` command that can change the edited file's line count or write another file, and
allows a one-line substitution and a one-line deletion.

## self-review checklist

- [x] Scope names one verifiable property: the guard refuses the class, not the instances.
- [x] context_files cite the check, its caller, its tests and sed's manuals.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: the `sed -i` class in the guard.

## result
