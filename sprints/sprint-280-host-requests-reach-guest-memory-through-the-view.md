# Sprint 280 — Host requests reach guest memory through the view

```yaml
---
id: 280
status: done
phase: 6
pass_kind: functional
---
```

## scope

Census row D7's second module (the step 6 sequence Decision, annotated at 278). `crates/w2m_vm/src/host_request.rs`
serves the guest's host requests (`hvc #8`): it reads the request from the transport page, copies the buffers the
request names (a file read's destination, a path, a kernel request's structures, through `KernelMemory`, 236-262), and
writes the answer back. It holds 22 slice calls, 20 of them outside its tests (before line 668); census D7 and
`tests/slice_calls.rs` hold it at 22.

- **The rows of the design's table it cites** (`docs/design-guest-memory-on-several-vcpus-2026-10-31.md`, "The
  structures the host reaches"): the transport page (copy; the requesting vCPU holds the runtime word across
  `hvc #8`); a guest buffer a host request names (copy; no ordering, a racing guest thread gets torn bytes, as on
  Windows); the run table's thread entries (copy; the runtime word); a vCPU's own areas (copy; its vCPU is stopped at
  the exit served).
- **The work:**
  - `host_request.rs`'s helpers and its 20 calls outside its tests go through `Guest::memory_view` or
    `guest::memory_view(&guest.memory)`; `KernelMemory`'s `read` and `write` copy through the view.
  - `tests/slice_calls.rs` and census D7: `host_request.rs` at its tests' count.
- **Not here:** `file_bridge` and `kernel`, which take what `host_request.rs` hands them; the other modules.

## prerequisites

- 279 (closed)

## context_files

- `crates/w2m_vm/src/host_request.rs`; `crates/w2m_vm/src/guest.rs` (`memory_view`)
- `docs/design-guest-memory-on-several-vcpus-2026-10-31.md` (the table); `docs/step-6-census-2026-10-30.md` (D7)
- `crates/w2m_vm/tests/slice_calls.rs`

## signal contract

### Emits

- None; an access path, held by tests and the one-vCPU runs.

### Invariants

- One-vCPU runs unchanged: the batch run's stop and stream, the 68 families, the bridges' and file tests.

## artifact contract

### Files modified

- `crates/w2m_vm/src/host_request.rs`, `crates/w2m_vm/tests/slice_calls.rs`, `docs/step-6-census-2026-10-30.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `host_request.rs` 0 slice calls outside its tests; the run-loop, family, bridge and batch-run
  tests pass as before.
- **Planted faults:**
  - (1) `KernelMemory::write` copying to one byte past its address: a host-request test names the bytes.
  - (2) one `as_slice()` added to `host_request.rs` outside its tests: the count test names the file and its count.

## done criteria

The host's request service reaches guest memory only through the view, and the count holds it there.

## self-review checklist

- [x] Scope names one verifiable property: the request service's guest memory access through the view.
- [x] context_files cite the module, the accessor, the table, the census and the count test.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: one module's access moved.

## result

- **The access** (e91524683): `host_request.rs`'s `read_u64` and a new `bytes` take the view; `KernelMemory`'s
  `read` and `write` copy through `Guest::memory_view`; the transport page's answers by `write` at `TRANSPORT + 48`
  and `+ 56` (seven sites); the copies into guest buffers by `write` (four, the XMM capture's 256 bytes among them,
  16 registers of 16 bytes, so the length the old slice checked holds); the copies out by `bytes` (five, the
  environment block among them).
- **The count:** `host_request.rs` 2, both in its tests; 106 slice calls in `crates/w2m_vm/src/`, from 126;
  `tests/slice_calls.rs` and census D7 lowered together.
- **Expected values:** the module's 4 tests and the count tests pass; the `w2m_vm` tests before the commit, 156 passed; a local that shadowed `bytes` renamed `written` after them, with the lint, the Linux check, the module's and the bridge tests run again.
- **The gates:** `scripts/gates.sh` exit 0 in 671 s, kept in `store/gates/passes.tsv` as sprint 280 at e91524683 with root=bdff9b308 and "rust=crates python=0 samples=0", rdeps(w2m_vm), 156 tests, base=fe8a9942c, Sprint 279's close; multi-vCPU: not run (Python gates not run)
- **Plant 1,** restored by `cp` and checked by `cmp`: `KernelMemory::write` writing one byte past its address; the
  file-view test failed, "a file view of descriptor 18446744073709551614, which is not open".
- **Plant 2,** restored by `cp` and checked by `cmp`: one `as_slice()` added outside the tests; the count test failed,
  "host_request.rs: 3 slice calls, census D7 allows 2".
