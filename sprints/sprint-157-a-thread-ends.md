# Sprint 157 — A thread ends: ExitThread, the exit code and the host's switch, and `sleeper` to ExitProcess

```yaml
---
id: 157
status: planned
phase: 6
pass_kind: functional
---
```

## scope

Link 5's run-loop judge, `sleeper` to its end (`docs/design-host-compile-path-2026-10-24.md` F.5). After Sprint 156
the Rust host's run of `tests/data/timed_waits/sleeper.exe` prints `woke\n` and stops at `hvc #8` op 30, thread
request 7 (THREAD_EXIT_CODE, from `WaitForSingleObject` on the spinner's handle). The Python host's run with one vCPU
goes on to ExitProcess with `woke\ndone\n`. Between the two lie three thread requests and one hand-over the Rust host
does not serve: THREAD_EXIT_CODE (7), THREAD_EXIT_STUB (2, ExitThread's detach stub), THREAD_END (3, W2mThreadEnd's
final exit code), and `hvc #12` (HVC_SCHEDULE) with SCHEDULE_END, the thread's end, which threads.c `w2m_switch`
leaves to the host. The Rust host now serves them as the Python host does, and `sleeper` runs to ExitProcess.

Carried from `src/w2m/vm_backend.py` and `src/w2m/scheduler.py` (read in full: vm_backend 3190-3245 (`_thread_request`),
3316-3330 (`current_tid`, `set_current`), 3463-3580 (`_held_elsewhere`, `_save_thread`, `_load_thread`, `_schedule`,
`_switch`), 2650-2700 (`_switch_stub`), 5020-5100 and 5285-5305 (`steps`' `Switched`, `serve_exit`'s HVC_SCHEDULE),
5129-5141 (`_before_run`); scheduler.py 20-200 (`GuestThread`'s status, `detach_stub`); `tools/guest_runtime/threads.c`
240-350; `tools/vm_monitor/vm_monitor.c` 440-490 (`vswap`)):

- `_thread_request` subs 2, 3 and 7: the detach stub (`detach_stub`, `_thread_callbacks`, W2mThreadEnd's thunk) in a
  code page; the current thread's exit code; a thread's exit code by id, refused by name for an id w2m did not create;
- `_schedule` with SCHEDULE_END: the current thread ended, every blocked thread ready (the run table's blocked count
  kept, as `GuestThread.status`'s setter keeps it), and the process's end when every thread has ended;
- `_switch` for one vCPU and one process: round robin over ready, unsuspended threads from the one after the current;
  SCHEDULE_ANSWER's x0; the current thread saved (`_save_thread`: its instruction and pause counts, registers, monitor
  words, state block, vCPU mark) and the next loaded (`_load_thread`), `set_current`, the vectors swapped as the
  monitor's `vswap` swaps them (Q0-Q15, FPCR, FPSR to the outgoing slot and from the incoming), the run resumed at the
  dispatcher. With no thread ready the switch refuses by name: the warp to a deadline and the 1 ms sleep come when a
  program needs them.
- the hypervisor crate's setter for a vector register (`hv_vcpu_set_simd_fp_reg`), whose value the header passes as a
  16-byte vector by value.

## prerequisites

- 156 (closed)

## context_files

- the Python and C above; `crates/w2m_vm/src/threads.rs`, `run.rs`, `host_request.rs`, `guest.rs`;
  `crates/hypervisor/src/lib.rs`; `crates/w2m_vm/tests/run_loop.rs`

## signal contract

### Emits

None this sprint: the skeleton's record comes at its gate (link 6), as the design orders; the run's stop is the test's
output.

### Consumes

`tests/data/timed_waits/sleeper.exe`; Sprint 156's fixture (`build/run_loop/sleeper-python.json`: stdout, stop,
translations); a second fixture from the frozen Python host's run of `sleeper` with one vCPU, its sha256 committed:
each thread request 2, 3 and 7 with its argument, answer and current thread, the detach stub's bytes, each hand-over's
schedule bits and the thread current after it, and the process's exit code.

### Invariants

- No Python file added or changed.
- `qpc`'s judge (Sprint 155) and Sprint 156's created-thread comparison pass unchanged.

## artifact contract

### Files created

- the second fixture's sha256 under `store/run_loop/`

### Files modified

- `crates/w2m_vm/src/threads.rs`, `host_request.rs`, `run.rs`, `crates/hypervisor/src/lib.rs`,
  `crates/w2m_vm/tests/run_loop.rs`, `WORKING_AGREEMENT.md` (the second fixture's generator), `BLACKBOARD.md` (Sprint
  156's CI run, in the tail entry)

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values, from the Python host:** the Rust run of `sleeper` stops at ExitProcess with code 0 and stdout
  `woke\ndone\n`, the Python's. Its thread requests 2, 3 and 7 are the Python's in order, with their arguments, answers
  and current threads (STILL_ACTIVE while the spinner runs, 0 after its end); the detach stub's bytes are the
  Python's; each `hvc #12` hand-over has the Python's schedule bits and leaves the Python's thread current. Each block
  the host compiles is a Python compile, start and end.
- **Planted fault:** the ended thread's waiters left blocked (no thread ready at the hand-over), or the exit code
  stored from the wrong argument: the comparison with the Python's names the request or the stop.

## done criteria

`sleeper` runs to ExitProcess in the Rust host with the Python host's output, thread requests and hand-overs.

## self-review checklist

- [x] Scope names one verifiable property: `sleeper` to ExitProcess as the Python host runs it.
- [x] context_files cite the Python and C carried.
- [x] Emits: none this sprint, with the design's reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the Python host's run as reference, and a planted fault.
- [x] One concept: a thread's end in the Rust host.
