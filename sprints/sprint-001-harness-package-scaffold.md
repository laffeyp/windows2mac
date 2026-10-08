# Sprint 001 — harness package scaffold

```yaml
---
id: 001
status: closed
phase: 1
pass_kind: architecture
---
```

## scope

Create the w2m harness Python package: `pyproject.toml` (name `w2m`, Python >=3.12, runtime dependency `substrate-kernel==1.1.1`, dev tools ruff/mypy/pytest) and `src/w2m/__init__.py` exporting `VOCAB_VERSION = "0.1"`. `uv sync --extra dev` creates the environment; `uv run python -c "import w2m, substrate.api"` succeeds.

## prerequisites

- 000

## context_files

- `sdd-kit-3/AGENTS.md`
- `WORKING_AGREEMENT.md` (build commands, permitted dependencies)
- `substrate/pyproject.toml` (Python floor, msgspec pin, ruff/mypy settings to mirror)

## signal contract

### Emits

None (scaffold).

### Invariants

- No dependency beyond `substrate-kernel` at runtime; dev extras only ruff, mypy, pytest, pytest-asyncio.
- `signals/0.1.json` unchanged.

## artifact contract

### Files created

- `pyproject.toml`
- `src/w2m/__init__.py`

### Content assertions

- `pyproject.toml` contains `substrate-kernel==1.1.1` and `requires-python = ">=3.12"`.
- `src/w2m/__init__.py` contains `VOCAB_VERSION = "0.1"`.

### Command exit codes

- `uv sync --extra dev` returns 0
- `uv run python -c "import w2m, substrate.api; print(w2m.VOCAB_VERSION)"` returns 0 and prints `0.1`
- `uv run ruff check` returns 0
- `uv run mypy` returns 0

## observation contract

Not applicable: no runtime behavior.

## done criteria

The package imports with substrate available, and lint and types are clean.

## self-review checklist

- [x] Scope concrete: two files, one gate set.
- [x] context_files listed.
- [x] Emits empty; consistent.
- [x] Assertions gradable by grep and exit codes.
- [x] Observation contract not required (architecture).
- [x] Two files, one concept.
