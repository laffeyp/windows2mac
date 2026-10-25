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
