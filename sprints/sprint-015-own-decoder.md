# Sprint 015 — w2m's own x86-64 decoder, differential against Capstone

```yaml
---
id: 015
status: closed
phase: 3
pass_kind: functional
---
```

## scope

Author `specs/x86_decode.json` (430 entries, each citing an Intel SDM opcode-map table: Vol 2D Appendix A Tables A-2..A-6, Vol 2A §2.1-2.3 for prefixes/REX/VEX/ModRM), `scripts/gen_decoder.py` (spec → `src/w2m/x86dec_tables.py`, with `--check`), and `src/w2m/x86dec.py` (prefixes, REX, VEX C4/C5, maps 1/0F/0F38/0F3A, ModRM/SIB/displacement, immediates). Decode every reachable instruction of the two probes and compare length and mnemonic with Capstone (two-decoder plan, user decision 2026-10-08).

## command exit codes

- `python3 scripts/gen_decoder.py --check` → 0; `uv run pytest` → 0 (71 passed); `ruff`, `mypy` → 0

## observation contract

- First differential run: 52,691 / 52,785 agree (99.82%); all 94 disagreements one class (SSE `movsd` F2 0F 10/11 decoded as `repne movsd`).
- After the fix: 52,785 / 52,785 agree on both binaries.

## limits stated before closing

- The spec was written from the mnemonic list of these two binaries, so agreement here is partly by construction; generalization is tested on unseen binaries next.
- Agreement covers length and mnemonic, not operands.
