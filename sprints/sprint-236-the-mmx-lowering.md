# Sprint 236 — The MMX lowering

```yaml
---
id: 236
status: active
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
