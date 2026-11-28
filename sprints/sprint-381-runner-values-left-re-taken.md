# Sprint 381 — The runner's values left in w2m re-taken from stored laptop evidence

```yaml
---
id: 381
status: active
phase: 6
pass_kind: functional
---
```

## scope

Sprints 379 and 380 re-took the version, system-info and message tables. A class search at Sprint 379 found values
still taken from GitHub's Server runner, and the Decision "Which machine-decided values w2m answers as" names the
display. This card re-takes each of them from stored laptop evidence (the user's rule "Match the reference",
BLACKBOARD 160), one concept: no value w2m gives a program is the runner's where a stored laptop row or profile gives
the laptop's.

- **The display.** The game's display calls (GetSystemMetrics 0x4c-0x4f, GetMonitorInfoA/W, GetDpiForMonitor,
  EnumDisplaySettingsW and any other the class search finds) read across the 14 reference game runs in
  `store/game-runs/` (runner 38216081440, the ten `lap-20261014T032442034Z-c8814547-*`, the three
  `lap-20261128T014720644Z-33b6423b-*`): for each, w2m's answer, the laptop's, the runner's, and whether the laptop's
  runs agree among themselves. w2m answers as the laptop where they do; a value the laptop's runs vary in is named.
- **The profile folder.** `tools/guest_runtime/known_folders.c` 29, `C:\Users\runneradmin`: the laptop's
  `C:\Users\w2m` (`store/reference_machine/profile-lap-20261128T042841141Z-4c0653e2.json` `user.profile`), and every
  other place the class search finds the runner's user name or profile path.
- **The temp-path fallback.** `tools/guest_runtime/environment.c` 216, `C:\Windows\`: the Windows directory Sprint 379
  measured, `C:\windows`, if Microsoft's GetTempPath2W/GetTempPathW page documents that fallback (cite it; if it does
  not, name the value as not measured).
- **The loader's system path.** `crates/w2m_vm/src/process.rs` 577, `C:\Windows\System32\`: re-taken only if a stored
  laptop row shows how Windows writes a module's path there (a GetModuleFileNameW of a system DLL, a strings row);
  otherwise named as not measured.
- Each re-taken answer that moves the game is measured as Sprint 379 measured, and the frozen stream's cut follows the
  Decision "The system messages re-taken, and where the frozen stream's cut falls".
- **Not here:** `specs/recorded_folders.json` and `specs/windows_exports.json`: their probes (`system_fonts`,
  `exports`) have no stored laptop run, and the laptop refuses new probe runs (blocked on the user). No API id is
  added.

## prerequisites

- 380 (closed)

## context_files

- `tools/guest_runtime/known_folders.c`, `environment.c`, the display answers' files (found by search),
  `crates/w2m_vm/src/process.rs` 560-600.
- `store/game-runs/` (the call traces of the 14 reference runs, read by tools, not whole), `store/reference_machine/`.
- `crates/call_compare/` (how G3 reads the display calls), `crates/w2m_vm/tests/batch_run.rs` (`FIRST_RETAKEN_CALL`).
- `grep -n "Which machine-decided values w2m answers as\|The version and system-info answers re-taken\|where the frozen stream's cut falls" BLACKBOARD.md`.
- `WORKING_AGREEMENT.md` lines 137-266 and 301.

## signal contract

### Emits

- None declared: the changes are to guest answers; the gates' existing runs judge them.

### Invariants

- Every run as held; G3's batch runs conform, or a moved call is named and measured; no Windows run on GitHub's
  runner and none on the laptop; `crates/guest_runtime/src/api_ids.txt` unchanged.

## artifact contract

### Files modified

- `tools/guest_runtime/known_folders.c`, `environment.c`, the display answers' files, `crates/w2m_vm/src/process.rs`
  where a row supports it, their tests and the class the search finds; file count recorded.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The display table: call, argument, w2m before, the laptop's (and whether its 13 runs agree), the runner's, w2m after.
  - Each string re-taken, with its source row or profile field, and each named as not measured.
  - 274 family runs; G3 on one and two vCPUs; the frozen stream's cut row and share.
- **Planted faults:**
  - (1) One display answer set back to the runner's: G3 or a family judge fails naming the call.
  - (2) The profile path set back to `runneradmin`: a judge fails naming the row.

## done criteria

No value w2m gives a program is the runner's where stored laptop evidence gives the laptop's, and the values no stored
row holds are named.

## self-review checklist

- [x] Scope names one verifiable property: the runner's values replaced where stored laptop evidence exists.
- [x] context_files cite the files, the game runs, the profile and the Decisions.
- [x] Emits: none, with the reason.
- [x] Artifact contract gradable by command; file count recorded.
- [x] Observation contract: the display table, the strings, the runs; numbered plants.
- [x] One concept.

## result

GATES-LINE
