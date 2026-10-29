# Sprint 213 — A created thread's stack grown

```yaml
---
id: 213
status: done
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 211": the batch run stops at row 1355, between calls, on HOST_THREAD's
THREAD_STACK_GROW (9) for thread 8, a created thread, address 0x1180dce5f0, probe 1: "a created thread's growth, not
carried yet" (host_request.rs, `HostError::NotServed`). The frozen host serves it in `_stack_grow` (vm_backend
3161-3188), which follows Windows's rows (contracts/windows.memory.created_thread_stack 0.1.0, default.grow and
default.grown.*): the touched guard page and those above it committed read-write, StackLimit (TEB+0x10) moved to it,
three guard pages below (PAGE_READWRITE | PAGE_GUARD); a probe grows page by page; 0, an access violation, when the
address is outside the reservation's guard reach or the guard would reach the reservation's last page. The card reads
that code against the contract's rows before porting it (the user's rule of 2026-10-29), and sweeps threads.rs for
copied defects (the sweep's uncovered files).

- **The request:** `THREAD_STACK_GROW` for a created thread serves as the contract's rows say, through
  `Arena::commit` and the thread's `teb` and `stack`; the frozen host's code followed where it agrees with the rows,
  a departure named where it does not.
- **The run:** the stream equals the reference past row 1354 to its next stop, held in the stop file.
- **The sweep:** threads.rs read against Windows's rows and the frozen host; each copied defect named in Surfaced.

## prerequisites

- 212 (closed)

## context_files

- `src/w2m/vm_backend.py` 3161-3196; `contracts/windows.memory.created_thread_stack/0.1.0.json`;
  `crates/virtual_memory/src/lib.rs` (`commit`, `query`'s stack rows 570-590); `crates/w2m_vm/src/threads.rs`,
  `crates/w2m_vm/src/host_request.rs` (THREAD_STACK_GROW)

## signal contract

### Emits

- The batch run's record carries the new stop; no new emit site.

### Invariants

- No Python file changed; the stream equals the reference through every row reached; the exit ledger's held file
  unchanged.

## artifact contract

### Files modified

- `crates/w2m_vm/src/host_request.rs`, `crates/w2m_vm/src/threads.rs`, `store/batch_run/first-stop-2026-10-28.tsv`,
  `docs/step-4-census-2026-10-28.md` (row 5a)

### Command exit codes

- the growth's unit test, `batch_run.rs`, `batch_record.rs`, `exit_ledger.rs` and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** a unit test on a created thread's stack: a touch of its first guard page commits it read-write,
  moves StackLimit to it and guards the three below, and VirtualQuery reads the contract's rows; a probe two pages
  below grows two pages; an address below the guard reach answers 0; the stream past row 1354 to its next stop.
- **Planted faults:** (1) StackLimit left unmoved: the unit test names TEB+0x10; (2) two guard pages: the query's row
  names the protection; (3) the probe treated as a touch: the unit test names the 0; (4) the request unserved again:
  `batch_run.rs` names the held stop.

## done criteria

A created thread's stack grows as Windows's rows say, and the batch run's stream passes row 1354.

## self-review checklist

- [x] Scope names one verifiable property: the stream past row 1354, by the contract's rows.
- [x] context_files cite the Python and the Windows rows.
- [x] Emits: the record's stop moves.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and four plants.
- [x] One concept: one thread request served (the sweep of its file is the user's rule's check, not a second change).

## result

- **The request:** `threads::stack_grow` serves THREAD_STACK_GROW for a created thread as the contract's rows say: a
  page in the guard commits the pages from it to StackLimit read-write and maps them, StackLimit (TEB+0x10) and the
  thread's record move to it, three PAGE_GUARD pages go below; a probe grows page by page; a touch below the guard, an
  address outside the reservation, or a growth that would bring the guard to the reservation's last page answers 0. The
  frozen host's `_stack_grow` checks the last bound inside its loop, so a probe reaching it commits pages and returns 0
  with StackLimit unmoved; here the bound is checked first and nothing changes. The contract's rows are one page per
  touch; a touch that skips a guard page has no row, and the frozen host's rule (every page from it to StackLimit) is
  followed.
- **The run:** the stream equals the reference rows 0-3130, 1,776 rows past Sprint 211's 1,354, with row 3131 in
  flight, mono's SuspendThread on thread 4, and stops at THREAD 30's request 4, "not served by the Rust host yet";
  exits hvc 1 409, hvc 4 10, hvc 8 1732, hvc 10 256, hvc 11 11, hvc 13 194. The record names
  KERNEL32.dll!SuspendThread, the stopping thread's call in flight.
- **The ledger:** the child, UnityCrashHandler64, now runs past its row 190 too, and its row 190 equals the fixture's
  stop row in every held column: the held file's one line, "4100 190 missing hvc 12", is gone, and the two hosts
  agree through the first MAP_VIEW in both processes. Sprint 211's rule had held a child's rows past the fixture as
  extra, taking the fixture's last row for the child's own stop; the fixture's run stops every process at the game's
  MAP_VIEW, so no process's rows past it are held now.
- **Plant 1** (StackLimit left unmoved): the unit test failed, "default.grow moved_1". Restored byte for byte.
- **Plant 2** (two guard pages): "default.grown.limit-3p, left: (8192, 0)". Restored byte for byte.
- **Plant 3** (a probe treated as a touch): "left: Ok(0)" at the five-page probe. Restored byte for byte.
- **Plant 4** (the request unserved again): `batch_run.rs` failed, "the batch run's first stop or a guest's stop moved".
  Restored byte for byte.
- **Plant 5** (the last-page bound removed): the arena's commit panicked on the reservation's edge
  (virtual_memory/src/lib.rs 280). Restored byte for byte.
- **Departures:** the card held the exit ledger's file unchanged; its one line went when the difference did, and the
  child's rule in `exit_ledger.rs` changed with it. The card's files did not name `tests/exit_ledger.rs` or
  `store/exit_ledger/differences-2026-10-29.tsv`. The first unit test's probe lay inside the guard, where plant 3
  could not show; a probe five pages below was added before the plants ran.
