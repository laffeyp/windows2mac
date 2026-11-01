#!/bin/sh
# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
# A work or card commit, owned by its checks (Sprint 317; reviewer-3's Low 1 at 316): `scripts/pre-commit.sh`, then
# `git commit` with this script's arguments, only when the checks passed on the index that is committed. Seven commits
# of Sprints 312 to 315 ran the two joined by ';' (one, 44b4730bc, over a failed format check), the form Sprint 281
# committed and WORKING_AGREEMENT's Sprint 281 amendment forbids; the separator now lives here alone.
#
# Refused, by name, with no commit: a tracked file whose working copy differs from the index (the checks read the
# working tree, the commit takes the index); a failed check (its status); an index changed while the checks ran.
#
# Usage: scripts/commit.sh -m "message" [git commit's other arguments], with the commit's files staged first.
set -eu
cd "$(dirname "$0")/.."

if ! git diff --quiet; then
  echo "commit: tracked files differ from the index, which the checks would not judge: $(git diff --name-only | tr '\n' ' ')"
  exit 1
fi
tree=$(git write-tree)
status=0
scripts/pre-commit.sh || status=$?
if [ "$status" -ne 0 ]; then
  echo "commit: scripts/pre-commit.sh exited $status; no commit"
  exit "$status"
fi
if [ "$(git write-tree)" != "$tree" ] || ! git diff --quiet; then
  echo "commit: the index or the tree changed while the checks ran; no commit"
  exit 1
fi
git commit "$@"
