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

## result

- **`sleeper` runs to ExitProcess in the Rust host.** The run stops at ExitProcess(0) with stdout `woke\ndone\n`, the
  frozen Python host's. Its thread requests are the Python's, in order: THREAD_EXIT_CODE for thread 8 from thread 4,
  answered 259 (STILL_ACTIVE); THREAD_EXIT_STUB from thread 8, answered 0x1000230000, the detach stub, whose bytes are
  the Python's; THREAD_END from thread 8 with code 0; THREAD_EXIT_CODE for thread 8 from thread 4, answered 0. Its one
  `hvc #12` hand-over is the Python's: schedule bits 4 (SCHEDULE_END), from thread 8 to thread 4. Exits: 14 misses, 8
  host requests, 1 hand-over. The C compiled 14 blocks and the Rust host 5; each start is a Python host compile's, and
  each host compile's (start, end) is the Python's.
- `crates/w2m_vm/src/threads.rs`: `detach_stub`, `exit_stub`, `end_thread`, `exit_code`, `set_status` (with the run
  table's blocked count), `schedule` and `switch` (`_save_thread`, `set_current`, `_load_thread` for vCPU 0), and
  `Switch`. `run.rs` serves `hvc #12` when the host compiles, swaps the vectors as the monitor's `vswap` does, and
  records each thread request 2, 3 and 7 and each hand-over in `Outcome`; a hand-over it cannot serve stops as
  `Stop::Handover` with the reason. With no thread ready the switch refuses by name; the warp to a deadline and the 1 ms
  sleep are not carried.
- **A declaration stable Rust refuses.** `hv_vcpu_set_simd_fp_reg` takes its value as `ext_vector_type(16) uint8_t` by
  value, which AAPCS64 passes in v0. Stable Rust rejects `uint8x16_t` in an `extern "C"` signature (the `simd_ffi`
  feature is nightly-only), so `Vcpu::set_simd` loads q0 and calls the function from inline assembly.
  `crates/hypervisor/tests/vector_register.rs` sets Q1 from the host, has the guest copy it to Q0, and reads Q0 back.
- **Planted faults.** The ended thread's waiters left blocked: the run stopped at `Stop::Handover` ("every thread waits
  with no timeout and nothing can wake one (a deadlock)"), and the judge failed at the thread requests, the fourth
  missing. THREAD_END's code taken from the sub-request word (3) and not its argument: the judge failed at the thread
  requests, the last answered 3 where the Python answered 0. Restored, it passes.
- The second fixture: `build/run_loop/sleeper-end-python.json`, its sha256 in
  `store/run_loop/sleeper-end-python.sha256`, its generator in WORKING_AGREEMENT; two runs gave the same bytes.
- Rule breach, recorded: WORKING_AGREEMENT's three changed lines were written by an inline script, not the Edit tool;
  its diff was read and holds only those lines.
- The first gates run failed at `cargo fmt --check` (`run.rs`, `threads.rs`, `run_loop.rs`); `cargo fmt` made the
  changes. The second failed at clippy's `large_enum_variant`: `Switch::Go` carried 31 registers beside `Ended`'s
  u32. `schedule` now leaves the next thread's registers in the `x` it is handed, and `Go` keeps only the vector
  slots. The gates ran a third time from the start.
