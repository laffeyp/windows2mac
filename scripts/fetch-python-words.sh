#!/bin/sh
# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
# The frozen Python lowering's words fixture into build/case_runner/python-words.txt (Sprint 228; reviewer-3's Medium 2
# of its review at Sprint 226: its generator left the tree, and this Mac held the only copy). The fixture is the asset
# of release python-words-2026-10-29; it is downloaded beside the destination and moved there only once its sha256
# equals store/case_runner/python-words.sha256, the pin `Fixture::load` checks too.
set -eu
cd "$(git rev-parse --show-toplevel)"
mkdir -p build/case_runner
tmp=$(mktemp -d build/case_runner/fetch.XXXXXX)
trap 'rm -rf "$tmp"' EXIT
gh release download python-words-2026-10-29 --pattern python-words.txt --dir "$tmp"
echo "$(cat store/case_runner/python-words.sha256)  $tmp/python-words.txt" | shasum -a 256 -c -
mv "$tmp/python-words.txt" build/case_runner/python-words.txt
