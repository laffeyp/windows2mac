# Sprint 134 — The runtime runs in a Rust VM

```yaml
---
id: 134
status: active
phase: 6
pass_kind: functional
---
```

## scope

The walking skeleton's third link. A Rust VM runs a function of the C guest runtime where it was linked, with the
MMU on: the monitor area (`MONITOR_SIZE` bytes) mapped at `IPA_BASE`; stage-1 tables written for
`(MONITOR_VA, IPA_BASE, MONITOR_SIZE)` by a port of `vm_backend._write_translation_tables` (`src/w2m/vm_backend.py`
2204-2242), 4 levels, 2 MB blocks where aligned, the descriptor bits `PAGE_ATTRIBUTES` (352); the runtime image
(Sprint 133) copied to `RUNTIME_IMAGE`; the system registers the Python host sets (4980-4981: CPACR_EL1 `CPACR_FPEN`,
MAIR_EL1 0xFF, TCR_EL1 `TCR`, TTBR0_EL1 the root, SCTLR_EL1 `SCTLR`, ACTLR_EL1 `ACTLR_ENTSO`); vectors that hand any
exception to the host by `hvc`, so a fault is seen, not lost. The vCPU calls `w2m_cpuid_query(leaf, sub, r)`
(`tools/guest_runtime/runtime.c` 119) with the link register at a stub that executes `hvc`, once for each input
`tests/test_vm_state.py` 203-205 lists: every leaf of `specs/guest_cpuid.json`, then (1, 5), (7, 1), (8, 0),
(0x40000000, 0), (0x80000009, 0), (0xFFFFFFFF, 0). Each answer must equal `cpuid.lookup`'s (`src/w2m/cpuid.py`
30-41, carried as the judge), as that test asserted for the Python host's VM.

## prerequisites

- 133 (closed)

## context_files

- `src/w2m/vm_backend.py` 80-86, 131-132, 348-355 (the descriptor bits and system register values), 2204-2242,
  2420-2424 (the map list), 4980-4981
- `src/w2m/cpuid.py` (the judge), `tests/test_vm_state.py` 193-240 (the inputs and the assertion carried)
- `tools/guest_runtime/runtime.c` 112-142; `crates/hypervisor`, `crates/guest_runtime`

## signal contract

### Emits

None yet: the skeleton's record comes with the guest program (a later link).

### Consumes

The runtime image; `specs/guest_cpuid.json`.

### Invariants

- No Python file is added or changed; the C runtime is not changed.

## artifact contract

### Files created

- `crates/vm/` (the monitor area, its tables, the image loaded, a call into the runtime), its tests

### Files modified

- `Cargo.toml`, `Cargo.lock`, `WORKING_AGREEMENT.md` (canonical homes)

### Content assertions

- The table writer cites `vm_backend.py` 2204-2242 and keeps its levels, block sizes and descriptor bits.

### Command exit codes

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` return 0
- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** for each of the inputs, the runtime's answer equals `cpuid.lookup`'s, and the vCPU returns to
  the stub (`hvc`) each time, with no exception vector taken.
- **Planted fault 1:** a table with the image's level-2 block left out: the call faults (an instruction abort),
  and the vector's `hvc` names the exception, in place of a hang or a wrong answer.
- **Planted fault 2:** the first leaf's EAX changed in the expected values: the comparison fails naming the leaf.

## done criteria

A function of the C guest runtime, linked by Rust, runs in a Rust VM at its link address and answers as the Python
host's VM answered.

## self-review checklist

- [x] Scope names one verifiable property: the runtime's CPUID answers equal the oracle's, from a Rust VM.
- [x] context_files cite every Python line carried.
- [x] Emits: none yet, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: expected values and two planted faults.
- [x] One concept: the runtime called in a Rust VM.

## result

- `crates/w2m_vm`: `tables.rs` (`_write_translation_tables`, carried line for line; two unit tests walk the tables as
  the MMU does and fill the area), `monitor.rs` (the monitor area with the runtime image copied where it was linked,
  16 vectors each an `hvc #0x100 + slot`, a return stub, the system registers the Python host set, and `call`),
  `lib.rs` (the values, each citing its `vm_backend.py` line).
- Expected values, held on the first run: for all 26 inputs (the spec's 20 leaves and the 6 past its edges), the
  runtime's `w2m_cpuid_query`, called in a Rust VM with the MMU on and EAX's upper half set to 0xDEAD, wrote the
  answer `cpuid.lookup` gives, and returned to the stub each time.
- Planted faults: (1) the runtime image's blocks left out of the tables: the call takes the vector at VBAR + 0x200
  (`hvc #0x104`), ESR's class 0x21 (an instruction abort from the current EL), ELR the function's address; (2) the
  expected EAX of leaf 0 raised by one in the test: it failed naming "CPUID(0x0, 0x0)", 13 against 14, and passed
  again restored.
- Tests: w2m_vm 6 (4 unit, 2 in a VM); the gates pass.
