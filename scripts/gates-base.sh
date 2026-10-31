#!/bin/sh
# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
# The commit the gates take the change from (Sprint 273; reviewer-3's High 1 at 272, KIT_DIARY #111): the newest
# commit, an ancestor of HEAD, whose subject opens "Sprint N closed:" or "Sprint N closed-partial:", the previous
# sprint's close, so a work commit pushed before its gates is still in the change; with no such commit, the merge base
# with origin/main, as before Sprint 273. The merge base alone was the pushed work commit itself, and the passes of 267
# to 272 selected from the uncommitted bookkeeping. Prints the commit's full id. Shared by scripts/gates.sh and
# signal_report's test.
set -eu
base=$(git log -1 --format=%H -E --grep='^Sprint [0-9]+ closed(-partial)?:' HEAD)
[ -n "$base" ] || base=$(git merge-base HEAD origin/main)
echo "$base"
