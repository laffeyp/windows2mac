# Sprint 017 — Intel XED as the primary reference decoder

```yaml
---
id: 017
status: closed
phase: 3
pass_kind: bridge
---
```

## scope

Build Intel XED v2026.08.23 (commit 0bcb6237, Apache-2.0) from source with mbuild 1b437e40 under `third_party/` (not committed); author `tools/xedshim/xedshim.c` (batch hex → length, iclass, iform, Intel text) and `src/w2m/xedref.py`; make XED judge length at every position in `scripts/decoder_differential.py`; settle the open disputes by XED.

## bridge mapping

- Build: `cd third_party/xed && python3 mfile.py --ar=$(xcrun --find ar) install` (mbuild defaults to `llvm-ar`, which Xcode does not provide).
- API used: `xed_tables_init`, `xed_decoded_inst_zero`, `xed_decoded_inst_set_mode(LONG_64, ADDRESS_WIDTH_64b)`, `xed_decode`, `xed_decoded_inst_get_length`, `xed_decoded_inst_get_iclass` / `xed_iclass_enum_t2str`, `xed_decoded_inst_get_iform_enum` / `xed_iform_enum_t2str`, `xed_format_context(XED_SYNTAX_INTEL)`.

## command exit codes

- `uv run pytest` → 0 (75 passed); `ruff`, `mypy` → 0; differential → 0

## observation

- XED decodes all 2,444,456 positions (probes + Python corpus). Capstone's length equals XED's at all of them; w2m's equals XED's at all 2,426,652 it decodes; 17,804 remain spec gaps.
- Disputes settled: F3 C3 is RET_NEAR (ours right, Capstone's "repz ret" is spelling); F2 0F 10 is MOVSD_XMM and A5 is MOVSD (two iclasses for one mnemonic); C5 FC 77 is VZEROALL (our spec bug confirmed).
- Capstone's 15 "undecodable" corpus locations are embedded ASCII in OpenSSL's hand-written assembly ("Camellia", "by <appr"...); XED also refuses all 15. The traversal walked into data; the decoders were right.
