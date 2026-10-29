# Sprint 217 — The comparison its own Producer

```yaml
---
id: 217
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 214", its third item, the part reviewer-3's Medium 5 and Medium 6 and Lows 1-3 of
its review of Sprints 212-214 name. `w2m-vm`'s `comparison` reads the repository's fixtures at run time and judges the
run inside the host binary; `call_compare` lives in `w2m_vm`; its causes and fields are strings; `judged` runs 196
lines.

- **The crate:** `crates/w2m_vm/src/call_compare.rs` and `tests/call_compare.rs` move by `git mv` into a crate
  `call_compare`, a library and a binary of that name. `w2m_vm` no longer depends on it. The module makes 17 items
  `pub` today (`read_tsv` through `load_reference`); the crate makes public only what its binary and its tests call,
  one entry from the files to a `Verdict`, and keeps the readers and the pairing private.
- **The Producer:** `harness/w2m-vm-game.toml` gains a `[[trigger]]` on EXECUTION_COMPLETED that starts the binary;
  its input is that event's payload (substrate `adapters/topology_file.py` 140). It reads the call stream and strings
  files w2m-vm wrote, from paths on its command line as w2m-vm's are; a stopped run is one whose `exit_status` is not
  `exited` (Low 4's reading of the stop file's text goes). It writes COMPARISON_COMPLETED with `target_execution_id`
  the event's `execution_id`.
- **The session's close:** check_records' T2 holds SESSION_COMPLETED last, and its `discrepancy_count` and `outcome`
  follow the verdict, so the judge writes it; w2m-vm writes SESSION_INIT through EXECUTION_COMPLETED. The trigger's
  input carries no session start, so `elapsed_seconds` is the execution's `elapsed_seconds` plus the judge's own, the
  gap between SESSION_INIT and EXECUTION_STARTED not counted, and the record's doc says so.
- **The shape:** a `Cause` enum (scheduling, a Windows thread with no w2m partner, after a value divergence at a
  sequence number) and a `Field` enum (argument 0-7, return, last error, called, missing) in place of the strings;
  `judged` split by what it judges; an unknown signature kind refused (Low 1), ties broken by first appearance; unit
  tests for `thread_map`'s names, `pools` and `placed` (Low 2); one trace-area reader in `guest.rs` for calls and
  strings, its count bounded by the area (Low 3).
- **Not here:** the coverage on the record (Medium 4) and the reference cut at the matched point (Medium 2's rest)
  change what a stopped run's verdict covers and need metrics vocabulary 0.12's closed enum lacks; they are Sprint
  218's, with a vocabulary proposal.

## prerequisites

- 216 (closed)

## context_files

- `crates/w2m_vm/src/call_compare.rs`, `crates/w2m_vm/tests/call_compare.rs`, `crates/w2m_vm/src/bin/w2m-vm.rs`
  (`comparison`, `game`), `crates/w2m_vm/src/record.rs` (`execution_lines`, `Comparison`), `crates/w2m_vm/src/guest.rs`
  (`traced_calls`, `traced_strings`)
- `harness/w2m-vm-game.toml`; `substrate/docs/process-producer.md` (the topology file's `[[trigger]]`)
- `crates/w2m_vm/src/bin/check_records.rs` (T1, T2); `crates/w2m_vm/tests/batch_record.rs`

## signal contract

### Emits

- COMPARISON_COMPLETED and SESSION_COMPLETED from the `call_compare` Producer, vocabulary 0.12, no new tag.

### Invariants

- The batch run's record passes `check_records` and `substrate replay --level 2`; its COMPARISON_COMPLETED equals the
  one w2m-vm wrote at Sprint 216's close field for field.

## artifact contract

### Files modified

- `crates/call_compare/` (new, by `git mv`), `crates/w2m_vm/` (`src/bin/w2m-vm.rs`, `src/record.rs`, `src/guest.rs`,
  `src/lib.rs`, `Cargo.toml`, `tests/batch_record.rs`), `harness/w2m-vm-game.toml`, `Cargo.toml`, `scripts/gates.sh`
  if its crate list names `w2m_vm`'s comparison

### Command exit codes

- `cargo nextest run -p call_compare -p w2m_vm`, the batch run under substrate with `--strict`, `check_records` over
  its record and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the same verdict, `reference_ref`, `first_divergence` and divergence counts (defects, past the
  cut, scheduling, varies) as before the move; SESSION_COMPLETED last on the record, from the judge's Producer.
- **Planted faults:** (1) the judge given `exit_status` `exited` for the stopped run: the 76 past the cut are named as
  defects; (2) an unknown signature kind in a copy of `specs/signatures.json`: refused, named; (3) the trigger removed
  from the harness: check_records names T2 or the batch record's test names the missing comparison.

## done criteria

The batch run's verdict is written by its own Producer, from the record's EXECUTION_COMPLETED and the files w2m-vm
wrote, and equals the verdict w2m-vm wrote before.

## self-review checklist

- [x] Scope names one verifiable property: the judge outside the host, its verdict unchanged.
- [x] context_files cite the code moved and the harness rule it uses.
- [x] Emits named, no new tag.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the comparison as its own Producer; coverage and the cut held for 218, with the reason.
