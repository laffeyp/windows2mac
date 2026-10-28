# Sprint 196 — The module families' judge

```yaml
---
id: 196
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 193", reviewer-2's Medium 5 of its review of Sprints 190-192 (nothing before
UnityMain is judged) and Medium 1 of its review of 185-189 (`in_app_directory`, 0 for a name found nowhere, `load_dll`).
Census row 6, the family judge, is absent in Rust. On 2026-10-28 `w2m-vm` ran one runner binary of each module family:
`windows.module.system` ends and its 14 rows equal the contract's; `windows.module.names_ansi` ends and its 10 rows
equal the contract's once its derivation drops `w_len` and `ret`; `windows.module.queries` stops at its first
LoadLibrary, `w2m-no-such-module.dll`, "a DLL file after the start, not served"; `windows.module.search` stops at
FILE_MKDIR 25.

- **The judge:** `crates/w2m_vm/tests/family_runs.rs` holds, for each of the four families and each runner binary its
  contract names (24 binaries), the rows `w2m-vm` printed, derived as `scripts/family_run.py`'s `Family.rows` derives
  them (raw, or names_ansi's drop, the excluded rows dropped), equal to the contract's `acceptance.rows` by key `s`; a
  family that stops is held to its stop by name in `store/family_runs/module-stops-2026-10-28.tsv`. `scripts/gates.sh`
  runs the 24 binaries into build/family_runs/.
- **The modules' answers:** `modules.rs` ports `in_app_directory` and returns 0 for a name found nowhere, so
  `windows.module.queries` runs past `w2m-no-such-module.dll`; a DLL file the probe's directory holds (`load_dll`) is
  served when `w2m-vm` has the probe's directory, otherwise it stops by name.
- **Bound:** `windows.module.search`'s FILE_MKDIR 25 is the file bridge's, the card after the second process; its stop
  is held.

## prerequisites

- 195 (closed)

## context_files

- `scripts/family_run.py` 84-142, 493-700, 1640-1860, 2841-2990; `src/w2m/modules.py` 94-200; `crates/w2m_vm/src/modules.rs`;
  `contracts/windows.module.{queries,system,names_ansi,search}/0.1.0.json`

## signal contract

### Emits

- None: the judge is a test over `w2m-vm`'s output.

### Invariants

- No Python file added or changed; the batch run's stream and the guests' stops as before.

## artifact contract

### Files modified

- `crates/w2m_vm/tests/family_runs.rs` (new), `crates/w2m_vm/src/modules.rs`, `scripts/gates.sh`,
  `store/family_runs/module-stops-2026-10-28.tsv` (new), `docs/step-4-census-2026-10-28.md`

### Command exit codes

- `family_runs.rs`, `modules.rs`'s unit tests and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** `system` and `names_ansi` conform on all 12 of their binaries; `queries` passes
  `w2m-no-such-module.dll` and conforms or stops at a later row, named; `search` stops at FILE_MKDIR 25.
- **Planted faults:** (1) names_ansi's derivation dropping nothing: the judge names `file.image`; (2) a name found
  nowhere answered with kernel32's handle: the judge names `load.missing`; (3) a system row's value changed in the
  contract read: the judge names the row.

## done criteria

The four module families are judged on the Rust host by their contracts, row by row, and every one that does not
conform stops by a name that is held.

## self-review checklist

- [x] Scope names one verifiable property: the module families' rows equal their contracts or stop by a held name.
- [x] context_files cite the Python and the contracts.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the module families' judge.

## result

- **The judge:** `crates/w2m_vm/tests/family_runs.rs` reads each module contract's runner binaries from its evidence
  ("probe sha256:"), 12 in all, not the card's 24; `gates.sh` runs each as probe.exe under build/family_runs/. A run that
  ends is held to the contract's rows by key `s`, derived as `Family.rows` derives them (names_ansi drops `w_len` and
  `ret`); a run that stops is held to `store/family_runs/module-stops-2026-10-28.tsv`.
- **The result:** 10 of 12 conform: windows.module.queries 6 of 6 (18 rows), .system 2 of 2 (14 rows), .names_ansi 2 of
  2 (10 rows); windows.module.search's 2 stop at FILE_MKDIR 25, held.
- **The modules' answers:** `module_in` resolves a LoadLibrary name as file_bridge.py's `host_file` does, a full C:\
  path or one relative to C:\w2m\app, against `Start::app_root` (the depot for the batch run; for a probe run a
  directory holding only the program); a DLL file found stops by name (no file bridge), a System32 name stops by name,
  a name found nowhere is 0. Unit test `a_load_of_a_name_found_nowhere_is_0`, with mono's full and relative paths
  (reviewer-2's High 2 of its review of 193-196: the first version checked only the top level).
- **The batch run:** its stream (rows 0-15 and row 16 in flight) and the guests' stops unchanged.
- **Plant 1** (names_ansi's derivation dropping nothing): the judge failed, "windows.module.names_ansi binary
  101395b5…: row file.image differs from the contract's". Restored byte for byte.
- **Plant 2** (a name found nowhere answered with kernel32's handle): the judge failed, "windows.module.queries binary
  dc5dc179…: row load.missing differs from the contract's". Restored byte for byte, rebuilt, the six queries binaries
  run again, the judge passes.
- **Plant 3** (the system contract's row `first` with `non_null` 2): the judge failed, "windows.module.system binary
  202f2b6b…: row first differs from the contract's". Restored byte for byte.
- **Plant 4** (the lookup held to the top level): the modules test failed, "called `Result::unwrap_err()` on an `Ok`
  value: 0". Restored byte for byte, rebuilt.
- **Departures:** 12 binaries where the card said 24; the first gates run stopped in its Python stage to take High 2; `load_dll` stops by name for a DLL file beside the program, since
  the Rust host has no file bridge.
