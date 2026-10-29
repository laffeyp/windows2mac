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
