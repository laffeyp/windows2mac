# Sprint 297 — One definition a fact

```yaml
---
id: 297
status: active
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-3's Low 1 at 296: `pub const` names defined in two files under `crates/`, beyond `tests/layout_constants.rs`'
reach. The search at 296's close found 24 besides `ALL` (generated enums, each its own type); each pair was read for
its value, its meaning and the crates' dependencies (`Cargo.toml`: `guest_runtime` depends on nothing; `pe_loader`
and `virtual_memory` on `pe_image`; `cpu_cases` on `guest_runtime`; `translator` on `a64_encoder` and `x86_decoder`;
`w2m_vm` on all of these):

- **One fact in one crate, merged:** `APP_DIRECTORY` (`modules.rs` 22, `process.rs` 44), `KIND_STANDARD` (`kernel.rs`
  31, `process.rs` 47), `STILL_ACTIVE` (`kernel.rs` 27, `u64`; `threads.rs` 51, `u32`), `VMC_ENABLED`
  (`monitor_code.rs` 92, `guest.rs` 50).
- **One fact across crates, one home where a dependency allows:** `HEAP_VA`, `HEAP_SIZE`, `MONITOR_VA` (`pe_loader`
  31-33, from `guest_runtime`'s `layout.rs`); `PAGE` (`virtual_memory` 21, from `layout.rs`); `STACK_GUARD_PAGES`
  (`w2m_vm/src/process.rs` 45, from `virtual_memory` 54); `HVC_SLOW_SSE`, `HVC_SLOW_STEP` (`cases.rs` 50-51, `u16`)
  and `REG_UNDEFINED_FLAGS` (`run.rs` 37, `u8`), derived from the translator's (`lower.rs` 27, 43, 45, `u32`).
- **One fact across crates no dependency joins, held equal by a test:** `ACCESS_VIOLATION` (`cpu_cases` 34, `u64`;
  `pe_loader` 48, `u32`), held by a test in `w2m_vm`, which depends on both; `X87_ST_OFF`, `X87_SW_OFF` (the
  translator's against `layout.rs`), held since Sprint 236.
- **Different facts that share a name, allowed by name:** `STACK_SIZE` (`cpu_cases` 0x10000, a case's stack;
  `pe_loader` 1 MiB, a thread's default); `ELR_EL1`, `ESR_EL1`, `FAR_EL1`, `SPSR_EL1` (`a64_encoder`'s MSR operand
  tuples; `hypervisor`'s Hypervisor.framework register ids); `IE`, `DE`, `ZE`, `OE`, `UE`, `PE` (the FPU status word's
  bits in `x87.rs`, MXCSR's in `x86fp.rs`, two registers Intel lays out alike).

- **The work:** the merges and the one homes, each a `pub use` or a value derived from the other's path; the equality
  test; `tests/layout_constants.rs` widened to every `pub const` under `crates/`: a name in two files fails unless one
  definition names the other's path in its value, or the name is allowed with its reason, or a named test holds the two
  equal.
- **Not here:** renaming the different facts.

## prerequisites

- 296 (closed)

## context_files

- each definition above; `crates/*/Cargo.toml`; `crates/w2m_vm/tests/layout_constants.rs`
- the Sprint 236 test of the translator's x87 offsets; BLACKBOARD Surfaced, the review at 296

## signal contract

### Emits

- None; definitions, held by a test.

### Invariants

- Every run as held, and every crate's tests.

## artifact contract

### Files modified

- the definitions above, `crates/pe_loader/Cargo.toml` and `crates/virtual_memory/Cargo.toml` (a dependency on
  `guest_runtime`), `crates/w2m_vm/tests/layout_constants.rs`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the widened test passing, every two-file name merged, derived, held or allowed by name.
- **Planted faults**, each judge the widened test, which reads every `pub const` under `crates/` and asserts the class,
  each plant a definition the test reads:
  - (1) `pub const PAGE: u64 = 0x1000;` written back into `virtual_memory` in place of its `pub use`: the test must
    name `PAGE` defined in two files.
  - (2) `ACCESS_VIOLATION` in `pe_loader` set to 0xC000_0006: the equality test must fail naming both values.

## done criteria

Every fact a `pub const` states under `crates/` has one definition, or a named reason for two.

## self-review checklist

- [x] Scope names one verifiable property: one definition a fact, or a named reason.
- [x] context_files cite the definitions, the manifests and the tests.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each changing what its judge reads.
- [x] One concept: duplicate definitions.

## result

- **The gates:** GATES-LINE
