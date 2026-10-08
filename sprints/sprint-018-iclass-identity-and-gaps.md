# Sprint 018 — XED iclass identity, spec gaps closed, CPU-feature dependence

```yaml
---
id: 018
status: closed
phase: 3
pass_kind: functional
---
```

## scope

Close the Sprint 016 worklist except EVEX/XOP; give every spec row an XED-style iclass assigned by rule (independent of XED) and compare it with XED at every position; model VEX.L, memory/register form, moffs operands, repeated REX bytes, REPE spelling; make the XED shim decode for a named chip and make the spec's CPU-feature-dependent rows explicit.

## repair-path labels (v0.2 §11.3)

| Change | Kind |
|---|---|
| 299 new rows (SSE2/AVX2 integer, AES-NI, SHA, ADX, BMI1/2, CRC32, MOVBE, kmov, string ops, x87 control, moffs) | contract revision (spec 0.2.0) |
| `l`, `form`, `moffs`, `repe_string` fields | contract revision (spec 0.2.0) |
| iclass per row; `requires` / `fallback` on 5 rows | contract revision (spec 0.3.0) |
| decoder reads VEX.L, form, moffs; last REX wins, legacy prefix after REX cancels it; REPE/REPNE fusion; iclass output; `features` argument | implementation repair (`src/w2m/x86dec.py`) |
| XED shim takes a chip name | implementation repair (`tools/xedshim/xedshim.c`) |

## command exit codes

- `python3 scripts/gen_decoder.py --check` → 0; `uv run pytest` → 0 (87 passed); `ruff`, `mypy` → 0

## observation

- First iclass comparison: 99.997% equal; the 4 differing kinds (TZCNT/BSF, ENDBR64/NOP, PAUSE/NOP, RDSSPQ/NOP) were XED decoding for an unspecified chip. Same bytes, different instruction by CPU feature; with chip ALL, 0 differences.
- Final, probes + Python corpus, 2,444,456 positions: ours matches XED on length and iclass at all 2,444,113 it decodes; 343 gaps (0.014%), all EVEX (62) or XOP (8F).
- Under chips PENTIUM4, SKYLAKE, ALL, ours with the matching feature set equals XED on all five feature-dependent encodings.
