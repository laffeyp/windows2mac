# Sprint 166 — An unknown condition refused, not lowered as another

```yaml
---
id: 166
status: active
phase: 6
pass_kind: functional
---
```

## scope

The code review of 2026-10-28, finding 7's two silent fallbacks (Decision 2026-10-28): `crates/translator/src/lower.rs`
`inverse` (582-598) gives "gt" for any condition it does not name, and `cond_flags` (610-621) gives `le`'s flags for
any x86 condition it does not name. The frozen Python raises on both, since `INVERSE` and `COND_FLAGS` are dicts
(`src/w2m/vm_backend.py` 830-831, 852-853): an unknown key is a KeyError, which refuses the block. The Rust lowers the
block with a wrong branch or a wrong flags liveness, on one vCPU.

- `inverse` names `le` (to `gt`) and every other condition `a64.COND` holds but `al`, and refuses any other name with a
  `CompileError` naming it.
- `cond_flags` names `le` and refuses any other name the same way; its callers (`op_flags`, 657-658) carry the error.
- No condition the lifter makes reaches either refusal: `lift::cc` (`lift.rs` 245-264) gives 16 names, which strip to
  o, b, z, be, s, p, l and le, and `le` alone reached `cond_flags` through its fallback; `inverse` takes the Arm
  conditions `nzcv_condition` gives.
- The words for every condition the judges reach are unchanged (the C words judge, the case runner, the Python words
  judge).

## prerequisites

- 165 (closed)

## context_files

- `crates/translator/src/lower.rs` 560-660, 1620-1630
- `src/w2m/vm_backend.py` 826-860; `src/w2m/a64.py` `COND`

## signal contract

### Emits

- On four `store/runs/case-runner-rust-<ns>` records (the four reference profiles): SESSION_INIT, CANDIDATE_GENERATED,
  COMPARISON_COMPLETED, CANDIDATE_ACCEPTED, SESSION_COMPLETED, as Sprints 159-161's.

### Consumes

The case runner's fixture and Windows results; the batch run's record calls (the C words judge).

### Invariants

- No Python file added or changed; every judged code word unchanged.

## artifact contract

### Files modified

- `crates/translator/src/lower.rs`, `WORKING_AGREEMENT.md`; four records under `store/runs/`

### Command exit codes

- the C words judge, the case runner and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the C words judge's compared count and the case runner's class and instruction counts equal
  Sprint 161's (11,518 calls; 131,649 instructions in 289 classes); no new refusal.
- **Planted fault:** a unit test lowers a branch on a condition named "xx": before, it compiles; after, the error names
  "xx". A second plant drops the `le` arm: the case runner names the classes that branch on `le`.

## done criteria

No condition the lowering does not name is lowered as another.

## self-review checklist

- [x] Scope names one verifiable property: an unknown condition refused.
- [x] context_files cite the Rust and the Python it ports.
- [x] Emits: the case runner's records, by tag.
- [x] Artifact contract gradable by command.
- [x] Observation contract: unchanged counts and two plants.
- [x] One concept: the two fallbacks.
