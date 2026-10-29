# Sprint 199 — The machine clock judged

```yaml
---
id: 199
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 198", reviewer-2's High 1, Mediums 3 and 5 and Low 1 of its review of Sprints
196-198. Sprint 197's machine clock has no run-level judge: the guests run on the wall clock, and the batch run's rows
0-48 read no child's clock. The Python judges it with twoproc on the instruction clock (tests/test_timed_waits.py 152,
"child ended") and plants a catch-up only on a resume ("clock behind", 141-152), the form the Rust child has. The
batch run is a machine from row 16 for good and meets Sleep(10) at rows 3049, 3063 and 3123, where an idle process's
warp is refused today.

- **The judge:** `w2m-vm` takes `--instruction-clock` for a guest (`family_run.INSTRUCTION_CLOCK`, 1,400,000,000 a
  second from 134362624920000000, as `run_guest` passes it); `gates.sh` runs twoproc on it, and `batch_run.rs` holds its
  output to "child ended".
- **Catch-up at a start:** a child is caught up to the machine's clock when its first turn begins, as on every resume
  (`_catch_up` in `VM.steps`' start, vm_backend 5055-5080).
- **Readiness when chosen:** each process posts its earliest deadline, not a bit; the supervisor computes a process's
  readiness when it chooses the turn: a thread ready, or the earliest deadline at or before the machine's clock, or a
  wake pending with a thread blocked (`ready()`, `_deadline_passed`, vm_backend 3653-3669).
- **The warp over the machine:** with no process ready on the instruction clock, the chosen timed process's clock is
  warped to its deadline and that thread made ready (`warp_to_deadline`, 3614-3628; `_next`, kernel.py 394-397).
- **The environment:** a child's environment is the parent's block at CreateProcessW (`environment_now`, 3599-3612).
- **No orphan, no held hang:** a child watches its parent's pid (kqueue EVFILT_PROC, NOTE_EXIT) and stops its vCPU and
  exits; `batch_run.rs` and `family_runs.rs` refuse a held stop that is "the run's time limit".
- **The round trip across processes:** measured once between a supervisor and a child `w2m-vm`, and recorded.
- **Edits:** no `sed` adds a line; each `sed` this sprint runs is quoted in its tail entry.

## prerequisites

- 198 (closed)

## context_files

- `tests/test_timed_waits.py` 51-62, 138-160; `src/w2m/vm_backend.py` 3599-3700, 5055-5080; `src/w2m/kernel.py` 380-436;
  `crates/w2m_vm/src/{machine.rs, turns.rs, run.rs, threads.rs, shared.rs}`, `src/bin/w2m-vm.rs`

## signal contract

### Emits

- None: the judges are `batch_run.rs` and unit tests.

### Invariants

- No Python file added or changed; the 12 guests that end still print the Python's output; the stream's rows 0-48 still
  equal.

## artifact contract

### Files modified

- `crates/w2m_vm/src/{machine.rs, turns.rs, run.rs, threads.rs, host_request.rs}`, `src/bin/w2m-vm.rs`,
  `crates/w2m_vm/tests/{batch_run.rs, family_runs.rs}`, `scripts/gates.sh`, `store/batch_run/first-stop-2026-10-28.tsv`

### Command exit codes

- `batch_run.rs`, `family_runs.rs`, the unit tests and `scripts/gates.sh` return 0

## observation contract

- **Expected values:** twoproc on the instruction clock prints "child ended"; the 12 guests as before; the batch run's
  stream as before; the round trip across two processes stated.
- **Planted faults:** (1) catch-up only on a resume, not at a child's start: twoproc on the instruction clock prints
  "clock behind", named by `batch_run.rs`; (2) the deadline term dropped from readiness: a judge named, or the plant's
  reach recorded; (3) a held stop of "the run's time limit": `batch_run.rs` refuses it.

## done criteria

The machine clock is judged at run level as the Python judges it, the Rust's readiness and warp are the Python's, and
no child outlives its supervisor.

## self-review checklist

- [x] Scope names one verifiable property: twoproc on the instruction clock ends as the Python's does.
- [x] context_files cite the Python to read first.
- [x] Emits: none, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the machine clock.
