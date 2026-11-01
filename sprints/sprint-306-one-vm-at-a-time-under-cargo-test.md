# Sprint 306 — One VM at a time under cargo test

```yaml
---
id: 306
status: done
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-3's Low 1 at 303: on this Mac `cargo test -p w2m_vm --test all` ran 72 passed and 8 failed at `Vm::new`
(Sprint 303), Hypervisor.framework allowing one VM a process and `cargo test` running a binary's tests as threads of
one; WORKING_AGREEMENT 142 records the failure instead of removing it. Eight test files make a VM in-process:
`run_loop.rs` and `litmus.rs` each hold a lock of their own (`ONE_VM`), which serialises their own tests and not one
file's against another's; `bridges.rs`, `cpuid_in_vm.rs`, `dispatch_hash_planted.rs`, `first_decline.rs`,
`monitor_code_in_vm.rs` and `unmapped_image_faults.rs` hold none.

- **The work:** one lock for the binary, in `tests/all.rs`, a `static ONE_VM: Mutex<()>` and a helper returning its
  guard whatever an earlier panic left (`into_inner` on a poisoned lock); every VM test takes it first and holds it for
  its life; `run_loop.rs`' and `litmus.rs`' own locks replaced by it. nextest, a process a test, is unaffected.
  WORKING_AGREEMENT 142 says `cargo test` exits 0 on this Mac.
- **Not here:** putting `cargo test` on macOS into the gates or `scripts/pre-commit.sh` (its 197 s is the gates'
  nextest run's work again); `hypervisor`, a binary a file since Sprint 303.

## prerequisites

- 305 (closed)

## context_files

- `crates/w2m_vm/tests/all.rs` and the eight files; `run_loop.rs` 677 and `litmus.rs`' `ONE_VM`
- WORKING_AGREEMENT 142; Sprint 303's result

## signal contract

### Emits

- None; the tests' setup.

### Invariants

- Every test as held under nextest.

## artifact contract

### Files modified

- `crates/w2m_vm/tests/` (`all.rs` and the eight files), `WORKING_AGREEMENT.md`

### Command exit codes

- `cargo test -q -p w2m_vm --test all` returns 0 on this Mac, three runs in a row; `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** 80 tests under `cargo test`, 0 failed, three runs; the time of a run.
- **Planted faults:** (1) the lock left out of `unmapped_image_faults`' test: `cargo test` must fail at `Vm::new` in
  some run of three, the failure quoted; the plant's runs counted, a race being what it plants.

## done criteria

`cargo test` on this Mac runs `w2m_vm`'s tests to exit 0, the one-VM rule held by the tests themselves.

## self-review checklist

- [x] Scope names one verifiable property: `cargo test` exit 0 on this Mac.
- [x] context_files cite the tests, the locks and the rule.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and a plant changing what its judge reads.
- [x] One concept: the tests' one-VM rule; ten files, each one line of setup.

## result

- **The change** (d0464e0c2): `tests/all.rs` `ONE_VM` and `one_vm()`, macOS-only like `common`, the poisoned lock taken
  regardless; `run_loop.rs` and `litmus.rs` import it in place of their own; `bridges`, `cpuid_in_vm`,
  `dispatch_hash_planted`, `first_decline`, `monitor_code_in_vm` and `unmapped_image_faults` take it first; and the case
  runner's test, which the card's list lacked; WORKING_AGREEMENT 142.
- **A VM-maker the card's list missed, found by the first run:** `cargo test` ran 80 passed and 1 failed,
  `unmapped_image_faults` holding the lock and `hv_vm_create` refused, `case_runner`'s
  `every_judged_case_agrees_with_windows_with_the_rust_translator` running then: it makes its VM through
  `case_inputs::with_case_vm` (`case_inputs.rs` 230), which the card's search, by `Vm::new`, `Guest::new` and
  `guest_for` in the tests, did not name. Searched again by the place a VM is made: the library calls `Vm::new` at
  `case_inputs.rs` 230 and in `w2m-vm`'s binary alone, and only the case runner's test reaches the first; `Guest::new`,
  which the library's unit tests call, maps memory and makes no VM. The two runs left of that three were stopped, their
  answer known.
- **Expected values:** `cargo test -q -p w2m_vm --test all`, three runs: exit 0, 81 passed, in 199, 186 and 186 s.
- **The gates:** `scripts/gates.sh` exit 0 in 724 s, kept in `store/gates/passes.tsv` as sprint 306 at d0464e0c2 with root=ba1804f74 and "rust=crates python=0 samples=0", rdeps(w2m_vm), 181 tests, base=8c18c4e00, Sprint 305's close; multi-vCPU: not run (Python gates not run)
- **Plant 1,** restored by `cp` and checked by `cmp`: the card named `unmapped_image_faults`' lock, a short test whose
  collision is chance; the plant took the case runner's off instead, a 170 s test any other VM test overlaps; the first of up to three runs failed, 80 passed and 1 failed, `unmapped_image_faults` at "HvError { call: \"hv_vm_create\", code: -85377016 }".

