# Sprint 214 — The call comparison against Windows

```yaml
---
id: 214
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 211" (census row 7; reviewer-3's Medium 3 of its review of Sprint 211: the ground
truth before more comparisons to the oracle). The Rust host's batch-run stream is held to the frozen host's, which
conforms to Windows over the reference's 168 log lines; nothing holds the Rust stream to Windows itself, so a frozen-host
defect the stream copies passes. `scripts/call_compare.py` (781 lines) is that comparison in the frozen host: the
Windows calls of the reference set (game run 38216081440 and its laps, `reference_set`), each run's capture thread left
out (`excluded`), threads mapped by name and order (`thread_names`, `thread_map`), each value judged by its kind and the
API's signature (`judge`, `rendering`, `carries_address`), the values Windows itself varies across the set held as
such (`undetermined`, contracts/windows.game.calls 0.1.0), and the first divergence by cause (`compare_set`,
`by_cause`).

- **The comparison:** a Rust `call_compare` (a crate, or a module of `w2m_vm` built on every platform) carrying those
  rules, read against the contract and the Windows rows before the Python (the user's rule of 2026-10-29), each
  departure from the Python named.
- **The judge:** the Rust host's batch-run stream, rows 0-3130 as of Sprint 213, compared to the Windows reference set
  over the rows it reaches: the verdict, and the first divergence by cause if any; a test holds it, and the frozen host's
  stream compared the same way gives the Python's recorded verdict (conforms, 0 divergences) over the same prefix.

## prerequisites

- 213 (closed)

## context_files

- `scripts/call_compare.py` (whole); `contracts/windows.game.calls/0.1.0.json`; `store/game-runs/38216081440/` and the
  laps the contract names; `store/frontier-calls/frontier-643a6283510c-1792623423346201000.tsv`;
  `crates/w2m_vm/tests/batch_run.rs`

## signal contract

### Emits

- COMPARISON_COMPLETED on the batch run's record, as the frozen host's frontier writes it (run_frontier.py), with the
  verdict and the first divergence; TRACE_INGESTED only if the Rust host writes a guest-plane trace, else the
  comparison's input is named in `unobserved` (P22).

### Invariants

- No Python file changed; the stream and the ledger as before.

## artifact contract

### Files modified

- the comparison's crate or module and its tests; `crates/w2m_vm/src/record.rs` (the event); `scripts/gates.sh` (the
  comparison run); `docs/step-4-census-2026-10-28.md` (row 7)

### Command exit codes

- the comparison's tests, `batch_record.rs`, `check_records`, `substrate replay --level 2` and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the Rust stream's verdict against Windows over its rows (conforms, or a first divergence named
  by row, thread, function and value); the frozen host's stream over the same rows gives the same verdict, or the
  difference is a frozen-host defect the stream copied, named.
- **Planted faults:** (1) a return value changed in the Rust stream: the comparison names the row; (2) a varying value
  held as fixed: the frozen host's stream diverges where it conforms, named; (3) the capture thread not left out: the
  thread map fails by name.

## done criteria

The Rust host's batch run is compared to Windows, not only to the frozen host, over every row it reaches, and the
verdict is on its record.

## self-review checklist

- [x] Scope names one verifiable property: the verdict against Windows over the Rust stream.
- [x] context_files cite the Python and the Windows rows.
- [x] Emits: COMPARISON_COMPLETED.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: one comparison ported.

## result

- **The comparison:** `w2m_vm::call_compare` (built on every platform) carries `call_compare.py`'s rules: the traces
  read, `judge` and `failed` by the signatures' kinds, the assigned values' one-to-one maps, the address-carrying
  fields, the hinted reservations left out (`placed`), threads named by SetThreadDescription and paired by name then
  agreement, job pools paired per function, the set joined on w2m's side, divergences by cause. Read against the
  contract first: its `excluded_threads` and `undetermined` are the rules' inputs; no rule was found wrong against it.
- **The port's check:** the frozen host's whole stream of 2026-10-21 against the eleven runs its record names gives
  "conforms", 0 defects, 0 scheduling, 24 values Windows varies, the verdict its record holds and the count
  docs/postmortem-2026-10-21.md 56 gives.
- **The run:** the Rust host's stream, 3,131 rows, conforms: 0 defects, 1,122 scheduling divergences, 394 values Windows
  varies, and 76 divergences past the cut (calls of Windows threads the stopped run had not started), the same 76 the
  frozen host's same rows show. `w2m-vm --game` writes COMPARISON_COMPLETED (state_relation, the eleven runs, conforms)
  after EXECUTION_COMPLETED; `batch_record.rs` holds its contract and set to the 10-21 record's and its verdict;
  `check_records` and replay pass.
- **The thread names:** `Guest::traced_strings` reads call_trace.c's string arguments (`TRACE_STRINGS_OFF` 0x308300,
  136-byte records, call_trace.c 57-63) and `w2m-vm --game` writes them beside the call stream; the first 21 equal the
  frozen host's.
- **Plant 1** (a HeapAlloc return set to null in the Rust stream): the judge failed, "UnityPlayer.dll!HeapAlloc call 0
  ret: Windows 0x24265b37a80, w2m 0x0: 11 of 11 reference executions disagree". Restored byte for byte.
- **Plant 2** (the contract's undetermined values not read): the frozen host's stream failed, 22 defects, beginning
  "UnityPlayer.dll!InterlockedPushEntrySList call 13 ret". Restored byte for byte.
- **Plant 3** (the capture thread not left out): the frozen host's stream failed, "values Windows varies, left: 111
  right: 24". Restored byte for byte.
- **Departures:** the reference set is the one the frozen host's run of 2026-10-21 names in its record; the set's
  membership rules (`outside_set`: thread names, records, arguments, processor count) are not ported. A stopped run's
  cut is new: the frozen host compared whole runs; the calls of a Windows thread with no w2m partner are past the cut of
  a run stopped before ExitProcess. The thread names' strings file was not in the card's files; without it the
  threads pair by agreement only, which Sprint 064 found mispairs. Plant 1's first try changed a void return, which no
  rule compares, and reached no judge; a pointer return was planted instead. Plant 3 first passed the defect count, and
  the judge now holds the count of values Windows varies too. reviewer-3's review of Sprints 212-214 found the
  stopped run's judge blind to the scheduling divergences; it holds every divergence to the frozen host's same rows now,
  and a fourth plant, thread 8's fifth call dropped, failed it. COMPARISON_COMPLETED carries the verdict without its
  coverage (the rows compared, the 76 past the cut); the comparison's own card puts it on the record. The card named
  `scripts/gates.sh` for the comparison's run; gates.sh is unchanged, since the comparison runs inside the game's
  harness run (`w2m-vm --game`) and `tests/call_compare.rs`, both of which the gates already run. The files changed
  beyond the card's: `crates/guest_runtime/src/layout.rs`, `crates/w2m_vm/src/{guest.rs, lib.rs, bin/w2m-vm.rs}`,
  `crates/w2m_vm/tests/batch_record.rs`.
