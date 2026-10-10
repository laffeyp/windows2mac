#!/bin/sh
# Every gate in WORKING_AGREEMENT.md, in order; exits non-zero at the first failure (set -e).
# Added Sprint 028 after two closes recorded passing gates that had failed: a gate piped
# through tail (Sprint 026) and gates chained with ';' before a commit (Sprint 028).
set -eu
cd "$(dirname "$0")/.."
# substrate is the clone ./substrate at substrate.commit (user 2026-10-10; review 2026-10-10 D1):
# CI clones that commit, so a local run judged on another kernel would not be the run CI repeats.
pin=$(cat substrate.commit)
head=$(git -C substrate rev-parse HEAD)
[ "$head" = "$pin" ] || { echo "gates: ./substrate is at $head, substrate.commit pins $pin"; exit 1; }
[ -z "$(git -C substrate status --porcelain -- src pyproject.toml)" ] || { echo "gates: ./substrate has uncommitted kernel changes"; exit 1; }
# every probe compiles against Windows headers (mingw-w64) before a runner sees it (Sprint 044: two
# runner rounds lost to compile errors); skipped where the headers are absent (CI's Ubuntu). The Windows
# SDK's rpcndr.h defines small and hyper, mingw's does not: defined here too (Sprint 046, two rounds lost)
win=/opt/homebrew/opt/mingw-w64/toolchain-x86_64/x86_64-w64-mingw32/include
if [ -d "$win" ]; then
  for c in $(grep -l "include <windows.h>" probes/*/*.c); do
    clang --target=x86_64-w64-mingw32 -fsyntax-only -Wall -Werror -Wno-unknown-attributes -Dsmall=char -Dhyper=__int64 -isystem "$win" "$c" \
      || { echo "gates: $c does not compile for Windows"; exit 1; }
  done
fi
python3 scripts/check_vocab.py
python3 scripts/gen_events.py --check
python3 scripts/gen_decoder.py --check
uv run ruff check src scripts tests
uv run mypy
uv run pytest -q
uv run python scripts/check_replay.py
echo "gates: all passed"
