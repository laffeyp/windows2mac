# Sprint 209 — The exit ledger whole

```yaml
---
id: 209
status: done
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 208" (reviewer-2's High 1, Mediums 1, 2 and 4 of its review of Sprints 206-209).
The exit ledger holds each column's first difference only: the site column differs in 141 of the game's 1,024 rows
(the thunks' numbering: the Rust host's first dispatch-miss target 0x7ffe00003460 against the frozen host's
0x7ffe00003100, 54 thunks of 16 bytes apart) and the result column in 6, and past those rows the two columns judge
nothing. Its written column holds the request view's writes only.

- **The thunks:** the Rust host makes its thunks in the frozen host's order (`make_thunk`, the import thunks, the COM
  vtables' `_import_thunk`, the trace's own), so a thunk's address is the frozen host's.
- **The judge:** the held file lists every differing (process, row, column) outside the varying cells, and the
  child's stop row (its row 190, `hvc 12`, which the fixture writes at the stop) is named.
- **The writes:** every guest write the host makes while serving an exit is logged, at one point in each host (the
  Rust's guest-memory write paths; the fixture's `self.mem.write`, `_GuestView.write` and the `mm` writes a wrap can
  reach), the fixture recorded again.
- **The logs:** the three repeats' run logs go under store/frontier-logs/<run id>/, and census rows 4 and 5a cite them.
- **The Decisions:** `signal_report --check` holds each agent-resolved Decision from 2026-10-25 to one decision clause
  and one revisit clause.

## prerequisites

- 208 (closed)

## context_files

- `src/w2m/vm_backend.py` (`make_thunk` 2920-2924, `_import_thunk` 3087-3092, `_com_vtables` 3078-3085, the
  loader's thunks), `src/w2m/loader.py`; `crates/w2m_vm/src/guest.rs` (`Guest::new`'s thunks, `make_thunk`,
  `import_thunk`, `trace_ranges`), `crates/pe_loader`; `crates/w2m_vm/src/exit_ledger.rs`,
  `crates/w2m_vm/tests/exit_ledger.rs`; `crates/signal_report/src/main.rs`

## signal contract

### Emits

- None: the judges are `exit_ledger.rs`, `batch_run.rs` and `signal_report --check`. The record card follows.

### Invariants

- No Python file added or changed; the stream still equals the reference through row 924.

## artifact contract

### Files modified

- `crates/w2m_vm/src/{guest.rs, run.rs, host_request.rs, modules.rs, exit_ledger.rs}`, `crates/pe_loader/src/lib.rs`
  as the thunks need, `crates/w2m_vm/tests/exit_ledger.rs`, `crates/signal_report/src/main.rs`,
  `store/exit_ledger/`, `store/frontier-logs/`, `WORKING_AGREEMENT.md`, `docs/step-4-census-2026-10-28.md`

### Command exit codes

- `exit_ledger.rs`, `batch_run.rs`, `family_runs.rs`, `signal_report --check` and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the site and result columns agree with the frozen host's at every row of both processes to
  the game's first MAP_VIEW; the held file names no row but the child's stop row and the varying cells; the written
  column covers the loader's and the arena's writes too, and agrees.
- **Planted faults:** (1) two thunks made in swapped order: the judge names every row whose site moves, not the first
  only; (2) one write left out of the log (the init stub's page): the judge names the written row; (3) a Decision
  given a second revisit clause: `--check` names it.

## done criteria

The exit ledger judges every row and every guest write, and the Rust host's thunks are the frozen host's.

## self-review checklist

- [x] Scope names one verifiable property: every ledger row agreeing, named where it does not.
- [x] context_files cite the Python to read first.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the exit ledger whole.

## result

- **The thunks:** the two hosts made the same 1,100 thunks at the start, in a different order: the Rust filled the
  COM vtables (54 thunks) before the traced image's own thunks (490), where the Python's `__init__` makes them after
  (`_trace_ranges` 2480, `_com_vtables` 2488). `Guest::com_vtables` now runs after `trace_ranges`, and the thunk
  tables are equal address for address. Past mono's load the site column parted again by 11 thunks: `import_thunk`
  matched any thunk of the name, the trace's own and GetProcAddress's among them; `Guest::imports` and
  `Guest::proc_thunks` now keep them apart as the Python's `Modules.imports` and `Modules._thunks` do. The code
  cache follows the Python's `__init__` order too: the trace's own bridges, the COM vtables', then the slice and entry
  stubs (the first gates run found the slice stub placed before the COM bridges, a BL offset in qpc's host-compiled
  block moved).
- **The judge:** `exit_ledger.rs` holds every differing (process, row, column) and names a row one side lacks by its
  kind; `store/exit_ledger/differences-2026-10-29.tsv` (renamed by `git mv` from first-difference-2026-10-28.tsv)
  holds one line, the child's row 190, `hvc 12`, which the fixture writes when it stops the run.
- **The writes:** a fourteenth column holds the sha256 of the process area below the trace (the PEB, the loader
  lists, the TEBs, the system images, the transport) at each exit, on both hosts, the writes no wrap reaches; at the
  stop the two process areas differed in 24 bytes, all in the trace's records 426 and 438, the stream's varying cells.
- **The run:** every cell of every exit agrees to the game's first MAP_VIEW, 1,024 exits of the game and 190 of the
  child, the process area included; the stream, the guests and the families as before.
- **The logs and the Decisions:** the three repeats' run logs are tracked under store/frontier-logs/<run id>/run.log
  and census rows 4 and 5a cite them; `signal_report --check` holds each agent-resolved Decision from 2026-10-25 to
  one decision clause and one revisit clause.
- **Plant 1** (the COM vtables made before the trace's own thunks again): the judge failed with 73 lines, every row
  whose site moved, beginning "main 0 site 7ffe00003100 7ffe00003460 hvc 1". Restored byte for byte.
- **Plant 2** (the loader list not written again after a late load): the judge failed with 668 lines, the process
  column from mono's load, "main 356 process 1b3b0956… 19a8f14e… hvc 8". Restored byte for byte.
- **Plant 3** (a Decision given a second revisit clause): `--check` failed, "the Decision of 2026-10-28 \"Step 4's
  reference run (reviewer-2's follow-up on Sprint 207\" holds 1 decision clauses and 2 revisit clauses, not one
  each". Restored byte for byte.
- **Departures:** every guest write is not logged: the written column stays the request view's, and the process
  column covers the process area below the trace; the arena's writes (a mapped DLL, VirtualQuery's record on the
  guest stack) are in neither, which the doc says. Plant 2 planted the loader list, not the init stub's page the card
  named, since the init stub's page is in the arena.
