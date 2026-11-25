#!/bin/sh
# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
# The commit the gates take the change from (Sprint 273; reviewer-3's High 1 at 272, KIT_DIARY #111): the newest
# commit, an ancestor of HEAD, whose subject opens "Sprint N closed:" or "Sprint N closed-partial:", the previous
# sprint's close, so a work commit pushed before its gates is still in the change; with no such commit, the merge base
# with origin/main, as before Sprint 273. The merge base alone was the pushed work commit itself, and the passes of 267
# to 272 selected from the uncommitted bookkeeping. Prints the commit's full id. Shared by scripts/gates.sh and
# signal_report's test.
#
# From Sprint 334 a sprint closes in its work commit (Sprint 333; the Decision "A sprint's close folds into its sprint's
# commit", 2026-11-24): a commit "Sprint N: ..." with N from 334 whose tree holds card N `status: done` or
# `status: closed-partial` is a close too, as `close_commit` in crates/signal_report/src/main.rs takes it
# (FIRST_FOLDED_SPRINT); a work commit pushed before its gates holds its card active and is passed over.
set -eu
# the first close among "<hash> Sprint N ..." lines read in `git log` order, printed (a function, since this Mac's
# /bin/sh, bash 3.2.57, misreads a `case` pattern's ")" inside "$( )")
first_close() {
  while read -r hash _ n rest; do
    case "$rest" in closed:* | closed-partial:*)
      echo "$hash"
      return
      ;;
    esac
    case "$n" in *:) ;; *) continue ;; esac
    k=${n%:}
    [ "$k" -ge 334 ] || continue
    card=$(git ls-tree --name-only "$hash" sprints/ | grep "^sprints/sprint-$(printf %03d "$k")-.*\.md\$" | head -1)
    if [ -n "$card" ] && git show "$hash:$card" | grep -Eq '^status: (done|closed-partial)$'; then
      echo "$hash"
      return
    fi
  done
}
base=$(git log --format='%H %s' -E --grep='^Sprint [0-9]+(:| closed(-partial)?:)' HEAD | first_close)
[ -n "$base" ] || base=$(git merge-base HEAD origin/main)
echo "$base"
