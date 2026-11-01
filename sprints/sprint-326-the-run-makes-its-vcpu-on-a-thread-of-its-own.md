# Sprint 326 — The run makes its vCPU on a thread of its own

```yaml
---
id: 326
status: done
phase: 6
pass_kind: functional
---
```

## scope

The run-loop design's "The threads": the run creates a host thread per vCPU, each making its own vCPU, running it and
serving its exits. A `Vcpu` stays on the thread that made it (`hypervisor`'s `Vcpu`, neither `Send` nor `Sync`; `Vm`
is `Sync`, and `hv_vcpu_create` makes a vCPU for the calling thread). Today each of the run's three callers makes the
one vCPU on its own thread and hands it in: `w2m-vm.rs` `on_one_vcpu` (with a timer thread that stops it at the run's
time limit through its handle), `tests/run_loop.rs` `run`, `tests/first_decline.rs`. Card 4c3b2b's second vCPU needs
the run to make its vCPUs; this card moves the one vCPU's making into the run first, so 4c3b2b changes how many it
makes and not where.

- **The work:** `run_compiling` and `run_to_first_decline` take the `&Vm` and the run's time limit
  (`Option<Duration>`) in place of a `&Vcpu`; `run` makes the vCPU on a thread inside `std::thread::scope`, sets it up
  and runs `vcpu_loop` there, the time limit's timer stopping that vCPU through its handle as `on_one_vcpu`'s did; the
  outcome and the borrows given back after the scope; the three callers make the `Vm` alone.
- **Not here:** a second vCPU, the stop of every other vCPU, the merged outcome (4c3b2b).

## prerequisites

- 325 (closed)

## context_files

- `docs/design-run-loop-on-several-vcpus-2026-10-31.md` "The threads" (154-165); `crates/hypervisor/src/lib.rs` `Vm`
  and `Vcpu` (248-445); `crates/w2m_vm/src/run.rs` `run`, `run_compiling`, `run_to_first_decline`;
  `crates/w2m_vm/src/bin/w2m-vm.rs` `on_one_vcpu` (523-560); `tests/run_loop.rs` 139-145; `tests/first_decline.rs`

## signal contract

### Emits

- None; where the run's vCPU is made.

### Invariants

- Every run as held.

## artifact contract

### Files modified

- `crates/w2m_vm/src/run.rs`, `crates/w2m_vm/src/bin/w2m-vm.rs`, `crates/w2m_vm/tests/run_loop.rs`,
  `crates/w2m_vm/tests/first_decline.rs`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** a test running `qpc` with a time limit of 0 s, the run stopping as `Stop::TimeLimit` (no test
  holds the limit today; `Stop::TimeLimit`'s one reader is `w2m-vm.rs` 575); the same guest with no limit ending as
  held; every run as held. That the vCPU is made and run on one thread is the compiler's: `Vcpu` is not `Send`.
- **Planted faults:** (1) the timer not started: the 0-s run ends as `ExitProcess`, and its test fails naming the
  stop; (2) the vCPU made on the caller's thread and moved into the scope's: the build fails, `Vcpu` not `Send`
  (E0277). Each judge's reach to its planted line read before this card's commit (WORKING_AGREEMENT, Sprint 284):
  (1) the timer is started inside the `run` the test calls; (2) the move is a spawn's closure capture, which the
  compiler checks for `Send`.

## done criteria

The run makes and runs its vCPU on a thread of its own, its callers handing it the VM.

## self-review checklist

- [x] Scope names one verifiable property: the vCPU made and run on the run's own thread.
- [x] context_files cite the design, the hypervisor's types and the three callers.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each judge's reach read.
- [x] One concept: where the vCPU is made.

## result

- **The run** (`crates/w2m_vm/src/run.rs`): `run_compiling` and `run_to_first_decline` take `vm: &Vm` and
  `limit: Option<Duration>` in place of `vcpu: &Vcpu`; `run` opens a `std::thread::scope`, whose thread makes the vCPU
  (`Vcpu::new(vm)`) and calls `run_on_vcpu`, the set-up, the loop and the outcome as before; a panic on it is resumed
  on the caller's. With a limit, a second scoped thread waits on a channel with `recv_timeout(limit)` and, on the
  timeout, stops the vCPU through its handle; the run's thread drops the channel's sender at its end, so the timer
  leaves then and no stop outlives the run (`on_one_vcpu`'s timer slept the whole limit, detached).
- **The callers:** `w2m-vm.rs` `on_one_vcpu` hands the `Vm` and `seconds` as a `Duration`; `tests/run_loop.rs`' `run`
  calls the new `run_limited` with no limit; `tests/first_decline.rs` hands the `Vm`. The two other vCPUs the crate
  makes (`w2m-vm.rs` 101, the WFE probe; `run_loop.rs` 435, the installer's direct run) do not run the loop and stay.
- **The test** `run_loop::the_runs_time_limit_stops_its_vcpu`: `qpc` with a limit of 0 s stops as `Stop::TimeLimit`,
  and with none ends `ExitProcess { code: 0 }`; no upper bound on the wall clock. `run_loop_lock.rs` reads
  `run_on_vcpu` where it read `run`, the call to `vcpu_loop(&lock, …)` now there.
- **Every run as held:** `cargo nextest run -E 'rdeps(w2m_vm)'`, 196 tests passed in 410 s; `cargo clippy` with the
  gates' flags clean.
- **Plant 1,** `run.rs` copied aside and restored by `cp`, checked by `cmp`: the timer's `let _ = handle.stop();` made
  `let _ = &handle;`; the test failed, "assertion `left == right` failed: qpc with a limit of 0 s", left
  `ExitProcess { code: 0 }`.
- **Plant 2,** restored likewise: `let vcpu = Vcpu::new(vm)?;` moved before `scope.spawn(move || …)`; `cargo check`
  (its own target directory, `build/target-plant`, beside a test run) failed, "error[E0277]: `*const
  hypervisor::ffi::HvVcpuExit` cannot be sent between threads safely".
- **The gates:** `scripts/gates.sh` exit 0 in 1203 s, kept in `store/gates/passes.tsv` as sprint 326 at e57889444 with root=6268904bc and "rust=crates python=0 samples=0", 241 tests (the `w2m-vm` binary's change selecting its dependants), 652 ns an exit at load 3.29, base=a4eac33ab, Sprint 325's close; multi-vCPU: not run (Python gates not run)
