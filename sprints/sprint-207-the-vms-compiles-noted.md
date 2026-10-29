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

## result

- **The note:** `run.rs`'s `vm_compiled` reads each log entry's marks and notes their tails, the block, its pages'
  owners and its translation in the host compiler. The ledger's count then agrees at the first model step and on: to
  the game's first MAP_VIEW (1,024 exits) and the child's 190, count and warp agree at every exit, and both hosts
  compile the same 19,224 blocks (`<ledger>.blocks`, the frozen host's `vm.translations`).
- **The ledger's read point:** the Rust writes a row as the next exit arrives, before its count refresh, where the
  fixture reads; read before the resume it had lagged the count a thunk's bridge stores on entering a service (the
  game's row 9: 338 against the frozen host's 407). The fixture now runs to the game's first MAP_VIEW and ends by an
  exception out of `machine.run`, not `os._exit`, which had left the VMs' monitor processes running.
- **The reference:** with the clock agreeing, row 924 still differed; the frozen host run now gives mono's cookie
  0x165d8e8cc734, the Rust host's, where the stored streams (2026-10-17) hold 0x13fc8e8cff34; they predate Sprint
  120's catch-up of a child at its start (2026-10-18). `batch_run.rs`'s stream is the frozen host's recorded run of
  2026-10-21 (store/runs/frontier-643a6283510c-1792623423346201000, its comparison with no divergence), and its
  repeats three runs of 2026-10-28, each 10,505 rows and equal to it but in the varying cells, which are the stored
  file's, rows 426 and 438, columns 9-12 (reviewer-2's follow-up; the Decision "Step 4's reference run").
- **Sprint 205's code:** `_load_dll` and the init stub landed (`the_init_stub_is_the_pythons` holds the bytes of
  vm_backend 4002-4021). The stream equals the frozen host's through row 924 and stops at row 925, MAP_VIEW 29; all 12
  module-family binaries conform, windows.module.search now row for row.
- **The run:** the ledger's held differences are the thunks' numbering only: the game's row 0 dispatch target and row
  702's answer, a thunk address (0x7ffe00006670 against 0x7ffe000065c0).
- **Plant 1** (the VM blocks' tails unnoted): the ledger judge failed, "main executed 4 187 194 hvc 11", the defect
  this sprint fixed. Restored byte for byte.
- **Plant 2** (the marks read one entry late): the ledger judge failed, "main executed 4 187 186 hvc 11". Restored byte
  for byte.
- **Plant 3** (card 205's): an import bound to the thunk though a loaded DLL exports it reached no judge, no DLL loaded
  after the start importing from the program's DLLs; its replacement, every thunk import bound to KERNEL32!Sleep,
  stopped the run, "stop\tHvc 9". DllMain's reason 0 failed the stub's bytes test, "left: [72, 131, 236, 32, 72, 185,
  0, 0, 1, 128, 17, 0, 0, 0, 186, 0, 0, 0, 0". The trace left open around a traced DllMain failed the stream at row
  501, "501 ... mono-2.0-bdwgc.dll ... GetSystemTimeAsFileTime", mono's CRT reading the time for its cookie. Each
  restored byte for byte.
- **Departures:** the card's scope grew by the ledger's read point and the reference recorded again, both found by
  this sprint's measurement; the thunks' numbering is held, not fixed; the fixture's two runs vary in five cells of
  the game (FILE_STAT's and FILE_OPEN's written times and FILE_OPEN's inode), taken as varying.
