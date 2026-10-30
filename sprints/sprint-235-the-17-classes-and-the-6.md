# Sprint 235 — The 17 classes and the 6

```yaml
---
id: 235
status: done
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 214" as annotated at Sprint 234, plan step 4's "456 of 458 classes", and
BLACKBOARD Deferred "(Sprint 178, partial)".

- **Where it stands, by lookup (2026-10-30):**
  - The class judge (Sprint 222) holds 19 classes inconclusive (`store/case_runner/class-held-2026-10-29.tsv`).
    - 17 are legacy SSE classes the Rust translator does not lift: CMPSD_XMM, CMPSS, MPSADBW, PABSW, PALIGNR, PAVGB,
      PCMPGTW, PINSRB, PINSRD, PMADDWD, PMAXSW, PMAXUD, PMAXUW, PMINSD, PMOVZXWD, PSLLW, SHUFPD (3,856 cases).
    - VRCPPS and VRSQRTPS, the model's approximations (1,258 cases), wait for their own card.
  - 6 classes conform only on the frozen Python's words (EMMS, LDDQU, MOVDQ2Q, ORPD, PEXTRW, PSHUFW): no case of
    theirs is compiled by the Rust translator.
  - The frozen registry (`store/rules/registry.tsv`) gives each class its module. Read at the tag
    frozen-python-2026-10-29:
    - the 17, LDDQU and ORPD set `rule = _sse.int_rule`, `lower = _sse.lower`;
    - PEXTRW sets `_sse.register_only_rule`;
    - EMMS, MOVDQ2Q and PSHUFW are `mmx`'s, lifted by `lift_mmx`.
  - The Rust translator has `sse_int`, `int_rule`'s port (`crates/translator/src/rules.rs`). Its `rule()` maps a
    class to it only from `SSE2_INT_BLOCK`, which holds none of the 17.
  - Its lowering already sends all 17 to the model, `hvc` HVC_SLOW_SSE (`lower.rs` 2505-2507, `SSE_050`).
  - The lift knows the MMX-only classes (`lift.rs` 706, `MMX_ONLY`) and PEXTRW and PSHUFW (728). Why the 6 compile
    from fixture words only is not yet known, and the class judge's session will show it.
- **The work:**
  - The 17, LDDQU and ORPD mapped to `sse_int` in `rule()`, as the registry maps them; PEXTRW to `register_only_rule`'s port
    (`_sse.py` 62-67, five lines the Rust rules lack: a memory operand is #UD, else `int_rule`).
  - The cause of the 6 read from the session, and fixed where it is the translator's.
  - The class judge run again (`harness/class-judge.toml`), its index written, and the held file and census row 3
    moved by what it shows.
- **Not here:** VRCPPS and VRSQRTPS; retiring the Python words fixture, if any class still needs it.

## prerequisites

- 234 (closed)

## context_files

- `src/w2m/rules/_sse.py` and the 23 classes' modules at the tag; `store/rules/registry.tsv`
- `crates/translator/src/rules.rs` (`rule`, `sse_int`), `lift.rs`, `lower.rs`
- `harness/class-judge.toml`, `crates/w2m_vm/src/bin/class_judge.rs`, `store/case_runner/class-held-2026-10-29.tsv`,
  `docs/step-4-census-2026-10-28.md` (row 3)

## signal contract

### Emits

- The class judge's session, as in Sprint 222: per class CANDIDATE_GENERATED and COMPARISON_COMPLETED, then
  CANDIDATE_ACCEPTED, or REDUCTION_COMPLETED, DISCREPANCY_REPORTED and DISCREPANCY_ROUTED per difference; one
  SESSION_INIT and one SESSION_COMPLETED.

### Invariants

- The 431 classes that conform on Rust-compiled words still conform; FCOS and FSIN stay discrepancies, as in the
  frozen index.

## artifact contract

### Files modified

- `crates/translator/src/rules.rs`, the class judge's index and record, the held file, census row 3

### Command exit codes

- `scripts/gates.sh` returns 0; the class judge's session completes, and `check_records` passes it

## observation contract

- **Expected values:** classes conforming on Rust-compiled words, against 431, of the 456 the frozen index accepts;
  each class that does not, with its first difference.
- **Planted faults:**
  1. One of the 17 left out of the mapping: the session names it inconclusive again.
  2. A lifted class lowered by the wrong rule in a scratch copy: the case runner names a case.
  3. The held file keeping a class that now conforms: `class_judge.rs` names it.

## done criteria

The 17 legacy classes and the 6 are compiled by the Rust translator and judged on their own cases.

## self-review checklist

- [x] Scope names one verifiable property: the 23 classes judged on Rust-compiled words.
- [x] context_files cite the frozen rules, the translator, the judge and the held file.
- [x] Emits: the class judge's session, as Sprint 222.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the classes the Rust translator does not compile.

## result

- **The rules:** `SSE_INT_MODULES` holds the 17, LDDQU and ORPD, mapped in `rule()` to `sse_int`, as the registry maps
  each to its own module whose rule is `_sse.int_rule`. PEXTRW maps to `sse_register_only`, `register_only_rule`'s
  port (`_sse.py` 62-67): a memory operand is #UD, else `int_rule`.
- **The lowering:** PEXTRW gets its native arm (`_sse.py` 361-367): `umov_h` of word imm & 7, stored into the general
  register. LDDQU joins MOVAPS's arm as a 16-byte load (368). The 17 go to the model through the existing `SSE_050`
  arm.
  - PEXTRW first lowered to the model, through `SSE_050`, and the C words judge named the batch run's one call:
    "rust [d4000142] c [0e023c30 2a1003e2]". With the native arm the words are equal.
- **The words:** for every case code the Rust translator compiles, now 25,437 of 25,919 in 455 classes, its words
  equal the frozen Python's.
- **The cases:** every case of sets 049, 050 and 065c is judged, where 367, 2,667 and 822 went to the model unjudged.
  All agree with Windows. The four FCOS and FSIN differences of set 065 are those held since Sprint 222.
  - `expected.tsv`'s three rows were rewritten by the Edit tool; `expected-by-class.tsv`,
    `rust-compiled-by-class.tsv` and `rust-compiled-codes-by-class.tsv` by the tests' own writers.
  - The batch run's census (`store/census/c-words-unlifted-2026-10-28.tsv`) lost PEXTRW's "not lifted" row.
- **The session:** record `store/runs/class-judge-1793354895000000000`, `check_records` 0 violations: 458 classes,
  454 CANDIDATE_ACCEPTED, FCOS and FSIN two reports each. The index was written from it by `class_judge --index`.
  - 454 of 458 conform, against 437. 3 of them, EMMS, MOVDQ2Q and PSHUFW, conform on no Rust-compiled case.
  - The held file keeps VRCPPS and VRSQRTPS.
- **A test that could not fail:** `the_rust_index_is_the_frozen_one_or_held` collected its faults and never asserted
  them, from Sprint 222. With the 17 still held and now conforming, it passed. It asserts them now, and its first run
  named exactly the 17.
- **Departure, the scope split:** EMMS, MOVDQ2Q and PSHUFW need the MMX lowering, `_mmx.py` at the tag, 121 lines.
  Every MMX form of the batch run's 337 calls in 20 classes waits on the same port. That is its own concept, card
  236, not this card's.
- **Departures, files:** `crates/translator/src/lower.rs`, `crates/w2m_vm/tests/class_judge.rs`, the case runner's
  four held files and the batch run's census, outside the card's file list.
- **Plant 1** (PAVGB taken out of `SSE_INT_MODULES`): the words test, the fast judge used in place of a whole session, named "PAVGB: held Some(26), now None".
- **Plant 2** (PEXTRW's lane taken as imm & 3): "PEXTRW [2e, 66, 0f, c5, c2, 7f]: rust [0e0e3c50 2a1003e0] false, python [0e1e3c50 2a1003e0] false".
- **Plant 3** (CMPSS put back in the held file): "CMPSS: conforms as the frozen index, and store/case_runner/class-held-2026-10-29.tsv holds it".
- Each plant was restored by `cp`, and `cmp` found the three files equal to their copies.
- **The first gates run failed** in `ledger.rs`, "a report is in no entry", 20 against 24: the new class-judge record
  added four reports. `case_runs --write-ledger` rebuilt the ledger, 4 defects with 24 reports. The second run passed
  in 500 s.
- **Reviewer-3's review at 235:**
  - *Medium 1* (222's gate could not fail): card 222's result and its Signal Report are annotated, and
    WORKING_AGREEMENT's plant rule now requires a plant to run on the committed tree. clippy's
    `collection_is_never_read` was tried on the test with its assertion removed and named "collection is never read" at
    `let mut faults` (line 100). The workspace is clean under it, and the gates' and CI's clippy now deny it.
  - *Low 1* (an `echo` hid the gates' status): a line in WORKING_AGREEMENT.
  - The lint and the gates change made the gates run a third time.
  - Departures: `scripts/gates.sh`, `.github/workflows/ci.yml`, card 222 and its Signal Report.
