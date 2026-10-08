# Sprint 022 — explicit operands compared with XED

```yaml
---
id: 022
status: closed
phase: 3
pass_kind: functional
---
```

## scope

Third layer of the decoder plan (Decision 2026-10-08, item 4): after length and iclass, explicit operands. XED side: `tools/xedshim/xedshim.c` prints each displayed operand (registers, memory base/index/scale/displacement/width, LEA address, immediate width and raw value, branch displacement); `XedResult.operands`. w2m side: `x86dec.decode` returns `Fields` (prefixes, REX, ModRM, SIB, displacement, immediate bytes, effective operand and address size, operand template); spec rows carry `ops` in SDM Vol 2D Appendix A notation (`Ev,Gv`, `Gv,M`, `Zv,Iv`, `Jz`, ...) and `opsize64` (`d64`/`f64`, Table A-1 superscripts); `src/w2m/x86ops.py` resolves the template against the fields. Integer general-purpose rows only (303 rows).

Compared form: segment kept only for FS/GS (other overrides have no effect in 64-bit mode, SDM Vol 1 3.4.2.1); scale dropped when there is no index register.

## repair-path labels (v0.2 §11.3)

- Spec 0.6.0-0.6.1 (`ops`, `opsize64`; FF /2 and FF /4 are f64; MOVSXD source `Ez`): contract revision.
- `Fields`, `x86ops.operands`, operand-size rule in the decoder: implementation repair.

## command exit codes

- `uv run pytest` → 0 (125 passed); `ruff check` → 0; `uv run mypy` (strict, src/w2m) → 0; `gen_decoder --check` → 0

## observation

| surface | positions | compared | differ |
|---|---|---|---|
| Python 3.14.8 corpus | 2,391,671 | 2,262,815 (94.61%) | 0 |
| runner-built probes (store/binaries) | 52,785 | 49,982 (94.69%) | 0 |
| generated encodings (agree cases with a template) | 28,302 | 11,924 | 0 after repair; 20 before |

The 2,444,456 positions of Sprints 017-021 are these two corpus rows together.

Power check: replacing the no-REX byte-register table (AH-BH) with the REX one (SPL-DIL) produced 224 corpus disagreements (MOVZX_GPRv_GPR8 190, AND_MEMb_GPR8 13, ADD_MEMb_GPR8 8, ...). The comparison detects a one-table error.

Generated encodings found two template errors the corpus never exercised: 66-prefixed near indirect CALL/JMP (XED: 64-bit target, Intel f64; ours read 16 bits) and 66-prefixed MOVSXD (source `Ez`, 16 bits; ours read 32).

## residue

131,659 corpus positions (5.4%) have no template: rows with no explicit operand (RET_NEAR 41,083, NOP 14,147, INT3, CDQE) and SSE/AVX rows (MOVUPS 17,802, MOVAPS, MOVSD_XMM, MOVDQA/U, PXOR, VPXOR, ...), which need the V/W/H/U register notation. AMD honors 66 on FF /2 and FF /4 (16-bit target); XED and w2m follow Intel — recorded as a vendor split, not a disagreement.
