# Sprint 166 — An unknown condition refused, not lowered as another

```yaml
---
id: 166
status: done
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

## result

- **The change:** `inverse` and `cond_flags` (`crates/translator/src/lower.rs`) name `le` and return a `CompileError`
  naming any other condition they do not hold; `op_flags`, `op_flag_masks` and `insn_flags` carry it, and
  `assemble_block` with it. The run loop's look-ahead (`crates/w2m_vm/src/compile.rs`), which returns a flag mask,
  counts every flag live on the error; the instruction's own compile refuses it by name.
- **The judges:** the C words judge compares 1,184,313 of 1,184,578 lifted calls with no Rust refusal and no
  difference, as before; the case runner's three tests pass (133.96 s with the second plant in); the four
  `case-runner-rust` records (`1793173959000000000`, `1793174000000000000`, `1793174024000000000`,
  `1793174038000000000`) hold SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED and CANDIDATE_ACCEPTED for 205,
  174, 107 and 26 classes and SESSION_COMPLETED, with no discrepancy, as Sprint 161's.
- **Plant 1** (`inverse`'s `_ => "gt"` restored): the unit test failed at `inverse("xx")`.
- **Plant 2** (`cond_flags`'s `le` arm dropped) did not do what the card said. The case runner passed with it, the C
  words judge passed with it, and `qpc` and `sleeper` ran to ExitProcess with it. `compile_insn` reads `op_flag_masks`
  only for an instruction that holds a flags op (`lower.rs` 1215), and a Jcc, SETcc or CMOVcc holds none; `cond_flags`
  is reached only through `insn_flags`, from `assemble_block` and the run loop's look-ahead, and the two guests'
  host-compiled blocks branch on no `le`. The unit test caught it (`cond_flags("le")`), and a second test now holds
  every one of `lift::cc`'s 16 conditions to the table. No end-to-end judge reaches `cond_flags` for every name; the
  batch run at step 4 is the first that may.
