#!/bin/sh
# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
# Every gate in WORKING_AGREEMENT.md, in order; exits non-zero at the first failure (set -e).
# Added Sprint 028 after two closes recorded passing gates that had failed: a gate piped
# through tail (Sprint 026) and gates chained with ';' before a commit (Sprint 028).
#
# Usage: scripts/gates.sh [--all]. By default (Sprint 168; user 2026-10-28: "must use established methods to run tests
# closer to code, not all tests all the time") the gates the change since origin/sprint-066 reaches, committed or not,
# new files included: the Rust tests by the crate graph (cargo-nextest's rdeps(), nexte.st filterset reference), the
# substrate samples and the Python gates by the paths they read, and the records checks over the new records. A change
# the selection cannot map runs those gates whole. --all runs every gate over everything.
set -eu
cd "$(dirname "$0")/.."
gates_start=$(date +%s)
# rustup installs cargo in ~/.cargo/bin, which a non-login shell may not have on its PATH (Sprint 126: the gates failed
# with "cargo: command not found")
[ -d "$HOME/.cargo/bin" ] && PATH="$HOME/.cargo/bin:$PATH"
all=0
plan=0
[ "${1:-}" = "--all" ] && all=1
[ "${1:-}" = "--plan" ] && plan=1

# the change: every path that differs from origin/sprint-066's merge base, and every new file (.claude/ is the
# session's, never committed)
base=$(git merge-base HEAD origin/sprint-066)
changed=$({ git diff --name-only "$base"; git ls-files --others --exclude-standard; } | grep -v '^\.claude/' | sort -u)
# GATES_CHANGED, newline-separated paths, stands in for the change: the selection's plants set it (Sprint 171)
[ -n "${GATES_CHANGED:-}" ] && changed=$GATES_CHANGED
# touched PATTERN [EXCEPT]: some changed path matches PATTERN and not EXCEPT
touched() { printf '%s\n' "$changed" | grep -E "$1" | grep -Ev "${2:-^$}" | grep -q .; }
said() { echo "gates: $*"; }
[ $all = 1 ] && said "--all: every gate" || said "the change since $(git rev-parse --short "$base"): $(printf '%s\n' "$changed" | grep -c .) paths"

# the selection, decided before any gate runs, so `--plan` prints it and stops (Sprint 171)
crates=$(printf '%s\n' "$changed" | sed -n 's|^crates/\([^/]*\)/.*|\1|p' | sort -u)
rust=none
if [ $all = 1 ]; then
  rust=all; why="--all"
elif touched '^(Cargo\.(toml|lock)|\.cargo/|rust-toolchain\.toml|scripts/sign_and_run\.sh|corpus/|signals/|harness/|probes/|specs/|tools/|tests/data/|store/|build\.rs)' '^store/runs/'; then
  # the cargo runner, the toolchain and corpus/ (read by crates/pe_image/tests/llvm_objdump.rs 266) are inputs of every
  # test binary (Sprint 171; reviewer-2's Medium 1: Sprint 168 changed the runner and ran no Rust test)
  rust=all; why="an input outside one crate changed"
elif [ -n "$crates" ]; then
  rust=crates
fi
# the untracked inputs under build/ (the Windows results the case runner judges against, the Python words fixture)
# against their committed manifest: a new download is a change no git path shows (Sprint 171; reviewer-2's Medium 2)
if [ $rust != all ] && [ -d build/probe-runs ]; then
  stale=$(shasum -a 256 -c store/case_runner/build-inputs.sha256 2>/dev/null | grep -v ': OK$' | head -1 || true)
  extra=$(ls build/probe-runs/*/result-1.bin build/case_runner/* 2>/dev/null | while read -r f; do grep -q "  $f\$" store/case_runner/build-inputs.sha256 || echo "$f"; done | head -1)
  if [ -n "$stale$extra" ]; then
    rust=all; why="a build/ input differs from store/case_runner/build-inputs.sha256: ${stale:-$extra new}"
  fi
fi
samples=0
if [ $all = 1 ] || touched '^(crates/(events|vocabulary_generator|vocabulary_samples|session_events)/|harness/|signals/|substrate\.commit)'; then
  samples=1
fi
# the new records; the frozen Python's tests read only records whose baseline names `differential`
# (tests/test_differential.py 99-111), which no Rust Producer writes
records=$(printf '%s\n' "$changed" | sed -n 's|^\(store/runs/[^/]*\)/.*|\1|p' | sort -u)
differential=0
for r in $records; do
  [ -d "$r" ] && head -c 4096 "$(ls "$r"/events-*.jsonl | head -1)" | grep -q '"differential"' && differential=1
done
python=0
if [ $all = 1 ] || [ $differential = 1 ] || touched '^(src/|scripts/.*\.py$|tests/|tools/|signals/|specs/|pyproject\.toml$|uv\.lock$|substrate\.commit$|store/)' '^store/runs/'; then
  python=1
fi
case $rust in
  all) said "Rust tests: all ($why)" ;;
  crates) said "Rust tests: $(for c in $crates; do printf 'rdeps(%s) | ' "$c"; done | sed 's/ | $//')" ;;
  none) said "Rust tests: none (no crate changed)" ;;
esac
[ $samples = 1 ] && said "substrate samples and their plants" || said "substrate samples: not run (no vocabulary, harness or event crate changed)"
[ $python = 1 ] && said "Python gates" || said "Python gates: not run (nothing the frozen Python reads changed)"
said "records: $(printf '%s\n' "$records" | grep -c .) new"
[ $plan = 1 ] && exit 0

# substrate is the clone ./substrate at substrate.commit (user 2026-10-10; review 2026-10-10 D1):
# CI clones that commit, so a local run judged on another kernel would not be the run CI repeats.
pin=$(cat substrate.commit)
head=$(git -C substrate rev-parse HEAD)
[ "$head" = "$pin" ] || { echo "gates: ./substrate is at $head, substrate.commit pins $pin"; exit 1; }
[ -z "$(git -C substrate status --porcelain -- src pyproject.toml)" ] || { echo "gates: ./substrate has uncommitted kernel changes"; exit 1; }
# every probe compiles against Windows headers (mingw-w64) before a runner sees it (Sprint 044: two
# runner rounds lost to compile errors); skipped where the headers are absent (CI's Ubuntu). The Windows
# SDK's rpcndr.h defines small and hyper, mingw's does not: defined here too (Sprint 046, two rounds lost). The
# runner's clang targets MSVC, where MS extensions (__try) are on; mingw's excpt.h declares __C_specific_handler
# dllimport, the SDK's does not (Sprint 053: a probe defines its own)
win=/opt/homebrew/opt/mingw-w64/toolchain-x86_64/x86_64-w64-mingw32/include
if [ -d "$win" ] && { [ $all = 1 ] || touched '^probes/.*\.(c|h)$'; }; then
  for c in $(grep -l "include <windows.h>" probes/*/*.c); do
    clang --target=x86_64-w64-mingw32 -fms-extensions -fsyntax-only -Wall -Werror -Wno-unknown-attributes -Wno-inconsistent-dllimport -Dsmall=char -Dhyper=__int64 -isystem "$win" "$c" \
      || { echo "gates: $c does not compile for Windows"; exit 1; }
  done
fi
# the Rust workspace (Sprint 125; docs/rust-port-plan-2026-10-21.md set-up): format, lints as errors, tests, and a
# release build, then the session check under substrate: a Rust binary's events on a record, replayed
cargo fmt --check
cargo clippy --all-targets -- -D warnings
# the case runner (crates/w2m_vm/tests/case_runner.rs) fails rather than skips when its inputs are missing: the
# downloaded Windows results (build/probe-runs) and the Python words fixture (build/case_runner) (Sprint 149; the
# review of link 3 found it could pass having judged nothing). CI, on Linux, does not compile it.
if [ $rust = all ]; then
  W2M_REQUIRE_CASE_RUNS=1 cargo test --quiet
elif [ $rust = crates ]; then
  filter=$(for c in $crates; do printf 'rdeps(%s) | ' "$c"; done | sed 's/ | $//')
  W2M_REQUIRE_CASE_RUNS=1 cargo nextest run --cargo-quiet --status-level fail -E "$filter"
  docs=$(for c in $crates; do [ -f "crates/$c/src/lib.rs" ] && printf -- '-p %s ' "$c"; done; true)
  # nextest does not run doc tests (a compile_fail doc test holds Vcpu to !Send, Sprint 157)
  [ -n "$docs" ] && cargo test --quiet --doc $docs
fi
cargo build --release --quiet
# the Rust judges' discrepancy ledger is what their records say (Sprint 154; review of Sprints 151-152)
target/release/case_runs --check-ledger
# the planted discrepancy chain (Sprint 171; reviewer-2's Low 1): the Rust case runner with the first run's CF flipped
# writes 32 discrepancies, each after its REDUCTION_COMPLETED and naming its case's hashes, which `check_records` holds
# to rules S5 and E1 (Sprint 167); from Sprint 175 also FCOS's and FSIN's, the open pair this profile reports unplanted
# (tests/test_class_runs.py 33), so 34
if [ $rust != none ] && [ -d build/probe-runs ]; then
  chain=$(mktemp -d)/case-runs-planted-$(date +%s)000000000
  W2M_CASE_PROFILE="INTEL(R) XEON(R) PLATINUM 8573C; gha-windows-2022-x64-10.0.20348.5622-Server" \
    uv run substrate run --file harness/case-runs-planted.toml --root "$chain" --strict > /dev/null
  reports=$(cat "$chain"/events-*.jsonl | grep -c '"kind":"DISCREPANCY_REPORTED"' || true)
  [ "$reports" = 34 ] || { echo "gates: the planted chain holds $reports DISCREPANCY_REPORTED, not 34"; exit 1; }
  target/release/check_records --since 2026-10-17 --show 3 "$chain" > /dev/null || { echo "gates: the planted chain breaks S5 or E1"; target/release/check_records --since 2026-10-17 --show 3 "$chain"; exit 1; }
fi
# the vocabulary generated into Rust and substrate's schema file, byte for byte (Sprint 126)
target/release/vocabulary_generator --check > /dev/null
# every sprint from 108 marked done has its Signal Report (Sprint 127: the reports stopped unnoticed three times)
target/release/signal_report --check > /dev/null
# every plan and design from 2026-10-24 on states the goal's figures and names the systems that solved the same
# problem, with sources (Sprint 128; Decision 2026-10-24 user, fix (a))
target/release/design_check > /dev/null
if [ $samples = 1 ]; then
  session=$(mktemp -d)/session-check
  uv run substrate run --file harness/session-check.toml --root "$session" --strict > /dev/null
  uv run substrate replay "$session" --level 2 > /dev/null
  samples=$(mktemp -d)/vocabulary-samples
  uv run substrate run --file harness/vocabulary-samples.toml --root "$samples" --strict > /dev/null
  uv run substrate replay "$samples" --level 2 > /dev/null
  # the planted sample: a version ending in a newline, refused by substrate as by the Rust type (Sprint 126)
  planted=$(mktemp -d)/vocabulary-samples-planted
  if uv run substrate run --file harness/vocabulary-samples-planted.toml --root "$planted" --strict > /dev/null 2>&1; then
    echo "gates: the planted sample was accepted"; exit 1
  fi
  # the failure itself, not the record's start, which names vocab_version in its config (Sprint 162 found the check read it)
  uv run substrate tail "$planted" --format jsonl --no-follow | grep '"kind":"substrate.ProducerFailed"' | grep -q "vocab_version" || { echo "gates: the planted failure does not name vocab_version"; exit 1; }
  # the planted metric: a value vocabulary 0.11 does not name, refused by substrate as by the Rust type (Sprint 162)
  planted=$(mktemp -d)/vocabulary-samples-planted-metric
  if uv run substrate run --file harness/vocabulary-samples-planted-metric.toml --root "$planted" --strict > /dev/null 2>&1; then
    echo "gates: the planted metric was accepted"; exit 1
  fi
  uv run substrate tail "$planted" --format jsonl --no-follow | grep '"kind":"substrate.ProducerFailed"' | grep -q "Invalid enum value 'exit_ns_p95'" || { echo "gates: the planted failure does not name the metric value"; exit 1; }
  # the planted guest_trace: a string where 0.12 types a bool, refused by substrate's schema past the Rust type (Sprint
  # 171; reviewer-2's Medium 4: Sprint 170's plant stopped at the Rust type)
  planted=$(mktemp -d)/vocabulary-samples-planted-guest-trace
  if uv run substrate run --file harness/vocabulary-samples-planted-guest-trace.toml --root "$planted" --strict > /dev/null 2>&1; then
    echo "gates: the planted guest_trace was accepted"; exit 1
  fi
  uv run substrate tail "$planted" --format jsonl --no-follow | grep '"kind":"substrate.ProducerFailed"' | grep -q 'at `$.guest_trace`' || { echo "gates: the planted failure does not name guest_trace"; exit 1; }
fi
if [ $python = 1 ]; then
  python3 scripts/check_vocab.py
  python3 scripts/gen_events.py --check
  python3 scripts/gen_decoder.py --check
  uv run python scripts/gen_stencil_table.py --check     # Sprint 080: the C table from store/stencil
  uv run ruff check src scripts tests
  uv run mypy
  # a failing run's output is kept under build/gates-failures/ before the gates exit, so a failure that does not repeat
  # leaves its evidence (Sprint 176; reviewer-2's Medium 5 of 2026-10-28: Sprint 173's frozen-host failure was re-run
  # away)
  mkdir -p build/gates-failures
  kept=build/gates-failures/pytest-$(date +%Y%m%dT%H%M%S).txt
  uv run pytest -q > "$kept" 2>&1 || { tail -40 "$kept"; echo "gates: pytest failed; its output is $kept"; exit 1; }
  tail -3 "$kept"
  rm "$kept"
fi
if [ $all = 1 ]; then
  uv run python scripts/check_replay.py
  # the vocabulary's rules over every record; records made from 2026-10-17 on are held to them (Sprint 109), by the
  # Python and by its Rust port (Sprint 169), whose printed lines must be the Python's
  py=$(uv run python scripts/check_records.py --since 2026-10-17 --show 0)
  echo "$py"
  [ "$(target/release/check_records --since 2026-10-17 --show 0)" = "$py" ] || { echo "gates: the Rust records check differs from the Python's"; exit 1; }
elif [ -n "$records" ]; then
  for r in $records; do uv run substrate replay "$r" --level 2 > /dev/null; done
  # shellcheck disable=SC2086
  target/release/check_records --since 2026-10-17 --show 0 $records
fi
echo "gates: all passed"
# each passing run is kept, so `signal_report --check` can find the run a closed tail entry's gates line names
# (Sprint 179; reviewer-2's Medium 3 of its review of 175-177: the close check read tokens, not the gates' state)
mkdir -p build
printf 'exit 0 in %s s\t%s\t%s\n' "$(( $(date +%s) - gates_start ))" "$([ $all = 1 ] && echo all || echo default)" \
  "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >> build/gates-passes.txt
