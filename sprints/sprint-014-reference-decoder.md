# Sprint 014 — reference decoder (Capstone), cross-checked against LLVM

```yaml
---
id: 014
status: closed
phase: 3
pass_kind: bridge
---
```

## scope

Adopt Capstone 5.0.9 (optional extra `reference`) as the existing x86-64 decoder in the two-decoder plan (user decision 2026-10-08). Author `src/w2m/x86ref.py`: recursive traversal from every static code target (entry, exports, TLS callbacks, `.pdata` function starts), following direct branches and calls, resolving IAT calls and references to "dll!name". Extend `src/w2m/pe.py` with IAT slot names and `.pdata` function starts. Grade Capstone itself against LLVM's disassembler at every reachable instruction.

## files

- `src/w2m/x86ref.py`, `tests/test_x86ref.py` (created); `src/w2m/pe.py`, `pyproject.toml` (modified)

## command exit codes

- `uv run pytest` → 0 (54 passed); `ruff check` → 0; `mypy` → 0

## observation contract

- Both probes: no undecodable bytes; > 20,000 reachable instructions; all `.pdata` starts used as roots; every import referenced; > 90% of executable bytes covered.
- Capstone and LLVM agree on boundary, length, and mnemonic at every reachable instruction, after normalizing LLVM's printing of `lock` (own line) and `rep` (two tokens).

## self-review checklist

- [x] Bridge mapping from installed package (capstone 5.0.9; module reports 5.0.7) and its operand API.
- [x] The reference is itself checked against a third decoder before it judges ours.
