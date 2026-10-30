# Sprint 236 — The MMX lowering

```yaml
---
id: 236
status: done
phase: 6
pass_kind: functional
---
```

## scope

Plan step 4's "456 of 458 classes" and the remainder of Sprint 235: the three MMX-only classes (EMMS, MOVDQ2Q,
PSHUFW) the class judge counts as conforming only on the frozen Python's words, and every MMX-register form the batch
run's calls hold.

- **Where it stands, by lookup (2026-10-30):**
  - The Rust lift has `lift_mmx` (`crates/translator/src/lift.rs` 709-741), one `Op::Mmx` per MMX instruction, and
    the Rust model steps it (`crates/w2m_vm/src/model.rs` 642-643).
  - The Rust lowering refuses `Op::Mmx` by name as a later group (`lower.rs` 1957-1964), so the case runner takes
    the fixture's Python words for every MMX form.
  - `store/census/c-words-unlifted-2026-10-28.tsv` lists 20 MMX classes whose lowering is refused in the batch run's
    calls, 337 calls: MOVQ 176, MOVD 33, PADDW 22 and 17 more.
  - The frozen lowering is `src/w2m/rules/_mmx.py` at the tag frozen-python-2026-10-29, 121 lines. A form runs in A64
    when its operands are MMX registers and memory, x87's TOP is 0 and no x87 exception is pending. Every other form,
    and any that names an XMM register, is a model step (`hvc` #11).
- **The work:** `_mmx.py` ported whole into `lower.rs`, its native forms as written and the rest as model steps, so the
  Rust words equal the Python's for every MMX case code, and the C words judge compares the batch run's MMX calls.
- **Not here:** VRCPPS and VRSQRTPS; retiring the Python words fixture.

## prerequisites

- 235 (closed)

## context_files

- `src/w2m/rules/_mmx.py` and `src/w2m/mmx.py` at the tag
- `crates/translator/src/lower.rs`, `crates/translator/src/lift.rs` (`lift_mmx`), `crates/w2m_vm/src/model.rs`
- `crates/translator/tests/c_words.rs`, `store/census/c-words-unlifted-2026-10-28.tsv`,
  `crates/w2m_vm/tests/case_runner.rs` and its held files

## signal contract

### Emits

- The class judge's session again, as in Sprint 235.

### Invariants

- Every case code the Rust translator compiles has the frozen Python's words; every judged case agrees with Windows.

## artifact contract

### Files modified

- `crates/translator/src/lower.rs`, the census, the case runner's held files, the class judge's index and record,
  census row 3

### Command exit codes

- `scripts/gates.sh` returns 0; the class judge's session completes and `check_records` passes it

## observation contract

- **Expected values:** the 20 MMX classes off the census; EMMS, MOVDQ2Q and PSHUFW compiled by the Rust translator;
  classes conforming on Rust-compiled words of the 456.
- **Planted faults:**
  1. The native path taken with TOP not 0: the case runner names an x87-state case.
  2. One lane operation's NEON name changed: the words test names the code.
  3. A form that names an XMM register run natively: the words test names it.

## done criteria

Every MMX form the cases and the batch run hold is compiled by the Rust translator, with the frozen Python's words.

## self-review checklist

- [x] Scope names one verifiable property: every MMX form compiled by the Rust translator with the frozen words.
- [x] context_files cite the frozen lowering, the Rust lift, lowering and model, and the judges.
- [x] Emits: the class judge's session.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the MMX lowering.

## result

- **The port:** `lower_mmx` in `crates/translator/src/lower.rs` is `_mmx.lower` (rules/_mmx.py 54-121 at the tag),
  with `mmx_native` (36-51), `MMX_LANES`, `MMX_SHIFTS` and `mmx_source_form`. `Op::Mmx` calls it where it was refused
  as a later group. Every encoding it needs was in the Rust encoder.
- **The offsets:** the x87 status word's and slots' offsets (158, 768) had two homes in `w2m_vm` and one in
  `guest_runtime`. `guest_runtime::layout` now holds both, and `cases.rs` re-exports them. The translator, which does
  not depend on `guest_runtime`, keeps its own copies beside `MXCSR_OFF`, held equal to the layout by the new
  `tests/layout.rs`.
- **The batch run:** the C words judge compares the MMX calls too. The census of unlifted and unlowered classes is
  empty: 1,186,246 of the 1,186,511 calls are compared, in 313 classes, with no difference and no refusal.
- **The cases:**
  - The Rust translator compiles every case code, 25,919 of 25,919 in 458 classes, each with the frozen Python's
    words. The two held lists of Rust-compiled classes were rewritten by the test's writer.
  - Every judged case agrees with Windows but set 065's four FCOS and FSIN rows, held since Sprint 222. The expected
    counts did not move: the MMX cases were judged before, on fixture words.
- **The session:** record `store/runs/class-judge-1793357669000000000`, `check_records` 0 violations, 454
  CANDIDATE_ACCEPTED. The index was written from it: 454 of 458 conform, every one on Rust-compiled words, 2 held. The
  judges' ledger was rebuilt by `case_runs --write-ledger`, 4 defects.
- **Plant 1** (the TOP check narrowed to ES, `ubfx(17, 17, 7, 1)`): the words test named "PMADDUBSW [0f, 38, 04, 74, 8f, d8]: rust [... d3471e31 ...]" against the Python's.
- **Plant 2** (PADDW's lane operation "sub" for "add"): "PADDW [0f, fd, 58, ba]: rust [... 2e7f87de ...]" against the Python's.
- **Plant 3** (a form that names an XMM register run natively): its first form, PEXTRW with an XMM source, reached no judge: that form lifts through the SSE path, not `lift_mmx`. Run again on MOVDQ2Q made native: "33 case codes whose Rust words differ from the Python's".
- Each plant was restored by `cp`, and `cmp` found `lower.rs` equal to its copy. The plants ran while the session ran; its binary had passed its start check and the plants rebuilt only the test profile.
- **Departures:** `crates/guest_runtime/src/layout.rs`, `crates/w2m_vm/src/cases.rs`, `crates/translator/tests/layout.rs`
  (new), the judges' ledger.
