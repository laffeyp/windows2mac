#!/bin/sh
# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
# The gates' nextest filter for a change to the crates named (Sprint 322; Deferred, Sprint 321): each crate's
# `rdeps()`, the tests of every crate that depends on it, and every line of `scripts/tree-tests.txt`, the tests that
# read beyond their own crate, which `rdeps()` alone skips when no crate they depend on changed (Sprint 320's two upper
# time bounds passed its gates so and failed CI). The one place the filter is built; `scripts/gates.sh` calls it.
#
# Usage: scripts/test-filter.sh <crate>...
set -eu
cd "$(dirname "$0")/.."
filter=""
for crate in "$@"; do
  filter="$filter${filter:+ | }rdeps($crate)"
done
while IFS= read -r line; do
  case $line in
    '' | '#'*) continue ;;
  esac
  filter="$filter${filter:+ | }($line)"
done < scripts/tree-tests.txt
printf '%s\n' "$filter"
