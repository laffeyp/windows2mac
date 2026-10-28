# Sprint 193 — The shared objects in one process

```yaml
---
id: 193
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 190": one card per op family in the order the call stream meets them. Sprint
191's stream stops at row 4, UnityPlayer's CreateEventW, KERNEL 33's EVENT_OPEN. The family is `src/w2m/kernel.py`'s
shared objects (rows 4-6 events, 7 CreatePipe, 16 CreateProcessW, 23 WaitForSingleObject on an event, 25-27 WriteFile
into the pipe). The plan puts the crash handler's process in step 4 (its gate is the batch run, which always starts
UnityCrashHandler64.exe; step 6 brings a process per guest process), but a second VM is a component of its own: this
card serves the objects one process uses, and the second process is the next card's.

- **The objects:** `kernel.py`'s `_request` for one process in Rust (`crates/w2m_vm/src/kernel.rs`): EVENT_OPEN (a
  name read as `read_text` reads it, an existing name's id and 1), EVENT_STATE, TAKE, SET, RESET, CLOSE (a handle the
  process holds, or a stop by name), REF, PIPE_CREATE (the read end's id, the write end's second), PIPE_READ (0 data, 1
  empty, 2 broken), PIPE_WRITE (what fits in PIPE_CAPACITY 4,096; a write no process reads is a stop), PROCESS_STATE
  (the process's own id STILL_ACTIVE, any other ~0); ids from OBJECT_IDS (1 << 32); `wake` as `wake_all`, every blocked
  thread ready. PROCESS_CREATE stops by name ("a second process: the next card's").
- **The run:** the batch run to its next stop, the stream held up to it.

## prerequisites

- 192 (closed)

## context_files

- `src/w2m/kernel.py` (whole), `src/w2m/vm_backend.py` 3700-3706 (`wake_all`), 3819-3831 (the dispatch),
  `tools/guest_runtime/sync_objects.c` 115-150, 280-360, `tools/guest_runtime/processes.c` 55-200,
  `crates/w2m_vm/src/host_request.rs`, `crates/w2m_vm/src/threads.rs` (`set_status`)

## signal contract

### Emits

- None: no producer runs; the judges are `batch_run.rs` and unit tests.

### Invariants

- No Python file added or changed; the stream's rows held so far held still; the guests' stops as before or moved
  forward, each move named.

## artifact contract

### Files modified

- `crates/w2m_vm/src/kernel.rs` (new), `crates/w2m_vm/src/guest.rs` (the objects' table), `host_request.rs`, `lib.rs`,
  `store/batch_run/first-stop-2026-10-28.tsv`, `docs/step-4-census-2026-10-28.md`, `WORKING_AGREEMENT.md`

### Command exit codes

- `batch_run.rs`, the kernel unit tests and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the Rust stream equals the Python's through row 15 and stops at row 16's CreateProcessW by
  name, or earlier at a row named; the guests that stopped at KERNEL 33 move to their next stop, each named.
- **Planted faults:** (1) EVENT_OPEN answering 0 for an existing name: the event unit test fails; (2) PIPE_WRITE taking
  all n bytes past the capacity: the pipe unit test names the count; (3) EVENT_OPEN's id off by one: the stream
  comparison names the row whose return differs.

## done criteria

The Rust host answers KERNEL 33 for one process as `kernel.py` does, and the batch run's stream conforms to the row
before its second process.

## self-review checklist

- [x] Scope names one verifiable property: the stream conforming to the row before the second process.
- [x] context_files cite the Python to read first.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the shared objects in one process.
