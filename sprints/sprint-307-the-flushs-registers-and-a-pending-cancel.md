# Sprint 307 — The flush's registers and a pending cancel

```yaml
---
id: 307
status: active
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-3's Low 1 at 306: Deferred (Sprint 305) held two tests of `run::flush_through_stub` for card 4's second
thread, and neither needs one. The register test needs a stopped vCPU; the cancel test needs the pending case
`hv_vcpu.h` 377-378 states, a cancel sent before `hv_vcpu_run` making that run return at once, so the nested run meets
it every time.

- **The work:** a test module, on a vCPU over `tests/common/mod.rs`' monitor area (`area`, the monitor code written,
  `FLUSH_STUB` among it): (1) x0 to x30, CPSR and the PC each set to a distinct value (reviewer-3, by message: a shared
  pattern would hide a register restored from the wrong slot), `flush_through_stub` called, every one read back equal,
  `flushes` 1; (2) the same with `hv_vcpus_exit` called on the vCPU's handle first: the flush still runs to its
  `hvc #15`, `flushes` 1, `cancelled` true, every register as set. Both take `ONE_VM`. Deferred (Sprint 305) closes.
- **Not here:** a cancel from another thread mid-split, card 4's.

## prerequisites

- 306 (closed)

## context_files

- `crates/w2m_vm/src/run.rs` (`flush_through_stub`, `FlushCounts`); `crates/w2m_vm/tests/common/mod.rs` (`area`,
  `start`); `crates/hypervisor/src/lib.rs` (`VcpuHandle`, `hv_vcpus_exit`); BLACKBOARD Deferred (Sprint 305)

## signal contract

### Emits

- None; tests.

### Invariants

- Every run as held.

## artifact contract

### Files modified

- `crates/w2m_vm/tests/` (a new module and `all.rs`' line), BLACKBOARD's Deferred entry at the close

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** test (1): 33 values back as set, one flush, no cancel; test (2): the same, the cancel noted.
- **Planted faults:** (1) the restore loop writing x(i) from slot i + 1: test (1) must name the first register wrong;
  (2) the `Exit::Canceled` arm returning an error instead of running the stub again: test (2) must fail on the error.

## done criteria

The flush's two promises, state given back and a pending cancel absorbed, each held by a test that needs no second
thread.

## self-review checklist

- [x] Scope names one verifiable property: the nested run's state and cancel.
- [x] context_files cite the function, the setup and the cancel call.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each changing what its judge reads.
- [x] One concept: one function's two promises, in one test file.

## result

