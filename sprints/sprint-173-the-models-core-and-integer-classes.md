# Sprint 173 — The model's core and its integer classes, in Rust

```yaml
---
id: 173
status: active
phase: 6
pass_kind: functional
---
```

## scope

Step 4's first component, the model (S4; Decision 2026-10-27; census row 4, `docs/step-4-census-2026-10-28.md`). The
model is the frozen Python's interpreter of lifted operations: `src/w2m/runtime.py` `_exec` and `_exec_ops` (995-1161),
with `integer_model.py` (157 lines) for the `x86` steps, `x86fp.py` for SSE, `x87.py` and `_mmx` for the rest. The VM
reaches it through hvc 10 (slow SSE) and hvc 11 (slow step); the Rust run loop ends at both (`run.rs` 553-554).

The case runner counts, by class, the cases it sends to the model and does not judge (2026-10-28, its run with output:
25,077 cases in 96 of 289 classes): 2,680 in `integer_model`'s classes (RCR 784, SHLD 276, SHRD 264, DIV 237, IDIV 219,
CMC 132, CLC 116, …), 562 in the locked forms whose operand is misaligned (hvc 11 from `lower.rs` 1375-1381), and 21,835
in MMX and SSE classes (MOVQ 2,600, PSRAW 1,416, PSRAD 1,362, …). Every slice runs on the interpreter's core, so this
sprint ports it with the integer classes:

- **The core:** `_exec` (a fault restores every register and flag but REP's completed iterations; 995-1013) and
  `_exec_ops`' integer and control ops: `imm`, `addr` (a segment-relative address refused as the Python refuses it),
  `ld`, `st`, `alu` (its 22 operations), `flags`, `set`, `sel`, `cst`, `cstz`, `cmov` (a 32-bit destination
  zero-extended), `br`, `df`, `pause`, `fence`, `ud`, `div`, `cx16`, `rep`, `cpuid`, `xgetbv`, `mxcsr`, `fcw`, with the
  helpers they call (`_load`, `_store`, `_set_flags`, `_cond`, `_flag`, `_div`, `_cmpxchg16b`, `_rep`), cited by line.
  An `sse`, `vex`, `vexs`, `fma`, `mmx`, `vzeroupper` or `x87` op is refused by name: the next slices.
- **The integer classes:** `integer_model.execute`, all 21 classes, line for line, with its Xeon-measured cases (the
  16-bit SHLD with a count of 17 to 31; the 32-bit destination written at a count of 0).
- **Where it runs:** the case runner serves its model cases for these classes with the Rust model and judges them
  against Windows, as it judges the rest; the run loop serves hvc 11 when the instruction's ops are all the core's, and
  ends at a named stop otherwise.

The Python tests' values carried (plan 132-133; census row 1): no Python test names `integer_model` (searched
2026-10-28: `tests/`, `scripts/`); its judge was the class runs against Windows, which the case runner is; the core's
values come from `tests/test_runtime.py` (5 tests), each Rust test citing the file and line it carries.

## prerequisites

- 172 (closed)

## context_files

- `src/w2m/runtime.py` 995-1161 and the helpers it calls; `src/w2m/integer_model.py` (read in full 2026-10-28)
- `crates/w2m_vm/src/cases.rs` (the model's cases, `Tally::model_by_class`), `crates/w2m_vm/src/run.rs` 437-554
- `tests/test_runtime.py`

## signal contract

### Emits

- On four `case-runner-rust` records: SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED,
  SESSION_COMPLETED, as before, now judging the integer classes' model cases.

### Invariants

- No Python file added or changed; every case judged today is judged as before.

## artifact contract

### Files created

- `crates/w2m_vm/src/model.rs` (or a `model` crate if the translator's `Op` is the shared type), its tests

### Files modified

- `crates/w2m_vm/src/cases.rs`, `crates/w2m_vm/src/run.rs`, `WORKING_AGREEMENT.md`; four records under `store/runs/`

### Command exit codes

- the case runner and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the 3,242 integer and misaligned-lock model cases judged against Windows with 0 differences;
  the case runner's "to the model" count falls by them; the MMX and SSE model cases stay counted, not judged.
- **Planted faults:** (1) RCR's count taken mod 8 for 8 bits where the Python takes mod 9: the case runner names RCR;
  (2) a fault that does not restore the flags: a case faulting mid-instruction names its flags.

## done criteria

The integer and misaligned-lock model cases are judged against Windows through the Rust model.

## self-review checklist

- [x] Scope names one verifiable property: the integer model cases judged through Rust.
- [x] context_files cite the Python read in full and the Rust it enters.
- [x] Emits: the case runner's records.
- [x] Artifact contract gradable by command.
- [x] Observation contract: counts and two plants.
- [x] One concept: the model's core and its integer classes.
