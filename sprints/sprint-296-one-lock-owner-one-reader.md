# Sprint 296 — One LOCK_OWNER, one reader

```yaml
---
id: 296
status: done
phase: 6
pass_kind: functional
---
```

## scope

Two debts named at Sprint 294's close, before `Guest` is divided on them:
- **The owner's read sits in the run loop.** `threads::schedule`'s assertion (Sprint 294) calls `crate::run::own_word`,
  the scheduler calling up into the loop that calls it; the drain reads the same word itself (`run.rs` 470).
- **`LOCK_OWNER` is defined twice**, both 0xB0: `guest_runtime/src/layout.rs` 26, whose value the C runtime's header
  takes as `W2M_LOCK_OWNER_OFF` (`layout.rs` 192), and `w2m_vm/src/run.rs` 34, which `cases.rs` and `tests/litmus.rs`
  import. A change to one would leave the host reading a word the runtime does not write.

- **The work:**
  - `monitor_code::vcpu_owner(mem)`, beside `runtime_word_held`, the one read of a vCPU's owner; `run::own_word` and
    the drain's read replaced by it; `threads::schedule` calling it;
  - a unit test of `vcpu_owner` against a buffer's monitor data;
  - `run.rs`' `LOCK_OWNER` removed, its users importing `guest_runtime::layout::LOCK_OWNER`;
  - a test that fails when `pub const LOCK_OWNER` is defined more than once under `crates/`.
- **Not here:** `Guest` divided.

## prerequisites

- 295 (closed)

## context_files

- `crates/w2m_vm/src/run.rs` 30-40, 460-475, 550-565; `crates/w2m_vm/src/monitor_code.rs` 130-175;
  `crates/w2m_vm/src/threads.rs` 684-695; `crates/guest_runtime/src/layout.rs` 20-30, 185-195
- `crates/w2m_vm/src/cases.rs` 30-35, 385-390; `crates/w2m_vm/tests/litmus.rs` 18-22, 210-215

## signal contract

### Emits

- None; one definition and one reader, held by tests.

### Invariants

- Every run as held: the batch run, the guests, the family runs, the run loop's tests, the litmus tests.

## artifact contract

### Files modified

- `crates/w2m_vm/src/` (`run.rs`, `monitor_code.rs`, `threads.rs`, `cases.rs`), `crates/w2m_vm/tests/litmus.rs`, a new
  test under `crates/w2m_vm/tests/`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** one `pub const LOCK_OWNER` under `crates/`; `served_outside_word` 0 on the qpc and sleeper runs;
  every run as held.
- **Planted faults**, each judge looked up to reach the plant's line and to assert what it breaks:
  - (1) `vcpu_owner` reading 8 bytes past the owner's word. Judge: a unit test in `monitor_code.rs` that writes an
    owner into a buffer's monitor data and asserts `vcpu_owner` returns it. The run-loop tests were looked up and
    rejected: through `take_word` a wrong owner counts nothing when the word is free and spins when the vCPU holds
    it, and `schedule`'s assertion passes with the host holding the word, so neither asserts what the plant breaks.
  - (2) a second `pub const LOCK_OWNER` written into `run.rs`. Judge: the new test, which counts the definitions.

## done criteria

The runtime word's owner offset is defined once, where the C runtime's header takes it, and read in one place.

## self-review checklist

- [x] Scope names one verifiable property: one definition, one reader.
- [x] context_files cite every user found by `grep`.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each judge looked up.
- [x] One concept: the owner word's one home.

## result

- **The change** (50eea43e1): `monitor_code::vcpu_owner`, the one read of a vCPU's owner, for `take_word`, the drain
  and `threads::schedule`, `run::own_word` gone; its unit test; `tests/layout_constants.rs`.
- **A class, not one name, a departure:** the card named `LOCK_OWNER`; a search found seven names `w2m_vm/src` defined
  again from `layout.rs`, all equal: `AP_READ_ONLY` (`tables.rs` 81), `CLOCK_WARP` (`process.rs` 25), `FCW_OFF`,
  `LOCK_OWNER`, `MXCSR_OFF`, `RUN_END` (`run.rs` 32-35), `PER_THREAD_MONITOR` (`threads.rs` 49). Each is now a
  `pub use` of `layout.rs`' on the same line, its importers unchanged; the test fails on any `layout.rs` name
  defined again in `w2m_vm/src`. Reviewer-3's Low 1 at 296 found the class wider still, across `crates/`; card 297.
- **Expected values:** one definition of each of the seven; 175 tests in the pre-commit run, the batch-run step
  first; the run loop's tests with `served_outside_word` 0.
- **The gates:** `scripts/gates.sh` exit 0 in 892 s, kept in `store/gates/passes.tsv` as sprint 296 at 50eea43e1 with root=8ce0426a4 and "rust=crates python=0 samples=0", rdeps(w2m_vm), 175 tests, base=05646ee7d, Sprint 295's close; multi-vCPU: not run (Python gates not run)
- **Plant 1,** restored by `cp` and checked by `cmp`: `vcpu_owner` reading 8 bytes past the owner's word;
  `the_owner_is_the_monitor_datas_lock_owner_word` failed, "left: 9 right: 3".
- **Plant 2,** restored by `cp` and checked by `cmp`: `pub const LOCK_OWNER: u64 = 0xB0;` written into `run.rs`
  again; `no_layout_constant_is_defined_again_in_w2m_vm` failed, "defined again outside layout.rs: LOCK_OWNER in
  …/w2m_vm/src/run.rs".
