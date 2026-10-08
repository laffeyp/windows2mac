#!/bin/sh
# Every gate in WORKING_AGREEMENT.md, in order; exits non-zero at the first failure (set -e).
# Added Sprint 028 after two closes recorded passing gates that had failed: a gate piped
# through tail (Sprint 026) and gates chained with ';' before a commit (Sprint 028).
set -eu
cd "$(dirname "$0")/.."
python3 scripts/check_vocab.py
python3 scripts/gen_events.py --check
python3 scripts/gen_decoder.py --check
uv run ruff check src scripts tests
uv run mypy
uv run pytest -q
echo "gates: all passed"
