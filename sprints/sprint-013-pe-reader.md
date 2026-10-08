# Sprint 013 — PE32+ reader

```yaml
---
id: 013
status: closed
phase: 3
pass_kind: functional
---
```

## scope

Author `src/w2m/pe.py`: parse a PE32+ x86-64 image (headers, sections, imports, base relocations, exports, TLS callbacks) per Microsoft's "PE Format", raising PeError with the offset on malformed input, and list statically known code targets (entry, exports, TLS callbacks; v0.2 §5.1). Fixtures: the two probe executables built by clang 20.1.8 on the `windows-2025` runner (workflow now uploads `probe.exe`), committed under `store/binaries/<sha256>.exe`. Grade against `llvm-objdump` (Apple LLVM 21.0.0), an independent reader.

## files

- `src/w2m/pe.py`, `tests/test_pe.py` (created); `store/binaries/*.exe` (fixtures); `.github/workflows/windows-probe.yml` (uploads probe.exe)

## command exit codes

- `uv run pytest` → 0 (50 passed); `ruff check` → 0; `mypy` → 0

## observation contract

- Both fixtures: entry RVA, image base, and every import name equal llvm-objdump's; `.text` is r-x and holds the entry; DIR64 relocations present.
- Malformed: no MZ, bad PE signature offset, truncation, i386 machine → PeError with the named reason.

## self-review checklist

- [x] Independent check surface (llvm-objdump), not self-consistency.
- [x] Fixtures are real runner-built binaries, hashes equal to the runner's recorded sha256.
- [x] Two source files, one concept.
