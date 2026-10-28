# Sprint 174 — The model's MMX slice

```yaml
---
id: 174
status: done
phase: 6
pass_kind: functional
---
```

## scope

Step 4's model (census row 4), its second slice. After Sprint 173 the case runner counts 21,556 model cases in 61
classes it does not judge; about 17,000 are MMX-register forms (MOVQ 2,600, PSRAW 1,416, PSRAD 1,362, PSRLQ 1,159, MOVD
1,039, the PUNPCK and PADD/PSUB/PMUL classes, PXOR 650, PACKUSWB 628, PSADBW 602, PMADDUBSW 513), the rest scalar SSE
with exceptions unmasked (112 to 180 a class).

The Python's MMX slice, carried:

- **The lifting, the model's own:** `lift.lift_mmx` (`src/w2m/lift.py` 304-324, with `rules/mmx.py`): an MMX-register
  instruction becomes one `("mmx", ic, dst, src, imm)` operation. The Python's VM lowers some MMX forms to A64
  (`rules/_mmx.py`) and steps the rest in the model; the C words judge holds every class the Rust lifter lifts to the
  C's words and fails on a Rust refusal (`tests/c_words.rs` 227), so `translator::lift::lift` stays as it is and the
  model lifts MMX forms through its own entry, `lift_mmx`, when `lift` refuses them. The A64 lowering is a later slice.
- **The model:** `runtime.py` `_mmx` (165-191) and `src/w2m/mmx.py` (121 lines), on the `Fpu` state of `src/w2m/x87.py`
  they use (the register file, TOP, the abridged tags; 497 on); a fault keeps TOP 0 and the tags (set 065b).
- **The state:** `_slow_sse`'s x87 transfer (vm_backend 4293-4298, 4315 `_store_x87`): FSW and the tags from the state
  block, ST(i) in FXSAVE order, in and out.

## prerequisites

- 173 (closed)

## context_files

- `src/w2m/rules/mmx.py`, `src/w2m/lift.py` 260-325, `src/w2m/mmx.py`, `src/w2m/runtime.py` 140-191,
  `src/w2m/x87.py` 497 on, `src/w2m/vm_backend.py` 4268-4323 (read in full before the code)
- `crates/w2m_vm/src/model.rs`, `crates/w2m_vm/src/cases.rs` (`model_step`), `crates/translator/src/lift.rs`

## signal contract

### Emits

- On four `case-runner-rust` records: SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED, CANDIDATE_ACCEPTED,
  SESSION_COMPLETED, now judging the MMX classes' model cases.

### Invariants

- No Python file added or changed; every case judged before is judged as before.

## artifact contract

### Files modified

- `crates/translator/src/lift.rs`, the translator's rules and lowering, `crates/w2m_vm/src/model.rs`,
  `crates/w2m_vm/src/cases.rs`, `store/case_runner/expected.tsv`, `WORKING_AGREEMENT.md`,
  `docs/step-4-census-2026-10-28.md` (row 4); four records under `store/runs/`

### Command exit codes

- the case runner, the C words judge and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the MMX classes' model cases judged against Windows with 0 difference classes, `expected.tsv`'s
  "to the model" falling by exactly the count stepped; the C words judge's words unchanged.
- **Planted faults:** (1) PSRAW's count saturating at 14 where the Python saturates at `bits - 1`, 15 (`mmx.py` 72; a
  saturation at 16 could not fail, a lane shifted by 16 or 15 being its sign either way): the case runner names PSRAW;
  (2) TOP left unchanged by an MMX instruction (the SDM sets it to 0): the case runner names the
  x87 status word.

## done criteria

The MMX classes' model cases are judged against Windows through the Rust model.

## self-review checklist

- [x] Scope names one verifiable property: the MMX model cases judged.
- [x] context_files cite the Python to read in full first.
- [x] Emits: the case runner's records.
- [x] Artifact contract gradable by command.
- [x] Observation contract: counts and two plants, each a fault on a path the judge reads (Sprint 173's lesson).
- [x] One concept: the MMX slice.

## result

- **The slice:** `crates/w2m_vm/src/model.rs` gains `lift_mmx` (lift.py 304-324, the model's own, since the translator
  refuses MMX forms and the C words judge allows no Rust refusal of a lifted class), `mmx_compute` and the MMX step
  (`mmx.py` whole, `runtime.py` `_mmx` 165-191) on the thread's x87 state (FSW, the abridged tags, R0-R7 at 80 bits),
  and `step_mmx`, whose guest fault restores what `_exec` restores and keeps the x87 state. The case runner carries the
  x87 state in from the state block and out after a step or a fault, as `_slow_sse` and `_store_x87` do
  (vm_backend 4293-4298, 4325-4330), and XMM in for MOVDQ2Q.
- **Judged against Windows:** 21,245 MMX cases the case runner had counted and not judged, with 0 difference classes:
  set 065b's 19,833 in 23 classes (MOVQ 2,600, MOVDQ2Q 1,584, PSRAW 1,416, PSRAD 1,362, PSRLQ 1,159, MOVD 1,039, EMMS
  861, the PUNPCK, PADD, PSUB and PMUL classes, PACKUSWB 628, PEXTRW 604, PMADDUBSW 513), every model case of that set;
  and set 065c's 1,412 (PSHUFW 810, PSADBW 602). With Sprint 173's slice the Rust model has stepped 24,766 cases.
  `store/case_runner/expected.tsv`'s two runs moved by exactly those counts.
- **Plant 1** (PSRAW and PSRAD saturating at `bits - 2`): the case runner failed, "40 PSRAW st0: case 14792: [0f, 71,
  e0, fe] model fffffffe000000000000, cpu ffffffff000000000000", and "18 PSRAD st0: case 13489". Restored.
- **Plant 2** (TOP left unchanged by an MMX instruction): the case runner failed, "861 EMMS FSW: case 0: [2e, 0f, 77]
  model 0x685e, cpu 0x405e (in 0x685e)", and "1023 MOVD FSW". Restored.
- **Left:** set 065's 53,349 model cases are x87 classes, and the scalar SSE classes with exceptions unmasked stay
  counted (the next slices); the translator's A64 lowering of MMX forms (`rules/_mmx.py`) is not ported.
