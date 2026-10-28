# Sprint 177 — One lifter

```yaml
---
id: 177
status: active
phase: 6
pass_kind: functional
---
```

## scope

reviewer-2's Medium 1 of 2026-10-28, and its repeat in Sprint 175. The Python has one lifter: `lift.py`'s `lift`
emits `("mmx", ic, dst, src, imm)` through `lift_mmx` (307-324) and `("x87", ic, opcode, reg, st, mem)` through
`rules/x87.py`, and both the model and the lowering read those ops. The Rust has the translator's `lift`, which
refuses MMX forms (`lift.rs` 626-629, "the MMX forms are a later group") and has no rule for the 17 x87 classes, and
two more lifters in `crates/w2m_vm/src/model.rs`: `lift_mmx` (Sprint 174) and `lift_x87` (Sprint 175). `MMX_ONLY` is
defined twice (`lift.rs` 597, `model.rs` 998).

- **The ops:** `Op::Mmx { ic, dst, src, imm }` and `Op::X87 { ic, opcode, reg, st, mem }` in `translator::lift`, emitted
  by `lift` as `lift.py` emits them; `commit_order` (lift.py 246-301) gives them the Python's load and store.
- **The lowering:** `Op::X87` lowers as `vm_backend.py` 1663-1666 lowers it: `_x87_fast` (1349-1400, ported: FLD m64,
  FSTP m64 and FSQRT call the runtime's `w2m_x87_fld64`, `w2m_x87_st0_double`, `w2m_x87_fstp_commit` and
  `w2m_x87_st0_exact` with x0-x15 saved round each call, then `hvc #11` where the C answers 0) when the runtime defines
  the symbols, `hvc #11` otherwise. `Op::Mmx` (`rules/_mmx.py`, lowered in A64 at 1658-1660) is refused by name until
  its group; `c_words` counts those calls in its new bucket.
- **The model:** consumes the two ops from `lift`'s `Lifted`; `lift_mmx`, `lift_x87`, `MmxStep`, `X87Step` and the
  model's `MMX_ONLY` go; `model_step` no longer falls through to a second lifter on any `lift` error.
- **The C words judge:** the calls that now lift and are refused by the lowering by name go to a bucket "lifted,
  stepped by the model", held to Sprint 176's file of unlifted classes (the classes leave that file in this sprint, by
  an edit).

## prerequisites

- 176 (closed)

## context_files

- `src/w2m/lift.py` 246-355, `src/w2m/rules/x87.py`, `src/w2m/vm_backend.py` 1349-1400 and 1655-1666, `crates/translator/src/lift.rs`, `crates/translator/src/rules.rs`
- `crates/translator/src/lower.rs` (the `Op::Sse` arms, 1738-1810), `crates/translator/tests/c_words.rs`
- `crates/w2m_vm/src/model.rs`, `crates/w2m_vm/src/cases.rs` (`model_step`)

## signal contract

### Emits

- On four `case-runner-rust` records: the same class verdicts as Sprint 175's records, unchanged by the move.

### Invariants

- No Python file added or changed; the case runner's counts and `expected-by-class.tsv` unchanged; the Rust words
  equal the Python's on every case code the Rust lifts.

## artifact contract

### Files modified

- `crates/translator/src/lift.rs`, `crates/translator/src/rules.rs`, `crates/translator/src/lower.rs`,
  `crates/translator/tests/c_words.rs`, `store/census/c-words-unlifted-2026-10-28.tsv`
- `crates/w2m_vm/src/model.rs`, `crates/w2m_vm/src/cases.rs`, `WORKING_AGREEMENT.md` (rows 82 and 84); four records

### Command exit codes

- `c_words`, the case runner and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** the case runner's counts and per-class counts equal Sprint 176's; `c_words`'s unlifted map
  loses exactly the MMX and x87 classes and its new bucket holds them with the same call counts.
- **Planted faults:** (1) `lift` giving PEXTRW's operands in the other order: the case runner names PEXTRW; (2) FSTP m64's
  store moved after its commit call (1386-1387 store first): `c_words` names a word difference for FSTP. Each checked against the Python's code before this card's commit.

## done criteria

One Rust lifter emits the MMX and x87 ops, and the model and the lowering both read them from it.

## self-review checklist

- [x] Scope names one verifiable property: one lifter.
- [x] context_files cite the Python lifter and every Rust lifter.
- [x] Emits: the case runner's records, unchanged.
- [x] Artifact contract gradable by command.
- [x] Observation contract: counts unchanged and two plants.
- [x] One concept: one lifter.
