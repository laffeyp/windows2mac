# Sprint 180 — The batch run's SSE lowering

```yaml
---
id: 180
status: done
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

## result

- **The count first:** each class's rule read from the frozen registry: CMPPS, CMPPD, the seven packed conversions and
  PMOVSXDQ through `_sse.py`'s `int_rule`, MOVHPD through `memory_only_rule`; their lowering at `_sse.py` 330-335
  (CMPPS and CMPPD join the MXCSR fast path), 336-339, 343-360, 400-424 with `_to_dwords` 491-528, and 425-426.
- **The slice:** `rules.rs` registers the eleven classes on `sse_int` and `sse_memory_only`; `lower.rs` gains
  `CMP_LOWERED`'s arm, PMOVSXDQ before the integer block, the conversions behind `unmasked_to_model`, `to_dwords`, the
  `SSE_050` list and its `hvc #10` arm, and CMPPS and CMPPD in `fp_class`; `CVT_INDEFINITE` is True inline, as the
  Rust's CVT*2SI arm already takes it.
- **The C words judge:** 1,185,189 calls compared in 239 classes, no refusal and no difference: the census's 848 SSE
  calls and PMOVSXDQ's 4 moved from "not lifted" to compared, nine classes, the census file changed by those nine rows.
  720 calls in 72 classes stay unlifted (the VEX 719 and one PEXTRW).
- **The case runner:** the Rust translator compiles 317 classes and 192,284 cases (from 306 and 189,268), each word
  equal to the Python's; the model now steps the newly lifted classes' cases, set 049 judging 19,679 (from 19,621) and
  set 050 10,166 (from 9,320), 904 cases in all, 0 difference classes. 17 legacy classes, 3,856 cases, stay counted
  to the model: CMPSD_XMM, CMPSS, MPSADBW, PSLLW and 13 more, which `_sse.py` sends to `hvc #10` whole or the Rust does
  not lift.
- **Plant 1** (CMPPS's LT lowered as LE): `c_words` failed, "148 CMPPS flags [0, 0, 0]: 0x1808139af 440fc2c701: rust
  [b9405390 d3473210 f100fe1f 54000040 d4000142 6e", with three more CMPPS rows. Restored byte for byte.
- **Plant 2** (CVTDQ2PS's MXCSR check dropped): `c_words` failed, "192 CVTDQ2PS flags [0, 0, 0]: 0x1180033e52 0f5bc0:
  rust [4e21d800] c [b9405390 d3473210 f100fe1f 5400004". Restored byte for byte.
- **Records:** four `case-runner-rust` records (1793197269611612000, -638024000, -666181000, -697380000): 236, 177, 107
  and 26 class verdicts, the first one more than Sprint 178's, FCOS and FSIN rejected; `check_records` 0 violations
  each; the ledger holds 8 open entries, two for each record that reports the pair.
- **Edits:** every multi-line edit used the Edit tool; one-line `sed`s for the plants, two `CVT_INDEFINITE` lines and
  `expected.tsv`'s two rows.
