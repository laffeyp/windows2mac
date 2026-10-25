# Sprint 141 — The bridges: an import's call reaches its service in the VM

```yaml
---
id: 141
status: done
phase: 6
pass_kind: functional
---
```

## scope

The run loop's second piece (`docs/design-host-compile-path-2026-10-24.md`, link 1, split by technique #17 after
Sprint 140; Sprint 142 runs `qpc` to its first declined block). The Rust host lays a program's guest memory out as
`VM.__init__` does (`src/w2m/vm_backend.py` 2382-2424: the monitor area, the process area, each loaded region, the
heap, the VirtualAlloc arena; one memory, one set of tables), writes the process area (Sprint 140) and the monitor
code (Sprint 136), and writes one bridge per import thunk (`_bridge`, 3733-3794): save the guest's x86 registers in the
state block, pass RCX, RDX, R8, R9 and RSP to the C service on the runtime stack under the runtime lock, put its result
in RAX unless the service is void, restore the registers, do RET's pop, and branch to the dispatcher; an import with
no service is `hvc #9`; a RETRY or a schedule bit goes to the switch stub (`_switch_stub`, 2650-) . Each bridge's
address goes into the recovery table (`_recon_append`, 4850-). The service table is Sprint 133's
(`guest_runtime::services`), and `service_for` (2302-2315) picks a thunk's service by DLL and name.

## prerequisites

- 140 (closed)

## context_files

- `src/w2m/vm_backend.py` 2302-2315 (`service_for`), 2382-2424 (the memory layout), 2650-(`_switch_stub`, read in
  full), 3733-3794 (`_bridge`), 4850-(`_recon_append`, read in full), 521-557 (the lock and budget helpers, carried
  in Sprint 136)
- `tools/guest_runtime/runtime.h` (`w2m_state`); `crates/guest_runtime/src/services.rs`
- `contracts/windows.process.parameters/0.1.0.json` row `parent.standard_handles`

## signal contract

### Emits

None yet: the record comes at the skeleton's gate.

### Consumes

`pe_loader::Loaded`, the process area, the monitor code, the service table.

### Invariants

- No Python file added or changed; no Python run or written.

## artifact contract

### Files created

- `crates/w2m_vm/src/guest.rs` (the memory layout and the bridges), `crates/w2m_vm/tests/bridges.rs`

### Files modified

- `crates/w2m_vm/`, `Cargo.lock`, `WORKING_AGREEMENT.md`, `BLACKBOARD.md` (Sprint 140's CI run, in the tail entry)

### Content assertions

- Each function cites the `vm_backend.py` lines it carries.

### Command exit codes

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` return 0
- `scripts/gates.sh` returns 0

## observation contract

- **Expected values**, in one Rust VM holding `qpc.exe`: with a return address R pushed on the guest stack and RCX
  `STD_OUTPUT_HANDLE` (-11), entering `KERNEL32.dll!GetStdHandle`'s bridge stops at the dispatcher's miss (`hvc #1`)
  with x17 = R, RAX equal to the parameters block's StandardOutput (`parent.standard_handles`: output_is_std), RSP
  raised by 8, the other x86 registers as they were, and the runtime lock free; `QueryPerformanceCounter` writes a
  nonzero count at [RCX] and returns 1; an import with no service (a planted thunk named `KERNEL32.dll!W2mNoSuch`)
  stops at `hvc #9`.
- **Planted fault:** the bridge's RET pop without its `add x4, x4, 8`: the RSP check fails and names the bridge.

## done criteria

A Windows program's import, called in the VM, reaches the guest runtime's service and returns to the dispatcher as
the Microsoft x64 convention says.

## self-review checklist

- [x] Scope names one verifiable property: a bridged call returns through the dispatcher with the right registers.
- [x] context_files cite every Python line carried.
- [x] Emits: none yet, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: expected values (one from Windows' rows) and a planted fault.
- [x] One concept: the bridges.

## result

- `crates/w2m_vm/src/guest.rs`: `Layout` (the memory `VM.__init__` lays out: the monitor and process areas, the loaded
  regions, the 2 MB-aligned heap and the 512 GB VirtualAlloc arena at a 1 GB-aligned IPA), `Guest::new` (the regions
  and the runtime image copied, the process area, the tables, the monitor code, then in the code cache the two switch
  stubs, one bridge per thunk, the slice stub and the entry stub, each marked in the reconstruction table), and
  `service_for`. Not carried yet: call tracing in the bridge, the COM vtables and the VM compiler's symbol table
  (Sprint 142).
- Expected values (`tests/bridges.rs`, one VM holding `qpc.exe`, 0.36 s): `GetStdHandle(STD_OUTPUT_HANDLE)` stops at
  the dispatcher's `hvc #1` with x17 the pushed return address, RSP raised by 8, RAX 0x74, the parameters block's
  StandardOutput (`parent.standard_handles`: output_is_std), the other registers kept, the budget kept and the runtime
  lock free; `QueryPerformanceCounter` writes 26,272,177,494,918 at [RCX] and returns 1; `KERNEL32.dll!W2mNoSuch`
  stops at `hvc #9`. The 513 GB memory mapped into one VM without refusal.
- Planted fault: the RET pop's `add x4, x4, 8` removed: "RET's pop: x17 the return address, RSP raised by 8",
  0xFFFFFF00 against 0xFFFFFF08; restored, the tests pass. The message names the pop, not the bridge, since one
  bridge was entered.
- Tests: w2m_vm 5 unit, 6 test files (one new).
