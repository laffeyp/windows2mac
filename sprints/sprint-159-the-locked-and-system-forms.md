# Sprint 159 — The locked and system forms: link 5's third group, lifted and lowered in Rust

```yaml
---
id: 159
status: active
phase: 6
pass_kind: functional
---
```

## scope

Link 5's third group (`docs/design-host-compile-path-2026-10-24.md`), the forms Sprint 153's card left out by name:
the `_LOCK` forms, CMPXCHG16B, the system ops, and the model's `x86` steps, carried into `crates/translator` with the
lowering their ops need. With it, every class `src/w2m/rules/_int.py` lifts is lifted in Rust. 54 classes, counted
from `store/rules/registry.tsv`:

- **The locked forms** (11): ADD_LOCK, AND_LOCK, OR_LOCK, XOR_LOCK, SUB_LOCK, INC_LOCK, DEC_LOCK, NEG_LOCK, XADD_LOCK,
  CMPXCHG_LOCK (`_int.locked`, 590-596: the base rule under the base iclass) and CMPXCHG16B_LOCK; with CMPXCHG16B
  (`_int.cmpxchg16b`, 578-587: one `cx16` op).
- **The system forms** (21): CLD, STD (`df`), CPUID (`cpuid`), SFENCE, LFENCE, MFENCE (`fence`), PAUSE (`pause`),
  STMXCSR, LDMXCSR, VSTMXCSR, VLDMXCSR (`mxcsr`), FNSTCW (`fcw`), XGETBV (`xgetbv`), and the eight REP STOS and REP MOVS
  classes (`rep`; a 0x67 prefix refused).
- **The model's steps** (21): SHLD, SHRD (`double_shift`), RCR (`rcr`), STC, CLC, CMC, SAHF, PUSHFQ, POPFQ and the
  single string forms CMPSB, CMPSD, LODSB, MOVSB, MOVSD, SCASB, SCASD, STOSB (`implicit`), JRCXZ, LOOP, LOOPE, LOOPNE
  (`loop`): one `x86` op each, lowered to `hvc #11`. The model that runs them is step 4's; their cases wait for it.

The lowering (`src/w2m/vm_backend.py`): in `compile_insn`, the locked load's alignment check and load-exclusive
(1531-1545) and the store-exclusive with its retry (1550-1552), with `locked` turning off folded addresses, fused moves
and operand forms (1466-1470); the branches for `mxcsr` (1655, `_lower_mxcsr` 1150-1195), `fcw` (1660-1662),
`xgetbv` and `ud` (1665), `fence` (1667), `rep` (1669-1686), `cx16` (1732-1760), `x86` (1785), `cpuid` (1840-1848),
`df` (1849), `pause` (1851-1859); and the ops' places in `FLAGS_NONE`, `FLAGS_ALL_READ` and `_op_flags` (857-889),
`_temps_read` (1013-1060), `OPERAND_SAFE` (1198) and `commit_order` (`lift.py` 246-290).

## prerequisites

- 158 (closed)

## context_files

- `src/w2m/rules/_int.py` 16-31, 262-361, 575-655; the 23 rule files named above; `src/w2m/lift.py` 10-66, 246-290
- `src/w2m/vm_backend.py` as above
- `crates/translator` (Sprints 151, 153); `crates/w2m_vm/tests/case_runner.rs`, `crates/translator/tests/c_words.rs`

## signal contract

### Emits

- The `case-runner-rust` sessions, four reference profiles (`harness/case-runs-rust.toml`): SESSION_INIT,
  CANDIDATE_GENERATED, COMPARISON_COMPLETED and CANDIDATE_ACCEPTED per class, with group 3's classes present in the
  cases now the Rust translator's candidates; SESSION_COMPLETED completed. The judges' ledger stays empty.

### Consumes

The stencil record, the case files and Windows results, the Python words fixture.

### Invariants

- No Python file added or changed.
- Sprints 151-158's judges pass unchanged, `sleeper`'s skeleton record among them.

## artifact contract

### Files modified

- `crates/translator/` (`lift.rs`, `rules.rs`, `lower.rs`), `WORKING_AGREEMENT.md`; four records under `store/runs/`

### Content assertions

- Each function cites the Python lines it carries.

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values, from the C emitter:** every record call of groups 1 to 3 the C compiles has Rust words equal to
  the C's; the count of group 3's calls is stated.
- **Expected values, from Windows:** the case runner with the Rust translator judges each run's committed counts with
  zero difference classes; the locked and system classes present in the cases join its compiled classes.
- **Expected values, from the Python words:** every case code the Rust rules lift has the fixture's words.
- **Planted faults:** (1) the locked form's store-exclusive stores the loaded value, not the result: the case runner
  names a `_LOCK` class's memory, the C judge its words; (2) the fence kinds of LFENCE and SFENCE swapped: the C judge
  names the fence class.

## done criteria

Every class `_int.py` lifts is lifted and lowered in Rust, equal to the C and the Python, and agrees with Windows where
the cases reach it.

## self-review checklist

- [x] Scope names one verifiable property: group 3 in Rust, equal to the C, the Python and Windows.
- [x] context_files cite the Python carried, by line.
- [x] Emits: the sessions' tags, each named.
- [x] Artifact contract gradable by command.
- [x] Observation contract: three judges and two planted faults, each reaching the judge it tests.
- [x] One concept: the third group, the last of `_int.py`.
