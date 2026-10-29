# Sprint 215 — A thread suspended, read and resumed

```yaml
---
id: 215
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 214": the batch run stops at row 3131, inside mono's SuspendThread on thread 4, on
HOST_THREAD's request 4, "not served by the Rust host yet". mono's collector suspends a thread, reads its context and
resumes it; two guests, `mutual` and `mutual_context`, already stop at request 5. The frozen host serves the three in
`_thread_request` (vm_backend 3212-3260): THREAD_SUSPEND counts the suspension and answers the previous count (M64 for a
thread that has ended); THREAD_RESUME answers the count before it falls (0 when it is 0 or the thread has ended);
THREAD_CONTEXT writes the thread's x86 context into the caller's CONTEXT (1232 bytes, the flags at 48), its EFLAGS
rebuilt from the monitor's lazy flags. The card reads that code against Windows's rows first
(contracts/windows.thread.running_context, windows.thread.threads) and follows it where they agree (the user's rule of
2026-10-29); the frozen host's waits for another vCPU (`_stop_elsewhere`, `_held_elsewhere`) do not arise on the Rust
host's one vCPU and are named as such.

- **The requests:** THREAD_SUSPEND, THREAD_RESUME and THREAD_CONTEXT served in `host_request.rs` through `threads.rs`.
- **The run:** the stream equals the reference past row 3130 to its next stop; the comparison against Windows over the
  rows reached; the guests that stop at request 5 run on, held to the frozen host's output.

## prerequisites

- 214 (closed)

## context_files

- `src/w2m/vm_backend.py` 3196-3280 (`_thread_request`), 2748-2782 (the lazy flags); `contracts/windows.thread.running_context`,
  `contracts/windows.thread.threads`; `crates/w2m_vm/src/{threads.rs, host_request.rs}`; `tests/data/timed_waits/mutual*.c`

## signal contract

### Emits

- The batch run's record carries the new stop and the comparison; no new emit site.

### Invariants

- No Python file changed; the stream and the comparison as before over the rows reached; the exit ledger's held file
  unchanged.

## artifact contract

### Files modified

- `crates/w2m_vm/src/{threads.rs, host_request.rs}`, their tests, `store/batch_run/first-stop-2026-10-28.tsv`,
  `docs/step-4-census-2026-10-28.md` (rows 5a, 8)

### Command exit codes

- the requests' unit tests, `batch_run.rs`, `batch_record.rs`, `call_compare.rs`, `exit_ledger.rs` and `scripts/gates.sh`
  return 0

## observation contract

- **Expected values:** a unit test: a suspend answers 0 then 1, a resume 2 then 1 then 0, an ended thread M64 and 0; a
  context read of a suspended thread gives its RIP, RSP and EFLAGS as the contract's rows say; the stream past row 3130
  to its next stop; `mutual` and `mutual_context` end with the frozen host's output, or stop past request 5, named.
- **Planted faults:** (1) the suspend answering the new count: the unit test names it; (2) EFLAGS without the lazy flags:
  the unit test names the bit; (3) the requests unserved again: `batch_run.rs` names the held stop.

## done criteria

A thread is suspended, read and resumed as Windows's rows say, and the batch run's stream passes row 3130.

## self-review checklist

- [x] Scope names one verifiable property: the stream past row 3130, by the rows.
- [x] context_files cite the Python and the Windows rows.
- [x] Emits: the record's stop moves.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: one suspension's three requests, which mono makes together.
