#!/bin/sh
# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
# The tree the gates test, printed: the work tree staged into a temporary index, less the session's own .claude/ and
# the bookkeeping paths a close commit may change (Sprint 192), keeping the documents design_check reads (Sprint 195;
# reviewer-2's Low 4 of its review of 190-192). `scripts/gates.sh` writes it to its pass line and `signal_report`'s
# test takes it the same way (reviewer-2's Medium 4: the test had copied the steps); `signal_report --check` rebuilds
# it from the work commit with the same lists (crates/signal_report/src/main.rs BOOKKEEPING, DESIGN_DOCUMENTS).
set -eu
index=$(mktemp)
rm -f "$index"
GIT_INDEX_FILE=$index git add -A -- . ':!.claude'
GIT_INDEX_FILE=$index git rm -r -q -f --cached --ignore-unmatch -- BLACKBOARD.md KIT_DIARY.md WORKING_AGREEMENT.md \
  sprints signal-reports docs store/gates ':(exclude)docs/design-*' ':(exclude)docs/solution-*' \
  ':(exclude)docs/recentering-*' ':(exclude)docs/architecture-*' ':(exclude)docs/*plan-*'
GIT_INDEX_FILE=$index git write-tree
rm -f "$index"
