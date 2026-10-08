# Sprint 019 — EVEX and XOP decoding

```yaml
---
id: 019
status: closed
phase: 3
pass_kind: functional
---
```

## scope

Decode EVEX (`62`, AVX-512; SDM Vol 2A §2.7) and AMD XOP (`8F` with map field ≥ 8; AMD APM Vol 6) prefixes; key the tables by encoding class (legacy / vex / evex / xop); add the 75 rows the corpus needs (W-dependent AVX-512 names such as vmovdqu32/64, vpternlogd/q, vinserti32x4/64x2; XOP vprot*).

## repair-path labels (v0.2 §11.3)

- Spec 0.4.0 rows and `enc` field: contract revision.
- EVEX/XOP prefix parsing; table key by encoding class; VEX.L applied only to VEX: implementation repair.

## command exit codes

- `python3 scripts/gen_decoder.py --check` → 0; `uv run pytest` → 0 (93 passed); `ruff`, `mypy` → 0

## observation

- Probes + Python corpus, 2,444,456 positions: spec gaps 0; ours equals XED on length and iclass at every position.
- Capstone differs from ours only in spelling: F3 C3 `repz ret` (XED RET_NEAR); 62..3F..02 `vpcmpleb` (XED VPCMPB, predicate in the immediate).
