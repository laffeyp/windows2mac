# Sprint 136 — The monitor code from Rust

```yaml
---
id: 136
status: done
phase: 6
pass_kind: functional
---
```

## scope

The walking skeleton's fifth link. The monitor code the Python host writes at a VM's start, written from Rust with
`a64_encoder` (Sprint 135) and an `Assembler` with labels (`vm_backend.py` 444-519): the 16 vectors (4209-4216: the
synchronous current-EL slot branches to the exception entry, the others `hvc #2`), the dispatcher (4217-4258: the
two-way table lookup on x17, the miss into the in-VM compiler under the runtime lock when `VMC_ENABLED`, else the
host's `hvc #1` and the cache and TLB maintenance), the empty dispatch table (4264-4265), the exception entry
(4343-4400: the code-write path to `hvc #13`, the save of x0-x30, ESR, FAR and ELR, `hvc #14`, the vectors' save, the
ERET to EL1h, the call into `w2m_exception` under the lock, the return through the dispatcher), and the helpers it
uses (`lock_runtime`, `unlock_runtime`, `count_from_budget`, `budget_from_count`, 521-557). Each Rust function cites
the lines it carries. Its judge is behavior in a Rust VM, since the Python's words exist only when the Python runs: a
dispatch-table hit in either of a pair's two slots reaches its code; a miss with the compiler off reaches `hvc #1`
with x17 the target; a write fault on a read-only page with the code-write syndrome reaches `hvc #13` with x17 the
faulting address; any other fault saves the registers, ESR, FAR and ELR to the save area and reaches `hvc #14`.

## prerequisites

- 135 (closed)

## context_files

- `src/w2m/vm_backend.py` 444-557 (`Assembler`, the lock and budget helpers), 4207-4267 (`_write_monitor`),
  4343-4400 (`_write_exception_entry`), 158-162 (`dispatch_slot`), 1127-1132 (`HVC_CODE_WRITE`, the code-write ESR)
- `crates/a64_encoder`, `crates/w2m_vm`, `crates/guest_runtime`

## signal contract

### Emits

None yet: the skeleton's record comes with the guest program.

### Consumes

The runtime image; the layout.

### Invariants

- No Python file is added or changed; the C runtime is not changed.

## artifact contract

### Files created

- `crates/w2m_vm/src/assembler.rs`, `crates/w2m_vm/src/monitor_code.rs`, their tests

### Files modified

- `crates/w2m_vm/src/monitor.rs` (the bare vectors and return stub replaced), `crates/w2m_vm/Cargo.toml`,
  `WORKING_AGREEMENT.md`

### Content assertions

- Each monitor-code function cites the `vm_backend.py` lines it carries.

### Command exit codes

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` return 0
- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the four behaviors above, each in a Rust VM; the Assembler's branches resolve forward and
  backward labels to the offsets `llvm-objdump` reads back.
- **Planted fault:** the dispatcher's hash computed with one shift changed: a target inserted where `dispatch_slot`
  puts it misses, and the test names the target.

## done criteria

The monitor code the Python host wrote at a VM's start is written from Rust and dispatches, misses and takes faults
as the Python's did.

## self-review checklist

- [x] Scope names one verifiable property: the monitor code's four behaviors in a Rust VM.
- [x] context_files cite every Python line carried.
- [x] Emits: none yet, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: expected values and a planted fault.
- [x] One concept: the monitor code.

## result

- `crates/w2m_vm/src/monitor_code.rs`: the vectors, the dispatcher, the empty dispatch table, the exception entry and
  the four helpers, each function citing the `vm_backend.py` lines it carries; `assembler.rs`: the `Assembler`.
  `MonitorMemory::with_monitor_code` writes them; Sprint 134's bare vectors stay behind `MonitorMemory::new` for its
  two tests. The skeleton's return stub moves from the dispatcher's place to 0x2000, past the exception entry.
  `tables::page_descriptor` carries `_page_descriptor` without its block split.
- Expected values, in one Rust VM (`tests/monitor_code_in_vm.rs`): 0x7ff3456789a0 reaches its code from its pair's
  first slot and 0x140002000 from its second; 0x401000 misses to `hvc #1` with x17 the target and the PC one word
  past the miss stub's start; `str x0, [x1]` to a read-only 4 KB page reaches `hvc #13` with x17 the page's address,
  and answered 0 goes on to `hvc #14` with x1, x5, x17 (the spilled value) and x30 saved, ESR matching the code-write
  syndrome, FAR the page, ELR the store, and the page unwritten; a load from an unmapped address reaches `hvc #14`
  with ESR class 0x25, FAR and ELR saved.
- Deviation: the card named `llvm-objdump` as the Assembler's judge; its unit test compares the resolved branches
  with `a64_encoder`'s words for the same offsets, which Sprint 135 checked against clang.
- Planted fault: the dispatcher's second shift changed from 40 to 41 in `monitor_code.rs`: "0x7ff3456789a0 from its
  pair's first slot: left Ok(1), right Ok(32)"; restored, the test passes. The same fault stays as a test
  (`tests/dispatch_hash_planted.rs`), written into the dispatcher's word in memory: the target misses to `hvc #1`.
- Tests: w2m_vm 5 unit, 4 VM test files.
