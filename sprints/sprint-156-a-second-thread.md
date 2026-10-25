# Sprint 156 — A second thread: CreateThread served by the Rust host, and `sleeper` past its thread request

```yaml
---
id: 156
status: open
phase: 6
pass_kind: functional
---
```

## scope

Link 5's run-loop judge, its second program (`docs/design-host-compile-path-2026-10-24.md` F.5: "each lets `qpc`, then
`sleeper`, run further"). `tests/data/timed_waits/sleeper.exe` in the Rust host's run loop (Sprint 155) stops at
`hvc #8` op 30 (HOST_THREAD), sub-request 1: THREAD_CREATE, its `CreateThread` (measured: `HostRequest { op: 30 }`, no
block compiled by the host, exits `{1: 1, 8: 1}`). The Rust host now serves it as the Python host's `_create_thread`
does, and the run, with two threads, takes time slices and lets the VM's own switch (threads.c `w2m_switch`) move
between them, as the Python's does with one vCPU; it goes on to its next stop, which it names.

Carried from `src/w2m/vm_backend.py` and `src/w2m/scheduler.py` (read in full: vm_backend 3017-3060, 3062-3160 (`_code_page`, `_import_thunk`,
`_create_thread` 3094-3150, `_map_pages` 3152-3159), 3190-3360, 3500-3580, 2756-2781, 5129-5141; scheduler.py 1-200):

- `_create_thread`: the thread id, the stack (reserve, the initial commit at the top, three PAGE_GUARD pages below
  StackLimit, the guard and reserved pages unmapped, `_map_pages`), the TEB and its nine words, the static TLS
  (`_thread_static_tls`), the attach stub (`attach_stub`, `_thread_callbacks`, ExitThread's thunk), the first registers,
  the per-thread monitor words, the state block, the vector slot (`_write_vectors`), the stack in the arena's list,
  `_set_budget`;
- `_add_thread` for any thread (the run table entry after the others, `GuestThread`'s fields), generalizing Sprint 142's
  `add_first_thread`;
- `_page_descriptor`'s split-table cursor moved from the host compiler to the guest, which `_map_pages` and
  `_protect_code` share, as the Python's `VM` holds one `_next_table`;
- `_before_run`'s SLICE_UNTIL by `_slice_until` for one vCPU and one process: 0 when another thread is ready, the
  deadline's count on the instruction clock, else ~0; `_set_budget` (the run's limit, or a time slice with two threads).

The run stops by name at a thread request, a host request or an exit the Rust host does not serve.

## prerequisites

- 155 (closed)

## context_files

- `src/w2m/vm_backend.py`, `src/w2m/scheduler.py` as above; `crates/w2m_vm/src/run.rs`, `host_request.rs`, `guest.rs`,
  `compile.rs`; `crates/virtual_memory`; `crates/w2m_vm/tests/run_loop.rs`

## signal contract

### Emits

None this sprint: the skeleton's record comes at its gate (link 6), as the design orders; the run's stop is the test's
output.

### Consumes

`tests/data/timed_waits/sleeper.exe`; the frozen Python host's run of `sleeper` with one vCPU (a fixture made by running
it, its sha256 committed): the created thread's id, TEB words, stack, arena pages, static TLS, attach stub and first run
table entry at THREAD_CREATE's return, and the run's compiles.

### Invariants

- No Python file added or changed.
- `qpc`'s judge (Sprint 155) passes unchanged.

## artifact contract

### Files created

- `crates/w2m_vm/src/threads.rs` (thread creation, the run table entry, the slice), the fixture's sha256 under
  `store/run_loop/`

### Files modified

- `crates/w2m_vm/src/host_request.rs`, `run.rs`, `guest.rs`, `compile.rs`, `crates/virtual_memory/src/lib.rs`,
  `crates/w2m_vm/tests/run_loop.rs`, `WORKING_AGREEMENT.md` (the fixture's generator), `BLACKBOARD.md` (Sprint 155's CI
  run, in the tail entry)

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values, from the Python host:** at THREAD_CREATE's return, the Rust host's created thread equals the
  Python's: the thread id, the TEB's nine words, the stack's (deallocation base, StackLimit, StackBase), the arena's page
  states and protections over the stack and the TEB, the static TLS bytes, the attach stub's bytes, and the run table
  entry (registers, monitor words, state block, vector slot). The blocks the host compiles up to the Rust run's stop
  are, in order, the Python host's first compiles. The stop is named; if the Python run goes past it, the stop names
  what the Rust host does not yet serve.
- **Planted fault:** the stack's guard pages left mapped, or the first registers' RSP off by 8: the comparison with the
  Python's names the field.

## done criteria

The Rust host serves CreateThread as the Python host does, field for field, and `sleeper` runs past its thread request
with two threads to a named stop.

## self-review checklist

- [x] Scope names one verifiable property: CreateThread served as the Python serves it, and `sleeper` runs further.
- [x] context_files cite the Python carried.
- [x] Emits: none this sprint, with the design's reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the Python host's created thread as reference, and a planted fault.
- [x] One concept: a second thread in the Rust host.

## result

- **`sleeper`'s second thread is the Python host's, field for field.** At THREAD_CREATE's return the Rust host's thread
  8 (parent 4, start 0x140001000) equals the frozen Python host's: the TEB at 0x1000200000 and its nine words, the stack
  (0x1000000000, 0x10001ff000, 0x1000200000), the arena's four reservations (the 2 MB stack with one committed page and
  three PAGE_GUARD pages, the TEB, the static TLS array, the attach stub's page) page by page, the static TLS and stub
  bytes, the run table entry (all 0x600 bytes: ids, registers, vCPU mark, monitor words, state block), the vector slot,
  and the count and limit (12 and 20012: a time slice from then on). The stack's reserved and guard pages are unmapped
  in the stage-1 tables and its committed page mapped, as `_map_pages` leaves them.
- **`sleeper` runs on with two threads.** The VM's own switch moves between the main thread and the spinner; the spinner
  counts, the main thread sleeps 20 ms and wakes, and the run prints `woke\n` (the Python run's output begins so). It
  stops at `hvc #8` op 30, thread request 7 (THREAD_EXIT_CODE: `WaitForSingleObject` on the spinner's handle), which
  the Rust host does not serve yet; the stop now carries the host's refusal (`Stop::HostRequest`'s `why`). The C
  compiled eight blocks and the Rust host five, among them the attach stub's (0x1000220000-0x1000220026); each start is
  a Python host compile's, and each host compile's (start, end) is the Python's.
- `crates/w2m_vm/src/threads.rs`: `add_thread` (any thread; `run.rs`'s first-thread code folded in), `current_tid`,
  `thread_callbacks`, `thread_static_tls`, `attach_stub`, `create_thread`, `set_budget`, `slice_until`.
  `_page_descriptor` moved from `HostCompiler` to `Guest` with its split-table cursor, beside `map_pages`; `Guest`
  keeps the threads and the loader's images; `ProcessArea` keeps the process id; `virtual_memory`'s `commit` is public;
  THREAD_STACK_GROW now looks at the current thread's stack, not the main thread's.
- **Planted faults.** The stack's reserved and guard pages left mapped (`map_pages(..., true)`): the judge failed at
  "the stack page 0x1000000000's mapping". RSP at `top - 0x40`: it failed at "the run table entry's registers".
  Restored, it passes.
- The fixture: `build/run_loop/sleeper-python.json`, its sha256 in `store/run_loop/sleeper-python.sha256`, its
  generator in WORKING_AGREEMENT; two runs gave the same bytes. The Python run with one vCPU ends at ExitProcess with
  `woke\ndone\n` after 19 compiles.
- Rule breach, recorded: one two-line insertion into `crates/w2m_vm/src/process.rs` was made with perl, not the Edit
  tool; its diff was read and holds only the two lines.
- Also found: Sprint 155 added no WORKING_AGREEMENT row for `compile.rs`; it has one now.
