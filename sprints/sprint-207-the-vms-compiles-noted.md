# Sprint 207 — The VM's compiles noted

```yaml
---
id: 207
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 205": the clock where the exit ledger names it. Sprint 206's ledger holds the
instruction count parting at each process's first `hvc 11` model step, the Rust host 7 more (the game's row 4, 187
against 194; the child's row 4, 129 against 136). The count's give-back at a model step adds the block's unrun
instructions (`_unrun_after`, from the tail notes); the Python's `_vm_compiled` (vm_backend 2559-2580) notes the tails,
pc map, addresses and code pages of each block block_compiler.c compiled, and the Rust's `vm_compiled` (run.rs
353-362) notes none of them, so a model step in a block the VM compiled gives nothing back.

- **The note:** `run.rs`'s `vm_compiled` reads each log entry whole (start, end, code, first, nmarks, count, cut,
  live_out, live_used, npages, the pages) and notes the marks' tails, the addresses and the pages' owners as the
  host's compiles note theirs.
- **The ledger:** the game's and the child's `executed` columns agree with the frozen host's past row 4; the held first
  differences move, each named. The Rust writes a row as the next exit arrives, before its count refresh, where the
  fixture reads (`serve_exit`'s entry): read before the resume, the word lags the count a bridge stores on entering a
  service.
- **The reference:** the batch run's stored streams (2026-10-17) predate Sprint 120's child catch-up (2026-10-18);
  the frozen host run now gives mono's cookie 0x165d8e8cc734, where they hold 0x13fc8e8cff34. `batch_run.rs`'s
  STREAM and REPEATS become three runs of the current frozen host (`run_frontier.py --scratch --max-steps 10^12`),
  copied under `store/frontier-calls/`, and the varying cells are derived from them again.
- **Sprint 205's code:** `_load_dll` and the DllMain init stub land from branch sprint-205-wip (Deferred "Sprint 205,
  partial"), with the init stub's bytes held to vm_backend 4002-4021 and card 205's three plants; the stream runs past
  row 924.

## prerequisites

- 206 (closed)

## context_files

- `src/w2m/vm_backend.py` 2559-2580 (`_vm_compiled`), `_note_tails`, `_unrun_after`, the log entry's layout
  (block_compiler.c `log_entry`); `crates/w2m_vm/src/run.rs` (`vm_compiled`, `exit_count`, `Tails`);
  `crates/w2m_vm/src/compile.rs` (the host's own notes)

## signal contract

### Emits

- None: the judges are `exit_ledger.rs`, `batch_run.rs` and unit tests.

### Invariants

- No Python file added or changed; every request, answer and written range in the ledger still agrees.

## artifact contract

### Files modified

- `crates/w2m_vm/src/run.rs`, `store/exit_ledger/first-difference-2026-10-28.tsv`,
  `docs/step-4-census-2026-10-28.md`

### Command exit codes

- `exit_ledger.rs`, `batch_run.rs` and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** `executed` agrees with the frozen host's at the game's and the child's row 4 and on, to the
  next difference, named.
- **Planted faults:** (1) the tails of the VM's blocks left unnoted: the ledger names row 4's count; (2) the marks
  read from the wrong entry (`first + k + 1`): the ledger names a count; (3) card 205's plants: an import bound to the
  thunk though a loaded DLL exports it, DllMain's reason 0 for 1 (the stub's bytes test names the byte), and the
  trace left open around a traced DllMain (the stream names the first extra row).

## done criteria

The Rust host notes the VM's compiles as the Python does, and the ledger's instruction counts agree past the first
model step.

## self-review checklist

- [x] Scope names one verifiable property: the counts past row 4.
- [x] context_files cite the Python to read first.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the VM's compiles noted.
