# Sprint 197 — The second process

```yaml
---
id: 197
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 196" (reviewer-2's Highs 1 and Mediums 1 of its review of Sprints 193-196):
kernel.Machine in Rust. The batch run stops at row 16, UnityPlayer's CreateProcessW of UnityCrashHandler64.exe, and six
guests stop at their PROCESS_CREATE (busychild, crosswake, handshake, pingpong, spinchild, twoproc). Hypervisor.framework
allows one VM per process (kernel.py 5-6), so the child is a second `w2m-vm` process; the processes take turns, one at a
time, on one machine clock, as the Python's do. No child that never runs is faked.

- **The tables:** `kernel::Objects` for every process (`Machine`'s): a record per process, PROCESS_CREATE's ids and
  inherited handles (`allocate_ids`, `_ref`), PROCESS_STATE, and a process's end closing what it held (`_release`).
- **The child:** the supervisor (the first `w2m-vm`) starts `w2m-vm --child` for a PROCESS_CREATE, its start as
  `spawner` makes it (the image by the command line's first token in C:\w2m\app, the parent's environment and directory
  and process group); the child's guest memory is a file both map shared (vm_monitor.c 328), so the supervisor serves
  the child's KERNEL 33 requests against it directly.
- **The turns:** a request word in a shared page, waited on with `os_sync_wait_on_address` and
  OS_SYNC_WAIT_ON_ADDRESS_SHARED (docs/design-rust-host-2026-10-24.md Change 4): the supervisor gives one process its
  turn; a process runs until it is idle, or its slice ends while another has a ready thread (`Machine.run`, `_next`), and
  hands the turn back with its state (ready, timed wait, idle, ended with its code, stopped with its stop).
- **The clock:** on the instruction clock the machine's time is the latest process's (`_machine_now`); a process that
  runs again is caught up (`_catch_up`); with no process ready the earliest deadline over the machine is warped to
  (`warp_to_deadline`, `_next`'s timed waiter). With none, the run stops as a deadlock (`_deadlocked`); a child's stop
  ends the run as "child process N: ..." (`_child_stopped`).
- **The run:** twoproc first, then the five other guests and the batch run; each stop moved is named, and the stream
  judged past row 16.

## prerequisites

- 196 (closed)

## context_files

- `src/w2m/kernel.py` (whole), `src/w2m/vm_backend.py` 2393-2395, 3612-3700, `tools/vm_monitor/vm_monitor.c` 328,
  `docs/design-rust-host-2026-10-24.md` 212-232, `crates/w2m_vm/src/kernel.rs`, `host_request.rs`, `run.rs`,
  `src/bin/w2m-vm.rs`, `tests/data/timed_waits/{twoproc,handshake,pingpong}.c`, `tools/guest_runtime/processes.c` 55-200

## signal contract

### Emits

- None: the judges are `batch_run.rs` and unit tests.

### Invariants

- No Python file added or changed; the stream's rows 0-15 still equal; the guests that end still print the Python's
  output; every child `w2m-vm` exits when the supervisor does.

## artifact contract

### Files modified

- `crates/w2m_vm/src/kernel.rs`, `crates/w2m_vm/src/machine.rs` (new), `crates/w2m_vm/src/host_request.rs`,
  `crates/w2m_vm/src/run.rs`, `crates/w2m_vm/src/bin/w2m-vm.rs`, `crates/w2m_vm/src/guest.rs`,
  `store/batch_run/first-stop-2026-10-28.tsv`, `docs/step-4-census-2026-10-28.md`, `WORKING_AGREEMENT.md`

### Command exit codes

- `batch_run.rs`, the kernel and machine unit tests and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** `twoproc` ends and prints "child ended"; each of the other five ends with the Python's output or
  stops at a later request, named; the batch run passes row 16 and its stream equals the Python's to its next stop; the
  turn's round trip measured and stated.
- **Planted faults:** (1) no catch-up when a process runs again: `twoproc` prints "clock behind", named by the guest
  test; (2) the child's exit code not kept at its end: `twoproc` prints "child bad code"; (3) Sprint 193's plant again,
  a new event's id off by one, against a guest whose child uses an inherited event (reviewer-2's Low 1).

## done criteria

A guest's CreateProcessW starts a second `w2m-vm` that runs the child in turns with its parent on one machine clock,
the two share the kernel objects as `kernel.py`'s Machine shares them, and the batch run passes row 16.

## self-review checklist

- [x] Scope names one verifiable property: a child process runs in turns on the machine's clock and shares its objects.
- [x] context_files cite the Python to read first.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: kernel.Machine.
