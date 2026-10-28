# Sprint 160 — The SSE float core: link 5's fourth group, lifted and lowered in Rust

```yaml
---
id: 160
status: active
phase: 6
pass_kind: functional
---
```

## scope

Link 5's fourth group (`docs/design-host-compile-path-2026-10-24.md`): the 52 classes whose rule is
`src/w2m/rules/_sse.py`'s `rule` (one module each, `store/rules/registry.tsv`), all lowered in `_sse.lower`'s own body,
none by `_lower_int`. After Sprint 159 the stencil record's calls the Rust rules do not lift are 89,584; measured by the
C words judge's new census (decoded class of each call no Rust rule lifts), this group holds the largest share:
MOVAPS 21,645, MOVSS 15,536, MOVUPS 8,498, MOVSD_XMM 4,784, MULSS 3,287, MULPS 3,134, CVTSS2SD 2,719, CVTSD2SS 2,389,
ADDPS 2,139, ADDSS 1,872, XORPS 1,776, COMISS 1,026.

- **The classes:** the moves MOVAPS MOVAPD MOVUPS MOVUPD MOVSS MOVSD_XMM MOVD MOVQ; the logic XORPS XORPD ANDPS ANDPD;
  ADD SUB MUL DIV MIN MAX SQRT in SS SD PS PD (28); the conversions CVTSS2SI CVTSD2SI CVTTSS2SI CVTTSD2SI CVTSI2SS
  CVTSI2SD CVTSS2SD CVTSD2SS; the compares COMISS COMISD UCOMISS UCOMISD.
- **The lift:** `_operand`'s XMM register (`lift.py` 177-178, ("x", n)); `_sse.rule` (`_sse.py` 32-39) as one `sse`
  op, its alignment 16 when a memory operand meets `SSE_ALIGNED` (`lift.py` 97-98); the op's places in `commit_order`
  (`lift.py` 263-264: a store when the destination is memory, else a load when the source is; an effect when the
  destination is not memory).
- **The lowering:** `_sse.lower` (`_sse.py` 324-488) for these classes: the alignment check (78-86), the MXCSR
  fast-path check for `FP_CLASSES` (`vm_backend.py` 1114-1116; `_sse.py` 330-335), `_sse_src` (70-75), the moves,
  logic, arithmetic, conversions (the integer indefinite's fix-up, `CVT_INDEFINITE` True) and `_set_flags_from_fcmp`
  (89-97); and the op's places in `_op_flags` (`FLAGS_SSE` for the four compares, 856), `_temps_read` (1015-1016) and
  `OPERAND_SAFE` (outside it).

Out of scope: `int_rule`'s 103 modules (the SSE2 integer block, `_lower_int`), the memory-only and register-only
rules, VEX, MMX and x87.

## prerequisites

- 159 (closed)

## context_files

- `src/w2m/rules/_sse.py` (read in full); `src/w2m/lift.py` 97-141, 175-214, 246-290; `src/w2m/vm_backend.py`
  855-889, 1013-1060, 1114-1135
- `crates/translator` (Sprints 151, 153, 159); `crates/translator/tests/c_words.rs`, `crates/w2m_vm/tests/case_runner.rs`

## signal contract

### Emits

- The `case-runner-rust` sessions, four reference profiles (`harness/case-runs-rust.toml`): SESSION_INIT,
  CANDIDATE_GENERATED, COMPARISON_COMPLETED and CANDIDATE_ACCEPTED per class, with this group's classes present in the
  cases now the Rust translator's candidates; SESSION_COMPLETED completed. The judges' ledger stays empty.

### Consumes

The stencil record, the case files and Windows results, the Python words fixture.

### Invariants

- No Python file added or changed.
- Sprints 151-159's judges pass unchanged.

## artifact contract

### Files modified

- `crates/translator/` (`lift.rs`, `rules.rs`, `lower.rs`; `tests/c_words.rs`, its census of unlifted calls by class),
  `WORKING_AGREEMENT.md`; four records under `store/runs/`

### Content assertions

- Each function cites the Python lines it carries.

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values, from the C emitter:** every record call of groups 1 to 4 the C compiles has Rust words equal to
  the C's; this group's count is stated.
- **Expected values, from Windows:** the case runner with the Rust translator judges each run's committed counts with
  zero difference classes; this group's classes present in the cases join its compiled classes.
- **Expected values, from the Python words:** every case code the Rust rules lift has the fixture's words.
- **Planted faults:** (1) PF's lazy value after a COMIS or UCOMIS compare set from `vs` in place of `vc`
  (`_sse.py` 96): the case runner names a compare class's PF, the C judge their words; (2) the alignment check skipped for MOVAPS: the C judge
  names MOVAPS's words, the case runner a MOVAPS case Windows faulted on.

## done criteria

The 52 classes of `_sse.rule` are lifted and lowered in Rust, equal to the C and the Python, and agree with Windows
where the cases reach them.

## self-review checklist

- [x] Scope names one verifiable property: the fourth group in Rust, equal to the C, the Python and Windows.
- [x] context_files cite the Python carried, by line.
- [x] Emits: the sessions' tags, each named.
- [x] Artifact contract gradable by command.
- [x] Observation contract: three judges and two planted faults, each reaching the judge it tests.
- [x] One concept: the SSE float core, `_sse.rule`'s classes.
