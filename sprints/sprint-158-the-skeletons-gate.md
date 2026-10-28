# Sprint 158 — The skeleton's gate: `sleeper`'s output and record from the Rust host equal the stored ones

```yaml
---
id: 158
status: done
phase: 6
pass_kind: functional
---
```

## scope

Link 6 of `docs/design-host-compile-path-2026-10-24.md` ("the skeleton's gate: the guest's output and record equal the
stored ones"), and step 3's gate in `docs/rust-port-plan-2026-10-24.md` ("It loads the guest, serves its exits, writes
its record through substrate, and ends with the guest's stored output"). Since Sprint 157 the Rust host runs
`tests/data/timed_waits/sleeper.exe` to ExitProcess(0) with the Python host's output, but only inside a test, and it
writes no record.

The Python host's record of a guest run is `record_translations` (`src/w2m/vm_backend.py` 5570-5612, called by
`scripts/run_skeleton.py` 153 and `tests/test_guest_runtime.py` 98): one substrate session, purpose `compilation`:
SESSION_INIT; for each block translated, in compile order (`vm.translations`, appended by `_vm_compiled` 2577 for the
C's blocks at each exit and by `_compile` 4638 for the host's), CODE_TARGET_DISCOVERED (discovery `static` for the
entry, else `runtime_transfer`; stage `execution`) and TRANSLATION_PUBLISHED (its guest range, code version 0, the
decoder spec's version, artifact `vm-code-cache:<code address>`); SESSION_COMPLETED.

This sprint:

- the run's translations in compile order in `Outcome` (start, end, code address), as `vm.translations` and
  `vm.blocks` hold them;
- `w2m-vm`, the Rust host as a program and a substrate Producer: it loads a guest, runs it on one vCPU with the host
  compiling, writes the guest's standard output to a file it is given, and writes `record_translations`' events in the
  vocabulary's types; a run that does not end at ExitProcess fails the Producer, naming the stop;
- `harness/w2m-vm-sleeper.toml`, the topology that runs it on `sleeper`;
- the stored ones: the frozen Python host's `record_translations` of `sleeper` and its output, made by running it,
  committed under `store/runs/` and `store/skeleton/`;
- the golden check in Rust: the Rust record's w2m events equal the Python's in order, field for field, but for
  `elapsed_seconds` (a time) and `harness_version` (each implementation's own), and the output file equals the stored
  output.

## prerequisites

- 157 (closed)

## context_files

- `src/w2m/vm_backend.py` 2559-2580, 4600-4640, 5570-5612; `scripts/run_skeleton.py`; `tests/test_timed_waits.py`;
  `crates/w2m_vm/src/run.rs`, `crates/w2m_vm/src/bin/case_runs.rs` (a Producer in Rust), `crates/w2m_vm/src/ledger.rs`
  (`frames`), `harness/case-runs-rust.toml`, `crates/events`

## signal contract

### Emits

SESSION_INIT, CODE_TARGET_DISCOVERED, TRANSLATION_PUBLISHED, SESSION_COMPLETED: the skeleton's record, from `w2m-vm`,
on a substrate record under `store/runs/`. All four tags are in vocabulary 0.10 and fired by the Python host's
`record_translations`.

### Consumes

`tests/data/timed_waits/sleeper.exe`; the frozen Python host's record and output of `sleeper` (stored, as above);
Sprint 156's fixture (`build/run_loop/sleeper-python.json`, its translations in order).

### Invariants

- No Python file added or changed.
- Sprints 155-157's judges pass unchanged.

## artifact contract

### Files created

- `crates/w2m_vm/src/bin/w2m-vm.rs`, `crates/w2m_vm/src/record.rs` (the record's events and the golden check),
  `harness/w2m-vm-sleeper.toml`, `crates/w2m_vm/tests/skeleton_gate.rs`; the Python's and the Rust's records under
  `store/runs/`; `store/skeleton/sleeper-stdout.txt`

### Files modified

- `crates/w2m_vm/src/run.rs`, `crates/w2m_vm/tests/run_loop.rs`, `WORKING_AGREEMENT.md`, `BLACKBOARD.md` (Sprint 157's
  CI run, in the tail entry)

### Command exit codes

- `uv run substrate run --file harness/w2m-vm-sleeper.toml --root store/runs/w2m-vm-sleeper-<ns> --strict` returns 0
- `scripts/gates.sh` returns 0

## observation contract

- **Expected values, from the Python host:** the Rust run's translations, in order, are the Python's 19 (start, end,
  discovery) of `sleeper`. The Rust host's record holds the Python's events in the same order with the same payloads
  but for `elapsed_seconds` and `harness_version`; the output file is `woke\ndone\n`, the stored output.
- **Planted fault:** two translations swapped in the record, or the output's last byte dropped: the golden check fails
  and names the event or the output.

## done criteria

`w2m-vm` runs `sleeper` under substrate, and its record and output equal the frozen Python host's, judged in Rust.

## self-review checklist

- [x] Scope names one verifiable property: the skeleton's record and output equal the stored ones.
- [x] context_files cite the Python carried.
- [x] Emits: four tags of vocabulary 0.10, each fired today by the Python host.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the Python host's record as reference, and a planted fault.
- [x] One concept: the skeleton's gate.

## result

- **The skeleton's gate is met for `sleeper`.** `uv run substrate run --file harness/w2m-vm-sleeper.toml --root
  store/runs/w2m-vm-sleeper-1793149355000000000 --strict` returned 0. `w2m-vm` loaded `sleeper.exe`, ran it on one
  vCPU to ExitProcess, wrote `woke\ndone\n` to `build/skeleton/sleeper-stdout.txt`, and wrote 40 w2m events:
  SESSION_INIT, 19 blocks of CODE_TARGET_DISCOVERED and TRANSLATION_PUBLISHED, and SESSION_COMPLETED.
  `crates/w2m_vm/tests/skeleton_gate.rs` holds them equal, in order and field for field, to the frozen Python host's
  `record_translations` of the same guest (`store/runs/translations-1793149239949363000`): the same 19 guest ranges,
  discoveries and code addresses (`vm-code-cache:7f0000062920` first). The output equals the stored output,
  `store/skeleton/sleeper-stdout.txt`.
- Three fields are not compared, each named in `w2m_vm::record::EXEMPT`: `elapsed_seconds`; `harness_version` (the
  Python's 0.0.1, the crate's 0.1.0); `vocab_version` (the frozen Python writes 0.9, the Rust vocabulary is 0.10). The
  four tags' fields are the same in both.
- **The translations in compile order.** `Outcome.translations` keeps the C's blocks as its log gives them at each exit
  and the host's at each compile, as `vm.translations` does. `qpc`'s 8 and `sleeper`'s 19 equal the Python's in
  order, (start, end, discovery); both run-loop tests now check it.
- The record's check runs everywhere, CI included: both records are committed and `record.rs` needs no
  Hypervisor.framework. The output's check needs the run's file under `build/`, which the gates require.
- The record's baseline names `host = "w2m-vm"`, not `judge`: the judges' ledger reads records whose baseline names a
  judge, and this one holds no verdict.
- **Planted faults.** A copy of the Rust record with its first two blocks' events swapped: the gate failed at "event 1
  (CODE_TARGET_DISCOVERED): discovery is "runtime_transfer" where the stored record has "static"". The output file
  without its last newline: the gate failed at "the output", `"woke\ndone"` against `"woke\ndone\n"`. Restored, both
  checks pass.
- Edits outside the Edit tool, recorded: `sed` made six one-line changes (two in `run.rs`, two calls added in
  `run_loop.rs`, the `pub mod` line in `lib.rs`, the topology's baseline key) and set and restored the test's record
  name for the plant, and the planted record was built with
  `awk`; each result was read back.
