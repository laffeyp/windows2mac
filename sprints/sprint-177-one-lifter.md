# Sprint 177 — One lifter

```yaml
---
id: 177
status: done
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

## result

- **One lifter:** `translator::lift` gains `Op::Mmx` and `Op::X87`, `MmxArg`, a public `MMX_ONLY` and `lift_mmx`
  (lift.py 307-324), called where `lift.py` 345-347 calls it, in place of the refusal "the MMX forms are a later
  group"; `rules.rs` gains `X87_ICLASSES` and the x87 rule (`rules/x87.py` 19-30); `commit_order` treats the two ops
  as lift.py 266-271 does. `model.rs` loses `lift_mmx`, `lift_x87`, `MmxStep`, `X87Step`, `step_mmx`, `step_x87` and
  its own `MMX_ONLY`: `exec_ops` steps `Op::Mmx` and `Op::X87`, and `step` keeps the x87 state a fault leaves for an
  MMX form, as `_mmx`'s `finally` does. `model_step` lifts once, with `lift`.
- **The lowering:** `Op::X87` compiles as `vm_backend.py` 1663-1666 does: `_x87_fast` (1349-1400) for FLD m64, FSTP
  m64 and FSQRT when the runtime defines `w2m_x87_fld64` (`RuntimeCalls.x87_fast`), the model step otherwise.
  `Op::Mmx` is refused by name as a later group; `Op::X87` sits with the model's steps in FLAGS_ALL_READ and `Op::Mmx`
  in FLAGS_NONE (859), and both read their memory addresses as `_temps_read` does (1043-1046).
- **The C words judge:** 1,184,337 calls compared in 230 classes, no refusal and no difference: 24 more calls and five
  more classes (FLD, FSTP, FCOS, FSIN, FSQRT) than Sprint 176's 1,184,313 in 225. The census file names each row's
  stage: "not lifted", 81 classes and 1,572 calls; "lowering refused", 20 MMX classes and 337 calls; 1,933 in all, as
  before. Two corrections to the judge itself came first: the Rust was given no `w2m_x87_*` addresses while the C was
  given its table's names at 0, and at address 0 `mov_imm` is one word where the C stencil's branch past the call is
  fixed for three (the Python's own words, `build/case_runner/python-words.txt` for `d9fa`, branch by `b4000531` with
  a three-word address). Both compilers now get one table, a placeholder `0x7F01_0001_1000 + 0x100 * i` for a name the
  record's header lacks.
- **The case runner:** its counts and `expected-by-class.tsv` unchanged; the Rust translator compiled 189,268 cases in
  306 classes, x87 among them, each word equal to the Python's; the hybrid compiler takes the Python's words for a
  lowering refused as a later group, as for a class the Rust does not lift.
- **Plant 1** (`lift_mmx` swapping PEXTRW's operands): the case runner failed, "86 PEXTRW r10: case 9606: [67, 4f, 0f,
  c5, d5, 2d] vm 0xff, cpu 0x5a9", with PEXTRW r0, r2, r11 and r12. Restored byte for byte.
- **Plant 2** (FSTP m64's store moved after its commit call): `c_words` failed, "2 difference classes", the first "5
  FSTP flags [0, 0, 0]: 0x1180ccfd35 dd5c24f8". Restored byte for byte.
- **Records:** four `case-runner-rust` records (1793191799691580000, -719220000, -749824000, -780988000) hold the
  verdicts Sprint 175's held: 235, 177, 107 and 26, FCOS and FSIN rejected on the first; `check_records` 0 violations
  each. `case_runs --write-ledger` added the first record's two reports as open entries, 4 in all.
- **Departures from the card:** `crates/w2m_vm/src/case_inputs.rs` (the hybrid's fallback) and
  `store/case_runner/discrepancy-ledger.json` changed, files the card did not name; the card's plant 2 at first read
  "the lowering compiling `Op::X87` as a no-op", redrawn in the scratchpad before the card's commit once `_x87_fast`
  was read. Every multi-line edit used the Edit tool; one-line `sed`s placed plant 1 and the imports.
