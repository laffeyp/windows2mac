#!/bin/sh
# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
# The checks before a work commit (WORKING_AGREEMENT "Before a work commit"), in order; exits non-zero at the first
# failure (set -e), its status the failing check's, and after the last check nothing runs, so its status is the last
# check's (Sprint 301; reviewer-3's Low 1 at 300). The list was typed by hand at each commit and broke three ways: joined
# by ';', a lint failure committed (Sprint 281, 45382e739); `set -e` inside a zsh subshell, which does not stop (285);
# piped into `tail`, which printed tail's "exit 0" over a failed lint (299). Read the whole output.
#
# Usage: scripts/pre-commit.sh. Run it before a work commit, and again after any edit made after it (Sprint 288).
# The tests and the batch run are not here: they run once, in scripts/gates.sh, on the work commit (Sprint 298).
set -eu
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"

# the sprint is the one active card's; the record check is the last closed sprint's, one before it. Zero or two
# active cards stop here, by name, rather than check against a guessed sprint (the gates refuse two the same way).
active=$(grep -l '^status: active$' sprints/sprint-*.md || true)
if [ "$(printf '%s' "$active" | grep -c .)" -ne 1 ]; then
  echo "pre-commit: not one card with status: active: ${active:-none}"
  exit 1
fi
sprint=$(printf '%s\n' "$active" | sed -E 's|.*/sprint-([0-9]{3})-.*|\1|')
closed=$(($(printf '%s' "$sprint" | sed 's/^0*//') - 1))
echo "pre-commit: sprint $sprint, the record checked against sprint $closed"

echo "pre-commit: format"
cargo fmt --all --check
echo "pre-commit: lint, the gates' flags"
cargo clippy -q --release --workspace --all-targets -- -D warnings -D clippy::collection_is_never_read
# code built only on macOS is guarded by these checks and the gates; CI builds on Linux (Sprint 247)
echo "pre-commit: the Linux check"
CARGO_TARGET_DIR=build/target-linux cargo check -q --target x86_64-unknown-linux-gnu --workspace --all-targets
cargo build -q --release -p design_check -p signal_report --bins
echo "pre-commit: design_check"
target/release/design_check
echo "pre-commit: the record check"
target/release/signal_report "$closed" --check
# the doc tests, which nextest runs none of (Sprint 292): every crate with a library, not the changed crates' alone,
# so the script keeps no selection of its own to drift from the gates' (Sprint 301)
libs=$(for m in crates/*/src/lib.rs; do printf -- '-p %s ' "$(basename "$(dirname "$(dirname "$m")")")"; done)
echo "pre-commit: the doc tests"
# shellcheck disable=SC2086
cargo test --quiet --doc $libs
