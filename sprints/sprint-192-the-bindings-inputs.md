# Sprint 192 — The bindings' inputs

```yaml
---
id: 192
status: active
phase: 6
pass_kind: functional
---
```

## scope

reviewer-2's Medium 3 and Lows 2, 3 and 4 of its review of Sprints 185-189 (the Decision "Sequence after Sprint 190"):
the gates' trees and `built_from`'s digest leave out inputs that change what is judged, the multi-vCPU stage's results
are untracked, the layout assertion leaves out halfword 3, and no test guards the VEX 256-bit store branch's reach.

- **The gated tree:** `gates.sh` stages the whole work tree into its temporary index and removes the bookkeeping paths
  (BLACKBOARD.md, KIT_DIARY.md, WORKING_AGREEMENT.md, sprints/, signal-reports/, docs/, store/gates/), and keeps one
  tree id of the root in the pass line's seventh column; `trees_moved` takes the work commit's root tree the same way
  and compares it whole, naming the first top-level path that differs. Its doc says "newest". `--check` also fails a
  "Sprint N closed:" commit that changes a path outside the bookkeeping ones.
- **The build's digest:** `crates/w2m_vm/build.rs` digests `crates/`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`
  and `.cargo/`, and `built_from` takes the same.
- **The multi-vCPU results tracked:** the stage appends to `store/gates/multi-vcpu.tsv`, and `--check` matches a closed
  entry's claim that the three passed to its sprint's three lines.
- **Halfword 3:** `layout.rs` asserts `RUNTIME_IMAGE_VA >> 48 == 0`.
- **The VEX store's reach:** a translator test that no VEX class with a memory destination at VL=256 lowers to `hvc #10`.

## prerequisites

- 191 (closed)

## context_files

- `scripts/gates.sh`, `crates/signal_report/src/main.rs` (`trees_moved`, `closed_too_early`), `tests/check.rs`,
  `crates/w2m_vm/build.rs`, `src/built_from.rs`, `crates/guest_runtime/src/layout.rs`, `crates/translator/src/lower.rs`
  (`lower_vex`, `VEX_NATIVE_256`, `VEX_SLOW`), `crates/translator/src/rules.rs` (`VEX_CLASSES`, `vex_base`)

## signal contract

### Emits

- None: no producer runs; the judges are tests and checks.

### Invariants

- No Python file added or changed; `--check` passes over Sprints 182-191 as before.

## artifact contract

### Files modified

- `scripts/gates.sh`, `store/gates/multi-vcpu.tsv` (new), `crates/signal_report/src/main.rs`, `tests/check.rs`,
  `crates/w2m_vm/build.rs`, `src/built_from.rs`, `crates/guest_runtime/src/layout.rs`, a translator test,
  `WORKING_AGREEMENT.md`

### Command exit codes

- `signal_report --check`, its tests, the translator's tests and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the root tree compared whole from Sprint 192; the multi-vCPU lines matched; the assertions hold.
- **Planted faults:** (1) `Cargo.toml` changed after the gates' pass: `--check` names the root tree's change; (2) a
  close commit that changes a crate file: `--check` names it; (3) a VEX move with a memory destination taken out of
  `VEX_NATIVE_256`: the translator test names it.

## done criteria

Every input that changes what is judged is in the gates' tree and the build's digest, and the multi-vCPU stage's
results are on the record.

## self-review checklist

- [x] Scope names one verifiable property: the judged inputs bound.
- [x] context_files cite the code each binding lives in.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the bindings' inputs.
