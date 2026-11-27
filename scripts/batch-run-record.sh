#!/bin/sh
# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
# Copyright (C) 2026 Peter Laffey
# The batch run's record with the machine's state at its start in its baseline (Sprint 240; reviewer-3's Medium 1 of
# its review at 240: a speed figure read beside another run's needs both runs' conditions on their records). As
# run_frontier.py's `host_load` (118-148 at the tag frozen-python-2026-10-29) put them in its baseline: `load_at_start`,
# the load averages to two places, the processes outside this run's tree above 50% of a CPU ("busy") and the w2m
# processes outside it ("other w2m"), as one string, since a run file's baseline holds text (an inline table became
# Python's repr of a dict on the record); and, for the Drift watchlist's "Absolute timings drift between runs", the QoS
# class the run inherits (`w2m-vm --qos-class`) and the cores, which no process on Apple silicon can pin
# (`thread_policy_set` with THREAD_AFFINITY_POLICY returns KERN_NOT_SUPPORTED). Substrate reads a run file's baseline
# and runs its Producers from the file's directory, so the file with these fields is written beside
# harness/w2m-vm-game.toml, as harness/.w2m-vm-game-<pid>.toml (ignored by git), and removed after the run.
#   scripts/batch-run-record.sh <root> [--time-compile] [--instructions-at-compile] [--cache <directory>] [--vcpus <n>]
# With --time-compile (Sprint 247), a measurement run: `w2m-vm` is given the flag, its monitor code times the miss
# stub's calls of the in-VM compiler, the record carries compile_critical_path_ms, and the baseline says so; the gates
# run without it, so the monitor code they judge is the frozen host's word for word; a timed run also leaves out
# --judge-blocks, whose host time would enter the rate (reviewer-3's Medium 1 at 257), and --exit-ledger, whose hashing
# at each exit weighs on the run with more exits (reviewer-3's Medium 1 at 261). With --cache (Sprint 257), the
# host installs UnityPlayer.dll's blocks from the translation cache in the directory (relative to the root), whose file
# `w2m-vm --make-cache` made under the same tree, and the baseline says so. With --instructions-at-compile (Sprint
# 266), the monitor code writes the instructions retired at each in-VM compile and install, the record carries the
# most of each in a 60 Hz frame of instructions, and the baseline says so; the count is the guest's, so the run need
# not be timed or run alone. With --vcpus <n> above 1 (Sprint 368; census G3), the game on n vCPUs on the wall clock
# (`w2m-vm --vcpus`, Sprint 367): the command leaves out --exit-ledger and --judge-blocks, which `w2m-vm` refuses above
# one vCPU ("reads every exit in order"; the one-vCPU run keeps both and stays the record they judge), its files are
# named first-stop-<n>-vcpus and stdout-<n>-vcpus.txt beside the one-vCPU run's, call_compare judges those, and the
# baseline says so; with 1, the command as without it.
set -eu
root=$1
shift
# the record's own directory, new, as gates.sh names it (build/batch_run/record-<ns>): `.` wrote a record into the
# repository root at Sprints 247 and 266
if [ -e "$root" ]; then
  echo "batch-run-record.sh: $root exists; the record's directory must be a new one, such as store/runs/w2m-vm-game-<ns>" >&2
  exit 2
fi
timed=
counted=
cache=
vcpus=1
while [ $# -gt 0 ]; do
  case $1 in
    --time-compile) timed=--time-compile; shift ;;
    --instructions-at-compile) counted=--instructions-at-compile; shift ;;
    --cache) [ $# -ge 2 ] || { echo "batch-run-record.sh: --cache <directory>" >&2; exit 2; }; cache=$2; shift 2 ;;
    --vcpus)
      [ $# -ge 2 ] || { echo "batch-run-record.sh: --vcpus <count>" >&2; exit 2; }
      case $2 in ''|*[!0-9]*|0*) echo "batch-run-record.sh: --vcpus $2 is not a count of 1 or more" >&2; exit 2 ;; esac
      vcpus=$2; shift 2 ;;
    *) echo "batch-run-record.sh: $1 is none of --time-compile, --instructions-at-compile, --cache, --vcpus" >&2; exit 2 ;;
  esac
done
# a timed run measures the one-vCPU run the gates judge; the run on several vCPUs is judged, not timed
if [ "$vcpus" -gt 1 ] && [ -n "$timed$counted$cache" ]; then
  echo "batch-run-record.sh: --vcpus $vcpus is run with none of --time-compile, --instructions-at-compile, --cache" >&2
  exit 2
fi
several=
[ "$vcpus" -gt 1 ] && several="--vcpus $vcpus"
# this script and its ancestors, which the run's tree is; the run's own processes start after the reading
mine=" $$ "
p=$$
while [ "$p" -gt 1 ]; do
  p=$(ps -o ppid= -p "$p" | tr -d ' ')
  mine="$mine$p "
done
rows=$(ps -axo pid=,%cpu=,command= | awk -v mine="$mine" 'index(mine, " " $1 " ") == 0')
# a w2m process by its executable, as `is_w2m` takes it (not by text a shell's command holds)
other=$(printf '%s\n' "$rows" | awk '{
    n = split($3, path, "/"); exe = path[n]
    if (exe ~ /^(vm_monitor|w2m-vm|class_judge|case_runs|call_compare|run_measurements)$/) {
      line = exe; for (i = 4; i <= 5 && i <= NF; i++) { m = split($i, w, "/"); line = line " " w[m] }
      gsub(/["\\]/, "", line); print line
    }
  }' | sort | paste -sd, - | sed 's/,/, /g')
busy=$(printf '%s\n' "$rows" | awk '$2 > 50 { n = split($3, path, "/"); name = path[n]; gsub(/["\\]/, "", name)
    printf "%d\t%s %d%%\n", $2 + 0.5, name, $2 + 0.5 }' | sort -rn | cut -f2 | paste -sd, - | sed 's/,/, /g')
busy=${busy:-none}
other=${other:-none}
load=$(sysctl -n vm.loadavg | tr -d '{}' | awk '{ printf "%.2f %.2f %.2f", $1, $2, $3 }')
qos=$(target/release/w2m-vm --qos-class)
file=harness/.w2m-vm-game-$$.toml
trap 'rm -f "$file"' EXIT
flags="$timed${counted:+ $counted}${cache:+ --cache $cache}${several:+ $several}"
flags=${flags# }
awk -v load="$load" -v busy="$busy" -v other="$other" -v qos="$qos" -v timed="$timed" -v counted="$counted" -v cache="$cache" -v flags="$flags" -v vcpus="$vcpus" '
  flags != "" && /^command = \["sh", "-c", "exec \.\.\/scripts\/sign_and_run\.sh \.\.\/target\/release\/w2m-vm --game / {
    sub(/"\]$/, " " flags "\"]")
    if (timed != "" || vcpus > 1) sub(/ --judge-blocks [^ "]*/, "")
    if (timed != "" || vcpus > 1) sub(/ --exit-ledger [^ "]*/, "")
  }
  # the run on several vCPUs writes its stop, its companions and its output beside those of the one-vCPU run, and
  # its baseline names its vCPUs
  vcpus > 1 && /^command = / {
    gsub(/build\/batch_run\/first-stop/, "build/batch_run/first-stop-" vcpus "-vcpus")
    gsub(/build\/batch_run\/stdout\.txt/, "build/batch_run/stdout-" vcpus "-vcpus.txt")
  }
  vcpus > 1 && /^implementation = / { sub(/on one vCPU/, "on " vcpus " vCPUs on the wall clock") }
  { print }
  /^\[baseline\]$/ {
    printf "load_at_start = \"load averages %s; busy: %s; other w2m: %s\"\n", load, busy, other
    printf "qos = \"%s, inherited from the shell, not set by w2m\"\n", qos
    print "cores = \"not pinned: Apple silicon refuses THREAD_AFFINITY_POLICY (KERN_NOT_SUPPORTED)\""
    if (timed != "") print "compile_timing = \"the miss stub'"'"'s calls of the in-VM compiler timed by the monitor code (--time-compile)\""
    if (timed != "") print "block_judge = \"off: a timed run does not judge the in-VM blocks, whose host time would enter its rate\""
    if (timed != "") print "exit_ledger = \"off: a timed run writes no exit ledger, whose hashing at each exit would weigh on the run with more exits\""
    if (counted != "") print "instructions_at_compile = \"the instructions retired at each in-VM compile and install written by the monitor code (--instructions-at-compile)\""
    if (cache != "") print "translation_cache = \"UnityPlayer.dll'"'"'s blocks installed by the host from " cache " (--cache)\""
    if (vcpus > 1) print "vcpus = \"" vcpus " (w2m-vm --vcpus " vcpus "), on the wall clock; no exit ledger and no block judge, which read every exit in order\""
  }' harness/w2m-vm-game.toml > "$file"
if [ -n "$flags" ] && ! grep -q -- " $flags\"]\$" "$file"; then
  echo "batch-run-record.sh: the run file's w2m-vm command did not take $flags" >&2
  exit 2
fi
if [ -n "$timed" ] && grep -q -- '--judge-blocks' "$file"; then
  echo "batch-run-record.sh: the timed run's command still judges the blocks" >&2
  exit 2
fi
if [ -n "$timed" ] && grep -q -- '--exit-ledger' "$file"; then
  echo "batch-run-record.sh: the timed run's command still writes the exit ledger" >&2
  exit 2
fi
if [ "$vcpus" -gt 1 ] && { grep -q -- '--exit-ledger\|--judge-blocks' "$file" || grep -q 'build/batch_run/first-stop\.\|build/batch_run/stdout\.txt' "$file" \
  || [ "$(grep -c "build/batch_run/first-stop-$vcpus-vcpus\." "$file")" != 2 ]; }; then
  echo "batch-run-record.sh: the $vcpus-vCPU run's commands still diagnose every exit or name the one-vCPU run's files" >&2
  exit 2
fi
uv run substrate run --file "$file" --root "$root" --strict
