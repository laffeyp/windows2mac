# Sprint 161 — The SSE2 integer block: link 5's fifth group, lifted and lowered in Rust

```yaml
---
id: 161
status: active
phase: 6
pass_kind: functional
---
```

## scope

Link 5's fifth group (`docs/design-host-compile-path-2026-10-24.md`): the 77 classes whose rule is
`src/w2m/rules/_sse.py`'s `int_rule` (or `memory_only_rule`, which falls to it) and whose lowering is `_lower_int`
(`SSE2_INT_LOWERED`, `_sse.py` 100-126), counted from `store/rules/registry.tsv` and the sets as written. By the C words
judge's census after Sprint 160, SHUFPS 2,841, MOVDQA 2,528, PSHUFD 1,161, UNPCKLPS 574, MOVDQU 447, PXOR 372, ORPS
341, ANDNPS 311, PSUBUSB 226 lead.

- **The classes:** the moves MOVDQA MOVDQU MOVNTPS MOVNTDQ MOVHPS MOVLPS MOVHLPS MOVLHPS; the logic PAND PANDN POR PXOR
  ORPS ANDNPS; the lane arithmetic, compares, unpacks and packs of `LANE_LOWERED` and `_lower_int`'s own branches; the
  shifts (`SHIFTS`, PSRLQ, PSRLDQ, PSLLDQ); the shuffles PSHUFD PSHUFLW PSHUFHW SHUFPS PSHUFB; PINSRW, PMOVMSKB,
  MOVMSKPS, CVTDQ2PD, PSADBW, PMADDUBSW, PMULHW; and PCMPISTRI, PCMPISTRI64, RCPPS, RSQRTPS, RCPSS, RSQRTSS, which
  `_lower_int` sends to the model (`hvc #10`).
- **The lift:** `int_rule` (`_sse.py` 42-51): an immediate when the template's last operand is one, the destination as
  the source for the shift-by-immediate forms; `memory_only_rule` (54-59): `ud` for MOVLPS, MOVHPD, MOVNTDQ with only
  registers; the `SSE_ALIGNED` additions for these classes (`lift.py` 104-126).
- **The lowering:** `_lower_int` (`_sse.py` 129-321) with `LANE_LOWERED` and `SHIFTS` (103-115), reached from
  `_sse.lower` (340-342).

Out of scope: CMPPS, CMPPD, the packed conversions and `_to_dwords`, PEXTRW, PMOVSXDQ, LDDQU, ORPD, and the 18 classes
`_sse.lower` sends whole to the model (Sprint 162); VEX, MMX, x87.

## prerequisites

- 160 (closed)

## context_files

- `src/w2m/rules/_sse.py` (read in full in Sprint 160); `src/w2m/lift.py` 97-141
- `crates/translator` (Sprints 151-160); `crates/translator/tests/c_words.rs`, `crates/w2m_vm/tests/case_runner.rs`

## signal contract

### Emits

- The `case-runner-rust` sessions, four reference profiles (`harness/case-runs-rust.toml`): SESSION_INIT,
  CANDIDATE_GENERATED, COMPARISON_COMPLETED and CANDIDATE_ACCEPTED per class, with this group's classes present in the
  cases now the Rust translator's candidates; SESSION_COMPLETED completed. The judges' ledger stays empty.

### Consumes

The stencil record, the case files and Windows results, the Python words fixture.

### Invariants

- No Python file added or changed.
- Sprints 151-160's judges pass unchanged.

## artifact contract

### Files modified

- `crates/translator/` (`lift.rs`, `rules.rs`, `lower.rs`), `WORKING_AGREEMENT.md`; four records under `store/runs/`

### Content assertions

- Each function cites the Python lines it carries.

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values, from the C emitter:** every record call of groups 1 to 5 the C compiles has Rust words equal to
  the C's; this group's count is stated.
- **Expected values, from Windows:** the case runner with the Rust translator judges each run's committed counts with
  zero difference classes; this group's classes present in the cases join its compiled classes.
- **Expected values, from the Python words:** every case code the Rust rules lift has the fixture's words.
- **Planted faults:** (1) PSHUFD's lane selector read as `imm >> lane` in place of `imm >> 2*lane`: the C judge and the
  case runner name PSHUFD; (2) PSRLW's arithmetic flag set, so it shifts as PSRAW: both judges name PSRLW.

## done criteria

The 77 classes `_lower_int` lowers are lifted and lowered in Rust, equal to the C and the Python, and agree with
Windows where the cases reach them.

## self-review checklist

- [x] Scope names one verifiable property: the fifth group in Rust, equal to the C, the Python and Windows.
- [x] context_files cite the Python carried, by line.
- [x] Emits: the sessions' tags, each named.
- [x] Artifact contract gradable by command.
- [x] Observation contract: three judges and two planted faults, each reaching the judge it tests.
- [x] One concept: `_lower_int`'s classes.

## result

- `crates/translator`: `rules.rs` gains `SSE2_INT_BLOCK` (the 77 classes), `sse_int` (`int_rule`),
  `sse_memory_only` (`memory_only_rule`, for MOVLPS and MOVNTDQ) and the full `SSE_ALIGNED` (104 classes, read from the
  frozen `w2m.lift`), replacing Sprint 160's table of lift.py 97-98; `lower.rs` gains `lower_int` with `lane_lowered`
  and `shifts`, routed from `lower_sse`.
- **An amendment found by the C judge.** The first run named PCMPISTRI, RCPPS and RSQRTPS in 7 difference classes:
  `compile_insn` lowers them by calling the runtime's C (`w2m_pcmpistri`, `w2m_approximate`, `vm_backend.py`
  1790-1833) whenever the compiling VM's symbols hold them, and leaves them to the model only otherwise, as
  `_lower_int` does. `compile_insn` and `assemble_block` now take `RuntimeCalls` (the three functions the words call)
  in place of Sprint 159's `cpuid_va`; the two calls are carried with `save_guest_registers`, `call_runtime` and
  `load_guest_registers` and `NATIVE_OPERANDS`; the run loop, the case runner (`case_runtime_calls`) and the C judge
  pass the runtime's symbols.
- **The C emitter:** the Rust rules lift 1,184,578 of the record's 1,186,511 calls (1,173,060 before); the other
  1,184,313 the C compiles, in 225 classes (157 before), equal the C's words. This group's are 11,518 calls in 68
  classes, SHUFPS 2,841, MOVDQA 2,528, PSHUFD 1,161, UNPCKLPS 574, MOVDQU 447, PXOR 367, ORPS 341, ANDNPS 311 the most.
  No Rust refusal. The calls left are CMPPS, the packed conversions, the VEX forms, the MMX forms (MOVQ 176, MOVD 33,
  PADDW 22 with MM registers) and x87.
- **Windows:** the case runner with the Rust translator compiled 131,649 case instructions in 289 classes (110,401 in
  212 before); each run's counts equal `expected.tsv`, zero difference classes.
- **The Python words:** the Rust words equal the fixture's for all 15,716 case codes the Rust rules lift (13,032
  before), of 25,919.
- **The records:** four `case-runner-rust` sessions, `store/runs/case-runner-rust-1793165537000000000`,
  `-1793165579000000000`, `-1793165603000000000`, `-1793165617000000000`: 205, 174, 107 and 26 classes accepted, as
  before; the Rust translator the candidate for 93, 152, 107 and 26 (65, 129, 107 and 0 before); no discrepancy; each
  session completed. The fourth profile's 26 are SSE2 integer classes (MOVHPS, PACKUSWB, PADDQ, …): Sprint 153's
  result called that set "x87 cases alone", which was wrong.
- **Planted faults** (both at once, different classes). (1) PSHUFD's selector `imm >> lane`: the C judge named 433
  PSHUFD calls among others, the case runner PSHUFD cases by register ("PSHUFD xmm2: case 2943 …"). (2) PSRLW as an
  arithmetic shift: the C judge named "PSRLW … rust [4f180421] c [6f180421]", the case runner "PSRLW xmm6: case 1223 …
  vm …ffffffffffff, cpu …0000". Restored, every judge passes.
- The gates were started with a shell `&` in place of a background task, so their end was read from a file the run
  wrote, not from a notification.
