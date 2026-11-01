# Sprint 313 — The host's per-vCPU base

```yaml
---
id: 313
status: active
phase: 6
pass_kind: functional
---
```

## scope

The per-vCPU pages design's card 4c2a (`docs/design-each-vcpus-own-pages-2026-10-31.md`, "The cards" 1; BLACKBOARD
Decision "Architecture: each vCPU's own pages"). The host reads a vCPU's monitor data page and state block by fixed
offset, `MONITOR_DATA + X` and `PROCESS_VA + STATE_OFF` turned into memory offsets, in about 100 places; on two vCPUs
each must read the exiting vCPU's own pages, which card 4c2b puts at that vCPU's `VCPU_STRIDE` slot.

- **The work:** `guest::VcpuPages`, the memory offsets of one vCPU's monitor data page and state block, vCPU 0's
  today's; a field of `Guest`, the vCPU a served exit serves, which `GuestLock::serve` takes and sets, and `run`'s own
  copy for its reads outside a service. Every host read or write of a per-vCPU word or the state block goes through it:
  the count and limit, the lazy-flag and spill words, `LOCK_OWNER`, `SLICE_*`, `CURRENT*`, `RUN_END`, `SAVE_AREA`,
  `SAVE_VECTORS`, `CODE_WRITE_SPILL`, `NATIVE_OPERANDS`, `PAUSE_COUNT`, the state block's fields; and the set-once
  words (`CLOCK_MODE`, `CLOCK_RATE`, `CLOCK_EPOCH`, `START_MODE`, `FOREGROUND_MODE`), which card 4c2b copies to every
  page. The transport page and the run table stay at their one offset. The generated monitor code's guest addresses
  are unchanged, being the same on every vCPU. One vCPU; every run as held.
- **Not here:** the private pages and tables (4c2b); the threads (4c3).

## prerequisites

- 312 (closed)

## context_files

- the per-vCPU pages design, its inventory and cards; `guest.rs` (`Guest`, `memory_view`); `run.rs` (`GuestLock`,
  `run`); the readers by file: `threads.rs`, `run.rs`, `cases.rs`, `model_step.rs`, `process.rs`, `monitor_code.rs`,
  `host_request.rs`, `compile.rs`, `block_judge.rs`

## signal contract

### Emits

- None; the host's reads re-addressed.

### Invariants

- Every run as held; one vCPU.

## artifact contract

### Files modified

- `crates/w2m_vm/src/` (`guest.rs`, `run.rs` and the readers); a test that no per-vCPU word is read at a fixed offset

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** every run as held, the sleeper and qpc runs, the family runs and the batch run; a source test
  that finds no `MONITOR_DATA +` and no `STATE_OFF` read outside `VcpuPages` and the layout, by file and line.
- **Planted faults:** (1) vCPU 0's `VcpuPages.monitor` one page off: the sleeper test must fail, its first exit's count
  read from the wrong page; (2) one read put back at `MONITOR_DATA + LOCK_OWNER`: the source test must name its file and
  line.

## done criteria

The host reads each vCPU's own state through that vCPU's base, so a second vCPU's pages need only a second base.

## self-review checklist

- [x] Scope names one verifiable property: no per-vCPU read at a fixed offset.
- [x] context_files cite the design, the lock and the readers.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each changing what its judge reads.
- [x] One concept: one base for the host's reads of a vCPU's state.

## result

