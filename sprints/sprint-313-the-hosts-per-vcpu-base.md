# Sprint 313 — The host's per-vCPU base

```yaml
---
id: 313
status: done
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

- **The base:** `w2m_vm::VcpuPages { monitor, state }` (`lib.rs`, `guest` re-exporting it), `VcpuPages::FIRST`
  vCPU 0's (`MONITOR_DATA`, `MONITOR_SIZE + STATE_OFF`); `Served.vcpu`, the served exit's vCPU, set by `GuestLock::serve(pages)` once the lock
  is taken and read back by `turn` across its wait; the run loop's own `pages` and `md` for its reads outside a service
  (`run.rs`, one vCPU, the first). `served.vcpu` is a Served row of the run-loop design's table, 24 now, which
  `the_served_part_is_the_designs` holds.
- **The reads re-addressed:** `threads.rs` 28 lines and the scheduler's word assertion; `run.rs`: `take_word` and
  `exit_count` given the page, `give_back`, the `hvc #12` state read, every `serve`; `cases.rs` 3 functions' monitor and
  state reads and 3 `exit_count` calls; `model_step.rs` its monitor and state bases; `guest.rs` the two RSP reads;
  `monitor_code::vcpu_owner(mem, monitor)` and its callers; the load-time writes of vCPU 0's page (`process.rs`, the
  monitor code's header) through `VcpuPages::FIRST`. The guest addresses, x28 and `PROCESS_VA + STATE_OFF` in
  generated code, are unchanged; the transport page and the run table stay at their one offset; the process area's
  thread-start defaults (`process.rs` 385) are written into vCPU 0's state block at load, inside the area's own buffer.
- **The source test** `vcpu_pages::no_vcpu_page_is_read_at_a_fixed_offset` (`tests/vcpu_pages.rs`, a module of
  `tests/all.rs`): no line under `src/` uses `MONITOR_DATA` as a value except after `MONITOR_VA +`, or
  `MONITOR_SIZE + STATE_OFF`, a name with its `guest_runtime::layout::` path counted as bare, save
  `VcpuPages::FIRST`'s; `the_rule_names_each_form` holds the rule on seven lines.
- **Plant 1,** restored by `cp` and checked by `cmp`: the run's base one page off (`monitor: FIRST.monitor + 0x1000`);
  `run_loop::sleeper_creates_and_ends_its_thread_as_the_python_host` failed, "panicked at
  crates/w2m_vm/tests/run_loop.rs:180:5: assertion `left == right` failed: the translations in compile order (start,
  end, discovery)", the drain reading the wrong page, before the first exit's count the card named.
- **Plant 2,** restored likewise: `vcpu_owner`'s read put back at `MONITOR_DATA + LOCK_OWNER`; the source test failed,
  "a vCPU's page read at a fixed offset: [\"monitor_code.rs:173\"]".
- **The Linux check:** `VcpuPages` first sat in `guest.rs`, a macOS-only module; `scripts/pre-commit.sh`'s Linux check
  failed, E0433 "cannot find `guest` in the crate root" at `monitor_code.rs` 22 and `process.rs` 275, which build on
  Linux; it moved to the crate root.
- **Changed from the card:** the files named in context plus the design's table row and count and
  `guest_parts.rs`' count; `host_request.rs`, `compile.rs` and `block_judge.rs` held no fixed-offset read.
- **The gates:** `scripts/gates.sh` exit 0 in 736 s, kept in `store/gates/passes.tsv` as sprint 313 at 96939301f with root=76a327741 and "rust=crates python=0 samples=0", 186 tests, 651 ns an exit at load 4.22, base=b137bd162, Sprint 312's close; multi-vCPU: not run (Python gates not run)

