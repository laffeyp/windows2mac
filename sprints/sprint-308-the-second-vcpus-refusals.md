# Sprint 308 — The second vCPU's refusals

```yaml
---
id: 308
status: active
phase: 6
pass_kind: functional
---
```

## scope

The run-loop design's card 4a (`docs/design-run-loop-on-several-vcpus-2026-10-31.md`, "The cards" 4, split at Sprint
307's close, the split written with this card's commit): before any thread, the reasons a run may not have more than one
vCPU, each refused by name. `w2m-vm` has no vCPU count today; the diagnostics are `--exit-ledger`, `--judge-blocks` and
`--log-lines` (`w2m-vm.rs` 259, 279, 285); a child process's place is `Guest.served.link`.

- **The work:** `run::vcpus_allowed(n, lse2, diagnostics, child) -> Result<(), String>`: 1 always; above 1 refused for
  FEAT_LSE2 not reported 1 (the dispatch pair's single-copy atomicity, the guest-memory design's choice 3), for a
  diagnostic named (each reads every exit in order), for a child process (the design: one vCPU until it does not), and,
  until card 4c, "a second vCPU is not built"; `hypervisor::feature("hw.optional.arm.FEAT_LSE2")` by `sysctlbyname`;
  `w2m-vm --vcpus N`, 1 by default, passed through the check before the run.
- **Not here:** a second vCPU's thread (4c); the lock on `served` (4b).

## prerequisites

- 307 (closed)

## context_files

- the design's preconditions 2 and "The threads"; `w2m-vm.rs` 241-330; `run.rs` (`run_compiling`)
- `man 3 sysctlbyname`; this Mac's `sysctl hw.optional.arm.FEAT_LSE2` (1, Sprint 303)

## signal contract

### Emits

- None; a run's refusals.

### Invariants

- Every run as held, one vCPU.

## artifact contract

### Files modified

- `crates/hypervisor/src/lib.rs` (`feature`), `crates/w2m_vm/src/run.rs` (`vcpus_allowed` and its unit test),
  `crates/w2m_vm/src/bin/w2m-vm.rs` (`--vcpus`); the design's card 4 split, with the card

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `feature` reading 1 on this Mac; `vcpus_allowed` refusing each reason by its own words, and
  allowing 1 always; `w2m-vm --vcpus 2` refused before the VM is made, naming the reason.
- **Planted faults:** (1) the diagnostic check dropped: the unit test must name the run with `--exit-ledger` and two
  vCPUs allowed; (2) the FEAT_LSE2 check reading the wrong name: the unit test of `feature` must fail on a value that is
  not 1 or 0.

## done criteria

Every reason the design gives against a second vCPU is a refusal by name, checked before a run starts.

## self-review checklist

- [x] Scope names one verifiable property: each reason refused by name.
- [x] context_files cite the design, the binary, the run and the lookup.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each changing what its judge reads.
- [x] One concept: a run's vCPU count and its refusals.

## result

- **The change** (WORK-COMMIT): `hypervisor::feature`, `hw.optional.arm.<name>` by `sysctlbyname` (in libc, which
  every macOS binary links; the crate is macOS-only, so no Linux form), with its unit test; `run::vcpus_allowed` and
  `each_reason_against_a_second_vcpu_is_refused_by_name`; `w2m-vm --vcpus N`, 1 by default, checked after the options
  are read and before any VM is made, the three diagnostics named by their flags. A child is started `--child <fd>
  <fd> <slot>` alone, so it never has the option and runs on one; the check's child clause holds the design's rule in
  the unit test, the binary passing `false`.
- **Expected values:** `feature("FEAT_LSE2")` `Some(1)` on this Mac, an unknown name `None`; each reason refused by its
  own words, one vCPU allowed whatever holds; `w2m-vm sleeper.exe --output … --vcpus 2` exit 1, "2 vCPUs refused: a
  second vCPU is not built (the run-loop design's card 4c)".
- **The gates:** GATES-LINE
- **Plant 1,** restored by `cp` and checked by `cmp`: the diagnostic check skipped (`.filter(|_| false)`); the unit
  test failed, "assertion failed: refused(vcpus_allowed(2, lse2, &[\"--exit-ledger\"], false)).contains(\"--exit-ledger
  reads every exit\")".
- **Plant 2,** restored by `cp` and checked by `cmp`: `feature` reading `hw.optional.arm.x<name>`; its test failed,
  "left: None right: Some(1)".

