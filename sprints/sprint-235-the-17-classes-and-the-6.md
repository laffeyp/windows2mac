# Sprint 235 — The 17 classes and the 6

```yaml
---
id: 235
status: active
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
