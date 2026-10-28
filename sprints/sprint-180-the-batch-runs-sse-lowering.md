# Sprint 180 — The batch run's SSE lowering

```yaml
---
id: 180
status: active
phase: 6
pass_kind: functional
---
```

## scope

Decision "Sequence after Sprint 178" (reviewer-2's High 1 of its review of 175-177): the batch run's SSE gap is a
lowering gap. The C words census holds 848 SSE calls the Rust rules do not lift (`store/census/c-words-unlifted-
2026-10-28.tsv`, stage "not lifted"): CMPPS 393, CVTDQ2PS 283, CVTPS2PD 64, CVTPD2PS 63, CVTPS2DQ 31, CVTTPS2DQ 11,
MOVHPD 2, CVTPD2DQ 1. The Python lifts each through `rules/_sse.py`'s `rule` and lowers it natively: `CMP_LOWERED`
(113), the packed conversions under `native_conversions` (400-416, `_to_dwords` 491-), PMOVSXDQ (334-337), MOVHPD;
each leaves for `hvc #10` only while MXCSR unmasks an exception (`_unmasked_to_model`, 23-28).

- **The count first:** each class's lifting rule and lowering lines read from `_sse.py` before the code, and the
  census's eight classes listed with the Python lines that lower them.
- **The lifting:** the Rust `sse` rule takes these classes as `_sse.py`'s `rule` does.
- **The lowering:** `lower_sse` lowers them as `_sse.py`'s `lower` does, word for word with the C emitter.
- **The model's cases:** the classes among Sprint 178's 27 that these lift go to the model's SSE step, judged by the
  case runner.

## prerequisites

- 179 (closed)

## context_files

- `src/w2m/rules/_sse.py` (whole), `crates/translator/src/rules.rs` (`sse`, `SSE_FLOAT_CORE`),
  `crates/translator/src/lower.rs` (`lower_sse`), `crates/translator/tests/c_words.rs`

## signal contract

### Emits

- On four `case-runner-rust` records: the verdicts Sprint 178's records hold, with the newly judged classes accepted.

### Invariants

- No Python file added or changed; every call compared before compares as before.

## artifact contract

### Files modified

- `crates/translator/src/rules.rs`, `crates/translator/src/lower.rs`, `store/census/c-words-unlifted-2026-10-28.tsv`
- `store/case_runner/expected.tsv`, `expected-by-class.tsv`, `rust-compiled-by-class.tsv`,
  `rust-compiled-codes-by-class.tsv`, `WORKING_AGREEMENT.md`, `docs/step-4-census-2026-10-28.md`; four records

### Command exit codes

- `c_words`, the case runner and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the census's 848 calls in 8 classes move from "not lifted" to compared with no difference; the
  case runner's judged counts rise by the cases those classes now lift.
- **Planted faults:** each checked against the Python's code before this card's commit: (1) CMPPS's predicate 1 (LT)
  lowered as 2 (LE): `c_words` names a CMPPS word difference; (2) CVTDQ2PS's unmasked check dropped: `c_words` names
  CVTDQ2PS. Checked: the record's CMPPS calls without a REX prefix use predicate 1 (LT) 118 times and 2 (LE) 70
  times, and `_sse.py` 352-355 lower them as two different FCM forms; CVTDQ2PS's 283 calls each emit the MXCSR check
  (`_unmasked_to_model`, 404) before SCVTF.

## done criteria

The batch run's SSE calls the Rust rules did not lift are lifted and lowered word for word with the C.

## self-review checklist

- [x] Scope names one verifiable property: the 848 calls compared.
- [x] context_files cite the Python to read in full first.
- [x] Emits: the case runner's records.
- [x] Artifact contract gradable by command.
- [x] Observation contract: counts and two plants.
- [x] One concept: the batch run's SSE lowering.
