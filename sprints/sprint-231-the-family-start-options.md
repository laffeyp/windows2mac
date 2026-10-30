# Sprint 231 — The family start options

```yaml
---
id: 231
status: done
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

- **One flag, not four:** `w2m-vm --start <json>` takes a family's start as one JSON object. Its keys are
  `vm_inputs`' fields: `arguments` (the command line `"C:\w2m\app\probe.exe" <arguments>`), `environment`, `gui`,
  `foreground_held`, `instruction_clock` and `output_file`. A key it does not know is an error.
  - The rows come from the output file in the program's directory, read before the directory is removed, else from
    standard output, as `run_in_vm` reads them (2949-2952).
  - `--instruction-clock` stays for the gates' `twoproc` run.
- **The values:**
  - The extract's seven family rows hold JSON objects of their values, each from its family's lines at the tag.
  - `family_known` reads them, refuses a key outside `START_KEYS`, and prints the object in the lookup's column 7.
  - Every other column of the 273 rows was unchanged (`diff` of columns 1-6 and 8).
  - The gates' family loop passes the column as `--start`, and the judge's "start options not applied" state is gone.
- **The 34 runs with start options all end at ExitProcess:**
  - The 6 that Sprint 230 held conform.
  - The environment families' 24 runs had stopped at CreateProcessW: started without options, the probe ran as its
    own parent, which starts the child. Started as the child, they end.
  - `windows`' 2 runs had stopped at their temporary file; with TEMP and TMP set as the frozen start sets them, they
    end.
  - These 26 ran under two rules Sprint 230 left out, `child_rows` (168-183) and `windows_rows` (338-390, with
    `without_message_0x60`, 311-335), now ported, and all 26 conform.
  - Departure: the card's scope named neither rule.
- **The counts:** 249 of 273 runs conform, from 217; 60 of 68 families in every run, from 54. The held file keeps 24
  stops: the warp to a deadline 10, FlsFree's form 8, OutputDebugStringW's form 4, SHGetKnownFolderPath's form 2.
- **The unit test** gains `child_rows` and `windows_rows` on rows worked by hand from the frozen lines: a drain's
  message 0x60 and its after row dropped, the drain renumbered, a later drain's `noremove_err` dropped, and a window
  placed from its origin against the default height 519.
- **A slip caught before it ran:** the first `default_height` expression held a dead term (`div_euclid(4 / 1)` times
  zero); it is now `((bottom - top) * 3).div_euclid(4) - 21`, Python's `//` floored.
- **Plant 1** (`local_free` 38092775612's start set to "-" in the lookup, the run made without it): "local_free 38092775612 ended; row null differs from windows.memory.local_free@0.1.0".
- **Plant 2** (`w2m-vm` reading standard output where an output file is named): "console 38210455282 ended; row std.input differs from windows.console.gui_start@0.3.0", and the environment runs at row `block`.
- **Plant 3** (the key `"foo":1` added to `local_free`'s start): `w2m-vm` printed "--start: foo is not a start option vm_inputs gives" and wrote no stop; the judge, with `W2M_REQUIRE_CASE_RUNS=1`, failed: "no build/family_runs/local_free-38092775612.stop (scripts/gates.sh writes it), and W2M_REQUIRE_CASE_RUNS=1 requires it".
- Each plant was restored by `cp`, `cmp` found the files equal, and the 34 runs were made again: all end, 249 conform.
- **Reviewer-3's review at 231,** which arrived with the gates running; that run was stopped with no pass line:
  - *Medium 1* (the extract's seven start rows entered by hand, held by nothing to the tag): a new test,
    `the_extract_start_options_are_the_tags`, reads `family_run.py` at the tag with `git show`. It requires each cited
    line to hold its text and each value's Python expression to stand in one of them. It then evaluates the
    expression (a literal, `True`, an f-string over `{start}`, a tuple of literals, `child`) and holds the extract's
    JSON to it, both ways. Two plants were named: a TMP path changed to `…\Local\Tmp`, "windows environment:
    store/frozen_python/family-run-2026-10-29.tsv has [...Tmp"], frozen-python-2026-10-29 [...Temp"]"; and the line
    2681 cited as 2682, "processes instruction_clock: family_run.py 2682 at frozen-python-2026-10-29 does not hold
    \"instruction_clock=True\"". Departure: the extract's header names the test.
  - *Low 1* (a missing output file read as empty rows): `w2m-vm` keeps the empty rows and prints "w2m-vm: the output
    file w2m-nothing.jsonl was not written; its rows are empty", run on a console binary given a file it does not
    write.
  - Low 2, the design point and the order are dispositions in BLACKBOARD: a Decision adopts `done` for the kit's
    `closed`; Deferred holds one start type for `--start` and `--game`; the Decision "Sequence after Sprint 214" puts
    the argument forms and the warp after the bookkeeping card.
