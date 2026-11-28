# Sprint 378 — The version and system-info tables re-taken from the laptop's stored rows

```yaml
---
id: 378
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Which Windows w2m answers as, the reference being the laptop" (Sprint 374) set w2m to answer as the
laptop's Windows 11 client build 26200, and named the tables still taken from GitHub's Server runner:
`tools/guest_runtime/version.c` and `system_info.c` first. This card re-takes those two from the laptop's stored rows,
`store/probe-runs/lap-20261013T231059422Z-136bec80-*` (the version rows begin at `-186`), with Sprint 374's
four-outcome comparison (`crates/windows_machine/tests/runner_families_on_the_laptop.rs`).

- **The rows.** For each value `version.c` and `system_info.c` give a program (GetVersionEx, RtlGetVersion,
  VerifyVersionInfo's masks, GetSystemInfo, GetNativeSystemInfo, IsProcessorFeaturePresent and the rest the two files
  hold), the laptop row that reports it, named by run and line, and its October and today's value where a run of build
  26200.9974 holds it (`lap-20261127T*`, `lap-20261128T*`).
- **The change.** Each value set to the laptop's; a value the laptop gives that the user or a Decision already settled
  otherwise (CPUID feature 89, the processor count of 4, audio) is left as settled and named with its Decision.
- **Two rules G3 needs.** A value that differs only between builds 26200.7840 and 26200.9974 is stated as G3 treats it
  (undetermined, scheduling or a divergence), from the rows; and whether runner run 38216081440 stays in G3's reference
  set for the values these two files give, by the rule "a machine-decided row" (Decision "Which machine-decided values
  w2m answers as").
- **Not here:** `system_messages.json` and `windows_exports.json` (card 379); `recorded_folders.json` and the display
  calls (card 380). No API id is added.

## prerequisites

- 377 (closed)

## context_files

- `tools/guest_runtime/version.c`, `tools/guest_runtime/system_info.c`.
- `crates/windows_machine/tests/runner_families_on_the_laptop.rs` (the four outcomes), `store/family_runs/known-2026-10-29.tsv`.
- `store/probe-runs/lap-20261013T231059422Z-136bec80-186/` (one run read in full: `body-1.jsonl` by its first 60 lines).
- `grep -n "Which Windows w2m answers as\|Which machine-decided values w2m answers as\|The order after Sprint 377" BLACKBOARD.md`.
- `WORKING_AGREEMENT.md` lines 137-266.

## signal contract

### Emits

- None: the change is to two guest tables; the runs that judge them are recorded by the gates' existing steps.

### Invariants

- Every run as held; G3's batch runs conform, or a divergence is named by the call and value that moved; no Windows run
  on GitHub's runner; `crates/guest_runtime/src/api_ids.txt` unchanged.

## artifact contract

### Files modified

- `tools/guest_runtime/version.c`, `tools/guest_runtime/system_info.c`, with others only where the class requires (a
  family judge or walk fixture the values move), reported to main when found. One concept: the two tables as the
  laptop gives them.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The table of values: call, field, w2m before, laptop 7840, laptop 9974 where held, w2m after, the row cited.
  - The families whose judges read these values (`version_*`, `system_info_*` and any other the class search finds),
    each equal to its laptop run after the change.
  - The build-only rule and the runner member's place, each with the rows it rests on; G3's verdict.
- **Planted faults:**
  - (1) One re-taken value set back to the runner's: the family judge that reads it fails naming the value.
  - (2) A row of the comparison marked equal where the laptop's value differs: the comparison's test fails.

## done criteria

`version.c` and `system_info.c` give the laptop's values, each cited to a stored laptop row, and the families that read
them are equal to their laptop runs.

## self-review checklist

- [x] Scope names one verifiable property: the two tables equal to the laptop's rows.
- [x] context_files cite the tables, the comparison, a laptop run and the Decisions.
- [x] Emits: none, with the reason.
- [x] Artifact contract: two files, gradable by command.
- [x] Observation contract: the value table, the families, the two rules; numbered plants.
- [x] One concept.

## result

GATES-LINE
