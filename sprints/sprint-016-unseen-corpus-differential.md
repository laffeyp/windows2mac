# Sprint 016 — two-decoder differential on an unseen corpus

```yaml
---
id: 016
status: closed
phase: 3
pass_kind: observation
---
```

## scope

Run Capstone and w2m's spec-generated decoder over every reachable instruction of binaries neither the spec nor its author had seen: the official Python 3.14.8 Windows embeddable package (33 PE files, MSVC-built; manifest with source URL and SHA-256 in `corpus/python-3.14.8.manifest.json`; binaries not committed). Author `scripts/decoder_differential.py`; write the worklist to `specs/x86_decode_worklist.json`.

## command exit codes

- `uv run python scripts/decoder_differential.py corpus/python-3.14.8 --json specs/x86_decode_worklist.json` → 0
- `uv run pytest` → 0 (71 passed); `ruff`, `mypy` → 0

## observation

- 2,391,671 reachable instructions; 99.24% agree on length and mnemonic.
- 17,804 instructions (0.74%) in 203 spec-gap kinds — missing table rows, dominated by AES-NI, ADX, BMI2, SSE2/AVX2 integer ops, bswap, shld/shrd (libcrypto 97.8% agree).
- 273 disagreements in 2 kinds, no length disagreements: F3 C3 (`repz ret` vs `ret`, spelling of one instruction) and C5 FC 77 (`vzeroall` vs `vzeroupper`: the spec ignores VEX.L — a real spec bug).
- Capstone itself failed to decode 15 reachable locations (libcrypto 13, sqlite3 1, libssl 1): reference failures, to be checked against LLVM.
