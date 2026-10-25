# Sprint 151 — The lifter and the integer core: link 5's first group, lifted and lowered in Rust

```yaml
---
id: 151
status: active
phase: 6
pass_kind: functional
---
```

## scope

Link 4 (the lifter) folds into link 5's first group, as the link-3 review advised: a lifter judged alone would be
judged only against a Python fixture of its own ops, while a group of rules lifted and lowered is judged against the C
emitter's words and against Windows (`docs/design-host-compile-path-2026-10-24.md`, links 4 and 5). The operand forms
(`x86ops.py`) are in Rust already (Sprint 144, equal to the C's on every record and sweep instruction).

A new crate, `crates/translator`, carries:

- **the lifter**, `src/w2m/lift.py` (355 lines, read in full): the op type (`Op`, `Loc`), `_OpBuilder`, `_operand`,
  `_val`, `_push`, `_pop`, `commit_order`, `lift_mmx`'s refusal (the MMX forms are a later group), and `lift`'s
  dispatch through a registry of Rust rules; an iclass with no Rust rule is a `LiftError` naming it;
- **group 1's rules**, from `src/w2m/rules/_int.py` (686 lines, read in full) and their 23 modules: `alu` (ADD SUB AND
  OR XOR CMP TEST), `move` (MOV MOVZX MOVSXD LEA), `jcc` (the 16 Jcc), `jmp_call` (JMP CALL_NEAR), `ret_near`, `push`,
  `pop`, `nop` (NOP and the five PREFETCH classes);
- **the lowering core**, `src/w2m/vm_backend.py` 482-519, 559-1062 and 1186-1348 and `compile_insn` (1417-1885, read
  in full) for the ops group 1 lifts (`imm`, `addr`, `ld`, `st`, `alu` add/sub/and/or/xor/sext32, `flags`
  add/sub/logic, `br`): the flag setters and liveness (`_set_flags`, `_op_flags`, `insn_flags`, `NZCV_CONDITIONS`),
  `_eval_condition`, `_exit_if_undefined`, the temporaries and their registers, operand forms (`_operand_forms`),
  address folding (`_folded_addresses`, `_mem`) and fused moves (`_fused_move`). An op outside the group is a
  `CompileError` naming it.

## prerequisites

- 150 (closed)

## context_files

- `src/w2m/lift.py`, `src/w2m/rules/_int.py`, `src/w2m/rules/__init__.py` (read in full); `src/w2m/vm_backend.py`
  as above
- `crates/x86_decoder` (`decode`, `operands`), `crates/a64_encoder`, `crates/w2m_vm/src/assembler.rs`
- `tools/vm_compiler/vm_compiler.h`; `crates/x86_decoder/tests/c_differential.rs` (the record's calls)
- `tests/test_operand_forms.py`, `tests/test_symbolic_lift.py`, and the lowering's Python tests, for values carried

## signal contract

### Emits

- The case runner's record (Sprint 150) made with the Rust compiler for group 1's classes: CANDIDATE_GENERATED,
  COMPARISON_COMPLETED and CANDIDATE_ACCEPTED for each group-1 class the cases judge, under a judge named
  `case-runner-rust`.

### Consumes

The stencil record (`store/stencil/record.txt.gz`), the case files and Windows results, the fixture.

### Invariants

- No Python file added or changed.

## artifact contract

### Files created

- `crates/translator/` (`Cargo.toml`, `src/lib.rs`, `src/lift.rs`, `src/rules.rs`, `src/lower.rs`, tests)

### Files modified

- `Cargo.toml`, `Cargo.lock`, `crates/w2m_vm/` (the case runner's Rust compiler), `harness/` (the Rust record's run
  file), `WORKING_AGREEMENT.md`, `BLACKBOARD.md` (Sprint 150's CI run, in the tail entry)

### Content assertions

- Each function cites the Python lines it carries.

### Command exit codes

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` return 0
- `scripts/gates.sh` returns 0

## observation contract

- **Expected values, from the C emitter:** for every record call whose iclass is in group 1 and for which the C emitter
  gives words, the Rust compiler's words at the record's address, exit and symbols, with the record's three flag
  arguments, equal the C's word for word. The C stencils were derived from the frozen Python lowering, so equality is
  the expectation; the count compared and the count equal are reported.
- **Expected values, from Windows:** the case runner with the Rust compiler for group 1's classes (refusing the rest,
  each refusal counted by class) judges the same group-1 cases the Python words judge, with zero difference classes.
- **Expected values, carried:** the Python tests' values for the lifter and the group's lowering, by hand.
- **Planted faults:** one in the lifter (`_operand`'s immediate sign extension) and one in the lowering (`_set_flags`'s
  CF for `sub`, `lo` changed to `hs`): each makes the record differential and the case runner name differences.

## done criteria

The Rust translator lifts and lowers link 5's first group as the frozen Python does, word for word against the C
emitter, and its cases agree with Windows.

## self-review checklist

- [x] Scope names one verifiable property: group 1 translated in Rust, equal to the C and to Windows.
- [x] context_files cite the Python carried.
- [x] Emits: the Rust record's tags.
- [x] Artifact contract gradable by command.
- [x] Observation contract: two external judges, carried values, two planted faults.
- [x] One concept: the first group, with the lifter it needs.

## result
