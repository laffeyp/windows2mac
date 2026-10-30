# Sprint 231 — The family start options

```yaml
---
id: 231
status: active
phase: 6
pass_kind: functional
---
```

## scope

BLACKBOARD Deferred 2026-10-30 (Sprint 230), "Family start options", and plan step 4's "68 of 68 families": the six
ended family runs not started as their family starts them.

- **Where it stands, by lookup (2026-10-30):**
  - `w2m-vm` takes a guest, its output, a beside DLL, the instruction clock and an instruction budget. It takes no
    command line, GUI start, environment block or output file.
  - The Rust `Start` already has `command_line`, `environment`, `gui` and `foreground_held`
    (`crates/w2m_vm/src/process.rs` 56-89).
  - The frozen start, `vm_inputs` and `run_in_vm` (`scripts/family_run.py` 2889-2955 at the tag), gives:
    - the command line `"C:\w2m\app\probe.exe" <arguments>`;
    - the environment block;
    - the GUI start and the foreground flag;
    - the rows read from the family's output file in the program's directory, when it names one, else standard output.
  - The values, at the tag:
    - `local_free`: arguments `safe` (1598);
    - `console`: GUI, `child`, `w2m-console-child.jsonl` (1381);
    - `time_and_error_mode_child`: GUI, `child`, `w2m-error-mode-child.jsonl` (439);
    - the environment families: GUI, `child-<start>`, `w2m-environment-<start>.jsonl` and their blocks (214-215);
    - `windows`: the TEMP and TMP block (2580).
  - `store/frozen_python/family-run-2026-10-29.tsv` records each family's option names, not their values.
- **The work:**
  - The extract gains each family's option values, read from the tag.
  - `family_known` prints them in the lookup.
  - `w2m-vm` takes `--command-line`, `--gui`, `--environment` and `--output-file`, each into `Start` as the frozen
    start sets it.
  - The gates' family loop passes them, and the judge drops its "start options not applied" state.
- **Not here:** the environment families' CreateProcessW stop and the windows family's stop, which their own cards
  meet.

## prerequisites

- 230 (closed)

## context_files

- `scripts/family_run.py` at the tag (`vm_inputs`, `run_in_vm`, the families above)
- `crates/w2m_vm/src/bin/w2m-vm.rs`, `crates/w2m_vm/src/process.rs` (`Start`), `crates/w2m_vm/src/bin/family_known.rs`
- `store/frozen_python/family-run-2026-10-29.tsv`, `store/family_runs/`, `scripts/gates.sh` (the family loop)

## signal contract

### Emits

- None: the families' runs write no record on the Rust host.

### Invariants

- The 217 runs that conform still conform. A run is judged only when started as its family starts it.

## artifact contract

### Files modified

- `w2m-vm.rs`, `family_known.rs`, the extract and the lookup, `family_runs.rs`, `scripts/gates.sh`, the held file,
  census row 6

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the six runs judged; each conforms or is held by its first differing row; runs conforming of
  273, against 217.
- **Planted faults:**
  1. `local_free`'s argument dropped in the lookup: its run prints the parent's rows and is held at row `null`.
  2. `--output-file` read from standard output instead: the console runs are held by a named row.
  3. A start option the gates do not pass: the judge names the run and the option.

## done criteria

Every family run that ends was started as its family starts it, and is judged.

## self-review checklist

- [x] Scope names one verifiable property: every ended family run started as its family starts it.
- [x] context_files cite the frozen start, the Rust start, the lookup and the loop.
- [x] Emits: none, with the reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the families' start options.

## result
