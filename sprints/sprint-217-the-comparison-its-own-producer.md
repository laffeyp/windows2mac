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
  gap between SESSION_INIT and EXECUTION_STARTED not counted, and the record's doc says so. *(Annotated in the work:
  `discrepancy_count` does not follow the verdict. Vocabulary 0.12's note on SESSION_COMPLETED says it counts
  DISCREPANCY_REPORTED, of which the judge writes none, so it stays 0, as Sprint 214's record wrote it.
  `run_frontier.py` 436 counts the divergences there, which departs from the note and is not followed. The session's
  close moves for T2 alone.)*
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

## result

- **The crate:** `call_compare.rs` and its test moved by `git mv` into `crates/call_compare`, a library and a
  binary; `w2m_vm` no longer names it. The library exports four types where the module exported 17 items: `Windows`
  (`load` from the repository root, `judge`, `reference_ref`, the contract's id and version), `Stream`, `Verdict` and
  `SetDivergence`; the readers, the pairing and the reference set are private.
- **The Producer:** `harness/w2m-vm-game.toml`'s trigger fires on EXECUTION_COMPLETED (seq 16 of record
  `record-1793309295000000000`); `call_compare` writes COMPARISON_COMPLETED (seq 19) and SESSION_COMPLETED (seq 20,
  last). `check_records` (T1-T3, E1) and `substrate replay --level 2` pass. `w2m-vm` writes SESSION_INIT through
  EXECUTION_COMPLETED; its `comparison` and `REFERENCE_RECORD` are gone. `scripts/gates.sh` builds the binary.
- **The verdict unchanged:** the COMPARISON_COMPLETED payload without its two ids hashes alike from this record and
  from a record the host at ab4df8dfa wrote the same hour (`shasum` 668ee8fb9f50b5f4c85b8932a88b796b678751cf). The
  frozen host's 10,505 calls: conforms, 0 defects, 24 values Windows varies; the Rust host's 6,698: conforms, 0
  defects, 1 past the cut, 506 scheduling, 249 values Windows varies, before the move and after.
- **The shape:** `Kind`, `Field`, `Space` and `Cause` are enums. `judged` is 32 lines, beside `paired`, `pool_partners`,
  `value_divergences`, `field_pairs` and `OneToOne`, where it was 196. Low 1: a signature value of an unknown or
  missing kind refuses the file; a pool's w2m workers are tried as they first appear; two `called` divergences of one
  function sort as w2m first calls them; each with a unit test that fails under the old order. Low 2: unit tests for
  `pools`, `thread_map`'s name-then-agreement pairing and its tie, and `placed`. Low 3: `Guest::traced` reads the
  trace area once, and a records count past its `capacity` or a strings count past `W2M_TRACE_STRINGS` (2,048,
  call_trace.c 31) is an error.
- **Plant 1** (`stopped: false` in `main.rs`): "discrepancy: UnityPlayer.dll!SetEvent call 0 missing: Windows 0x1, w2m
  none: 11 of 11 reference executions disagree (run 38216081440 seq 3031) [a Windows thread with no w2m partner]". The
  card's 76 was Sprint 214's stop; the stream at Sprint 215's stop leaves 1 past the cut. Restored byte for byte.
- **Plant 2** (`specs/signatures.json` line 365's kind made "integer"): the judge exited 1, "call_compare: signatures:
  AddVectoredExceptionHandler has a value of kind Some(\"integer\")". Restored byte for byte.
- **Plant 3** (the trigger `on` an event never written): substrate exited 0; `check_records` counted T2's one
  violation, and `batch_record.rs` failed, "0 COMPARISON_COMPLETED where the reference has one". Restored byte for byte.
- **Departures:** (1) the verdict is "inconclusive" when the trace left calls out, with first divergence "the trace
  left out N calls", as `run_frontier.py`'s comparator gives it (its Sprint 064 rule); Sprint 214's port left this out,
  so a trace with calls dropped would have read "conforms". `dropped` is read from the stop file's `calls dropped` row,
  0 at today's stop; the integration test holds the rule. (2) `Verdict::verdict` returns the vocabulary's
  `events::Verdict`, not a string. (3) `batch_record.rs` now requires one COMPARISON_COMPLETED judging the run, after
  EXECUTION_COMPLETED, and SESSION_COMPLETED last; it had read the comparison only if present. (4) A file outside the
  card's list: `crates/guest_runtime/src/layout.rs` gains `TRACE_STRINGS`, the bound Low 3 needs.
- **After reviewer-3's review of Sprints 215-217:**
  - **Medium 1:** the session's `outcome` follows the verdict, as this card's scope says: `Verdict::outcome` gives "completed_with_discrepancies" on a discrepancy. Before, it was "completed" always, carried from Sprint 214's `record.rs`. The unit test `the_session_closes_with_discrepancies_on_a_defect` failed under the old rule: "left: [(\"discrepancy\", \"completed\"), …]".
  - **Medium 2:** WORKING_AGREEMENT's canonical-home row names `crates/call_compare`.
  - **Low 1:** a missing `elapsed_seconds` is an error. `main.rs`'s doc says the sum is a lower bound, leaving out the process start, the signing and substrate's dispatch. The Rust EXECUTION_COMPLETED's `elapsed_seconds` includes the depot's read and load; the frozen host's clock starts later (`run_frontier.py` 322).
  - **Low 2:** `Verdict`'s fields are read through accessors.
  - The gates run then going was stopped with no pass line, and run again.
