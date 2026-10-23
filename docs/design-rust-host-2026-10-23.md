# The Rust host: who owns each piece of state, and which lock guards it (Sprint 124)

Step 1 of `docs/rust-port-plan-2026-10-21.md`, written before any host code. The Python host, frozen at
`python-final`, keeps 202 pieces of state in the classes `VM`, `Vcpu`, `Channel`, `Machine`, `Process`, `Event`,
`Pipe`, `GuestThread`, `FileBridge`, `VirtualMemory` and `Reservation`: each attribute assigned on `self`, and each
dataclass field. This document places each one: owned by one vCPU's thread, shared under a named lock, set once before
the first run, or removed with the Python plumbing that needed it. The table at the end has one row per attribute;
since the Python tree is frozen, the list cannot grow, and the table is the record. Two requirements from BLACKBOARD
Decision 2026-10-23 shape it, and a feasibility section measures what the design asks of this Mac (M5 Max, macOS
26.5.1, rustc 1.99.1).

## What is known first

- **One VM per macOS process.** hv_vm.h: `hv_vm_create` "Creates a VM instance for the current process";
  `hv_vm_destroy` "Destroys the VM instance associated with the current process". So each guest process runs in a
  macOS process of its own, as each `vm_monitor` does today.
- **One vCPU per thread, and its calls from that thread.** hv_vcpu.h: `hv_vcpu_create` "Creates a vCPU instance for
  the current thread"; "Each thread can only have one vCPU associated at a time". `hv_vcpus_exit` may be called from
  any thread, and a vCPU not running returns at once from its next run (`docs/design-several-vcpus-2026-10-17.md`).
- **Rust's `std::sync::Mutex` on macOS is a pthread mutex** (rust-lang/rust 1.99.1,
  `library/std/src/sys/sync/mutex/mod.rs`: `target_family = "unix"` selects `pthread::Mutex`).
- **Apple's pthread mutex is first-fit unless asked otherwise.** pthread_mutexattr(3): since macOS 10.14
  `PTHREAD_MUTEX_POLICY_FIRSTFIT_NP` is "the default mode for mutexes initialized without a policy attribute set";
  under it "new contending acquirers may obtain ownership of the mutex ahead of existing waiters".
  `PTHREAD_MUTEX_POLICY_FAIRSHARE_NP` "guarantees that ownership of a contended mutex will be granted to waiters on a
  strictly ordered first-in, first-out basis". `std` sets no policy.
- **The guest runtime lock is a word in guest memory** (`RUN_TABLE + RUNTIME_LOCK`, vm_backend.py `RuntimeLock`). The
  guest holds it only inside calls into the runtime's C, taken at `lock_runtime`'s sites, and the host takes it around
  a service when several vCPUs run. `threads.c` reads the run table (`count`, `blocked`, `waiting`, `in_vm`) only under it, and the host writes
  `in_vm` and `waiting` under it (`_before_run`, `_count_waiting`). It stays the guest's lock until step 8 moves the
  guest runtime to Rust.

## The shape

**A `w2m-vm` process per guest process.** It creates the VM, maps guest memory, and starts one native thread per vCPU.
Each thread creates its vCPU, calls `hv_vcpu_run` itself and serves its own exits; no other thread runs a vCPU or
serves an exit for it. The `vm_monitor` subprocess, its text pipe, the shared-memory channel and its kicker thread go:
the host is now the process that owns the VM. Guest memory is anonymous memory of that process, so it has no backing
file (requirement 2).

**A supervisor process, `w2m`, holds what processes share:** the process table, named events, pipes, handles one
process holds on another's objects, and the run's stop. It is the Python `kernel.Machine` without its scheduling.
A `w2m-vm` reaches it through one shared-memory request page per vCPU thread: the thread writes the request, bumps a
sequence word and wakes the supervisor (`os_sync_wake_by_address_any`, shared); the supervisor answers the same way.
That is the channel `vm_monitor.c` already uses between Python and C, measured below between two Rust processes.

**What disappears with the Machine's thread.** In Python one thread, the Machine's, ran vCPU 0 of every process in
turn: a process handed it on (`IDLE`, `Slice`), a wake from another process was left pending (`_wake_pending`) and the
running process kicked so it would notice (`kick_vcpu0`). Every process now runs its own vCPUs at once. A wake from
the supervisor reaches the woken process's scheduler directly; a vCPU of that process waiting with no thread wakes on
its condition variable; a vCPU running guest code with no slice end is stopped with `hv_vcpus_exit` from the thread
that delivers the wake. The round robin, `Process.steps`, `Machine._running` and the slice hand-overs between
processes have no counterpart: 40 attributes of the table are `removed`.

**Layers.** Each Rust struct belongs to one of the five layers `docs/plan-2026-10-09.md` step 3 set: `cpu` (vCPU
threads, the code cache, the stubs), `memory` (guest memory, page tables, VirtualAlloc), `nt` (threads, the scheduler,
files, modules, the runtime lock), `host` (the process's identity, its output, its end), `diagnostics` (counters,
the call trace, the census at close), plus `supervisor` in the other process. No struct holds more than one layer.

## The locks

Every shared piece of state has one owner struct and one lock. A lock is held only for the access, never across
`hv_vcpu_run`, never across a request to the supervisor unless the guest's runtime lock already spans it (below).

| Lock | Guards (rows in the table) | Taken by | Held for | Policy |
|---|---|---|---|---|
| `code` | the code cache: blocks, bridges, code pages, lift and flag caches, translation cache, census (19) | a vCPU thread at a dispatch miss, a code write or a fault | one lookup, or one block's compile and insert | FAIRSHARE |
| runtime (guest word) | the run table and its threads, the code cursor the in-VM compiler bumps (5) | the guest's C on every runtime call; a vCPU thread around a service | one runtime call or service | the guest's spin (unchanged until step 8) |
| `vm_map` | VirtualAlloc reservations, image protections, pristine pages, stacks (8) | a vCPU thread serving VirtualAlloc, VirtualProtect, a guard-page fault, an image map | one reservation change and its page-table writes | FAIRSHARE |
| `modules` | the loaded images, the module table, a DllMain stub due (3) | a vCPU thread serving LoadLibrary, GetProcAddress, an unresolved import | one module's map or one lookup | FAIRSHARE |
| `files` | descriptors, read-only and recorded descriptors, checked copies (5) | a vCPU thread serving a file request | one host file call | FAIRSHARE |
| `output` | the guest's standard output (1) | a vCPU thread serving WriteFile to a standard handle | one append | default |
| `trace` | the call trace's bound images and thunks (3) | a vCPU thread at a traced GetProcAddress | one insert | default |
| `sched` | parked vCPUs, the change count, a wake not yet applied (3), with its condition variable | a vCPU thread parking or notifying; the thread delivering a wake | one count change and notify | FAIRSHARE |
| `vcpu_handle` | a vCPU's kick against its end (3) | any thread kicking; the vCPU's thread at its end | one flag read and `hv_vcpus_exit` | default |
| `kernel` (supervisor) | processes, events, pipes, names, handles held across processes (15) | the supervisor's request threads | one request | FAIRSHARE |

Set-once values that one thread writes and others only test become atomics or `OnceLock`: the stop reason
(`VM._stopping`, `Machine.stop_requested`), the exit code, and the first unimplemented call, so the first writer wins
and a second write is refused, not lost (`VM.unimplemented` today keeps whichever write lands last; finding 2 below).

**Lock order.** runtime (guest word) → `modules` → `vm_map` → `code` → `files` → `output` → `trace` → `sched` →
`vcpu_handle`. A thread holding one takes only those to its right. The order follows the calls: a runtime service
(the guest holds the runtime word) may map a module, which reserves memory, which may drop compiled code on the pages it
changes; files, output and the trace are leaves; parking and kicking come last and take nothing. `kernel` lives in
the other process: the supervisor holds it for one request, makes no request to a `w2m-vm` while holding it, and sends
a wake only after releasing it, so no wait crosses the process boundary in both directions. Each lock is a Rust type
that is reachable only through its guard, and the order is enforced in debug builds by a per-thread record of the
highest lock held (a level check in the guard's constructor).

**Policy.** Apple's default first-fit lets a thread that releases a lock and takes it again move ahead of a thread
already waiting. Measured below, that starves a waiter for tens of milliseconds when the other thread holds the lock
most of the time. A lock that two vCPU threads both take on their exit paths is created with FAIRSHARE through
`pthread_mutexattr_setpolicy_np`. `std` does not expose it, so the Rust host declares the four pthread calls itself
(as it declares Hypervisor.framework's): an FFI surface added to the bridge mapping in WORKING_AGREEMENT before
step 2's code, no crate.

## Requirement 1: a thread that serves one vCPU never slows another's

Measured on the Python host, 2026-10-22 (BLACKBOARD Decision 2026-10-23). In `crosswake` on two vCPUs, the child
process's start took 3.4 to 6.8 s in 17 of 30 runs against 0.22 to 0.33 s in the rest. The Machine's thread spent it
reading the 51 runtime C files (`guest_runtime._texts`); each read gave up the GIL, and vCPU 1's host thread, serving
the parent's `WaitForSingleObject(up, 0)` poll at about 90,000 exits a second, held it nearly always. With the texts
read once, 30 of 30 runs ended under 1.01 s.

Rust has no GIL, but the same starvation comes back with any lock one thread holds most of the time. Measured with a
pthread mutex beside a thread that takes it about 86,000 times a second, holding it 10 µs of each 11 µs: under the
default first-fit policy an occasional taker waited a median 7.5 and 13.4 ms and at most 105 and 122 ms, getting the
lock 199 and 157 times in 3 s (two runs); under FAIRSHARE it waited a median 5.1 and 6.4 µs and at most 53 and
123 µs, about 28,000 times. With a 1 µs hold both policies kept the median at or under 83 ns and the maximum under
39 µs. The design therefore holds:

- no lock across `hv_vcpu_run`, and none across a guest's run of code;
- each lock for one access, short against the time between exits (the feasibility section bounds it);
- per-vCPU counters with no lock: each vCPU thread owns its `diagnostics::VcpuCounters` and they are summed when the
  threads join (today 17 counters are shared and one, `exit_ns`, counts vCPU 0 only; `monitor_commands` loses increments, finding 3
  below);
- FAIRSHARE for each lock on two vCPU threads' exit paths;
- no work at a process's start on another process's path: a child starts in its own `w2m-vm`, so nothing it reads
  reaches its parent's threads; the runtime's sources are read once, at build, not per run.

## Requirement 2: every VM is closed and its backing file removed when its process ends

Measured on the Python host, 2026-10-22: `kernel.Machine.run` removes an ended child from `vms` and frees its handles
(kernel.py 345-351), and its `finally` closes the `steps` generators (366-368), but no line closes a child VM; only
the drivers do (`family_run.py` 2943-2947, `run_frontier.py` 359-362). So every run of a test that starts a child left
its `w2m-vm-*` file, 550 GB sparse: two `crosswake` runs added two, and the user's temp directory held 1,846 of them,
88 GB on disk.

The Rust host has no backing file: guest memory is anonymous memory of the `w2m-vm` process, freed by the kernel when
the process exits, whatever way it exits. The process owns its VM (`hv_vm_destroy` after every vCPU thread has
destroyed its vCPU, as hv_vm.h requires) through a guard whose `Drop` runs on every return path. The supervisor reaps
each `w2m-vm` it started (`waitpid`) and records its exit status; a run's end waits for all of them. The translation
cache file per image stays, written whole to a temporary name and renamed, so a killed process leaves no partial file.

## What the Python host got wrong that the Rust types must make impossible

Found by reading the code for this design (scratch classifications, 2026-10-23); none was seen to fail a run.

1. **`VM.threads` iterated with no lock while another thread inserts.** `_seconds_to_deadline` (vm_backend.py 2744)
   walks `self.threads.values()` on the Machine's thread, reached from `Machine._await_change` (kernel.py 405), while a
   host thread's thread creation inserts (`_add_thread`, 3338). In Rust the table is reachable only through the runtime
   word's guard.
2. **`VM.unimplemented` written by two threads.** kernel.py 431 writes the main VM's from the Machine's thread while
   the main process's host threads may still write it (3805, 3882, 5262). In Rust it is a `OnceLock`: the first stop
   reason wins.
3. **`monitor_commands` incremented in and out of the lock.** steps 5042 and 5044 outside it, host threads at 2021,
   5164 inside. Per-vCPU counters remove the sharing.
4. **`_parked` read without its lock.** `wake_later` (3715) reads it holding neither the runtime lock nor `_changed`;
   a vCPU between `_park`'s `_take_wakes` (3441) and `_parked += 1` (3403) is not yet counted, so a wake can skip the
   notify and that vCPU sleeps until another change or its timeout. In Rust the count and the condition variable are
   one `sched` lock, and a waker always takes it.
5. **A child added during the Machine's wait is not a candidate.** `_await_change` builds its rotation once
   (kernel.py 371-375); a process `add()` inserts meanwhile notifies the wait but is not in the rotation, and the
   deadlock check (407-408) can fire while it waits to start. The Rust supervisor has no rotation: every process runs
   its own vCPUs, and deadlock is decided under `kernel` from every process's last reported state.
6. **A whole child VM built under `Machine._lock`.** `spawn` runs inside `request` (kernel.py 191 → 268), so every
   other process's kernel request waits for a child's construction. The supervisor starts the child process outside
   `kernel` and holds the lock only to insert its row.

## Feasibility against the plan's numbers (KIT_DIARY #64)

The budget is the time between exits on a busy vCPU. The poll loop of 2026-10-22 made about 90,000 exits a second on
one vCPU, 11 µs apart, under a Python host whose own serve took most of that; a Rust serve leaves more of each 11 µs to
the guest, so the rate can only rise, and the budget is stated per exit.

| Cost per exit | Measured | Against 11 µs |
|---|---|---|
| an uncontended lock and unlock (`std::sync::Mutex`) | 4.9 ns over 20,000,000 | 0.04 % |
| a request to the supervisor and its answer, two processes, shared page, spin 50 µs then `os_sync_wait_on_address` | p50 583, 667 and 708 ns; p99 0.75 to 1.67 µs; p99.9 under 5.5 µs; max 1.0 to 2.4 ms (3 runs of 200,000) | p50 about 6 %, p99 under 16 % |
| a FAIRSHARE lock's wait beside a thread holding it 10 µs of 11 | p50 5.1 and 6.4 µs | the worst case the design forbids by its hold rule |
| the exit itself, out of the guest and back | 0.72 µs (`scripts/measure_exit.py`, 2026-10-14, quoted in vm_monitor.c) | 6.5 % |

A named-event poll, the case that starved the Python host, becomes one exit and one supervisor request: about 1.4 µs
at the median against 11 µs, a margin of about 8 times, with no lock held across the request but the guest's own
runtime word. The supervisor answers requests from every vCPU of every process; at 1 µs each it serves about a
million a second before a queue forms, against the 90,000 seen. The 1 to 2.4 ms maxima of the round trip lie above
its p99.9 (under 5.5 µs); while requests come less than 50 µs apart, each side finds the other's word in its spin and
neither waits in the kernel.

## Sources

- Apple, Hypervisor.framework headers, macOS 26 SDK (Xcode 26.6): hv_vm.h (`hv_vm_create`, `hv_vm_destroy`), hv_vcpu.h.
- Apple, pthread_mutexattr(3), libpthread (`man/pthread_mutexattr.3`, `PTHREAD_MUTEX_POLICY_FIRSTFIT_NP`,
  `PTHREAD_MUTEX_POLICY_FAIRSHARE_NP`); macOS 26 SDK `pthread/pthread.h` 174-175, `sys/_pthread/_pthread_types.h` 38-39.
- Apple, `os/os_sync_wait_on_address.h`, macOS 26 SDK.
- rust-lang/rust 1.99.1, `library/std/src/sys/sync/mutex/mod.rs`.
- CPython 3.12, `Python/ceval_gil.c` (`take_gil`, the switch interval).
- Measurements: scratch programs of 2026-10-22 and 2026-10-23 (`crosswake_phases.py`, `crosswake_cached.py`,
  `crosswake_tail.py`, `lockbench`, `rtbench`), results quoted above.

## Every attribute

`Python: kind, guard` is today's: `once` written before the first run, `vcpu` one vCPU's, `shared` touched by several
threads, and the lock that guards it now. `Rust: kind` is this design's, `gone` for the Python plumbing that goes.

| Class | Attribute | Python: kind, guard | Rust: kind | Lock | Home | First set |
|---|---|---|---|---|---|---|
| `Channel` | `_cmd_word` | vcpu, none | gone |  | removed | src/w2m/vm_backend.py:1904 |
| `Channel` | `_exit_word` | vcpu, none | gone |  | removed | src/w2m/vm_backend.py:1905 |
| `Channel` | `_kick_lock` | once, none | shared | vcpu_handle | cpu::VcpuHandle | src/w2m/vm_backend.py:1918 |
| `Channel` | `_kick_word` | shared, _kick_lock | shared | vcpu_handle | cpu::VcpuHandle | src/w2m/vm_backend.py:1906 |
| `Channel` | `_left` | shared, _kick_lock | shared | vcpu_handle | cpu::VcpuHandle | src/w2m/vm_backend.py:1919 |
| `Channel` | `_ref` | once, none | gone |  | removed | src/w2m/vm_backend.py:1902 |
| `Channel` | `barrier` | once, none | gone |  | removed | src/w2m/vm_backend.py:1915 |
| `Channel` | `base` | once, none | gone |  | removed | src/w2m/vm_backend.py:1901 |
| `Channel` | `cmd_addr` | once, none | gone |  | removed | src/w2m/vm_backend.py:1909 |
| `Channel` | `exit_addr` | once, none | gone |  | removed | src/w2m/vm_backend.py:1909 |
| `Channel` | `kick_addr` | once, none | gone |  | removed | src/w2m/vm_backend.py:1907 |
| `Channel` | `mm` | once, none | gone |  | removed | src/w2m/vm_backend.py:1900 |
| `Channel` | `seq` | vcpu, none | gone |  | removed | src/w2m/vm_backend.py:1916 |
| `Channel` | `wait` | once, none | gone |  | removed | src/w2m/vm_backend.py:1911 |
| `Channel` | `wake` | once, none | gone |  | removed | src/w2m/vm_backend.py:1913 |
| `Event` | `handles` | shared, Machine._lock | shared | kernel | supervisor::Kernel | src/w2m/kernel.py:79 |
| `Event` | `manual` | once, Machine._lock | once |  | supervisor::Kernel | src/w2m/kernel.py:77 |
| `Event` | `name` | once, Machine._lock | once |  | supervisor::Kernel | src/w2m/kernel.py:76 |
| `Event` | `signaled` | shared, Machine._lock | shared | kernel | supervisor::Kernel | src/w2m/kernel.py:78 |
| `FileBridge` | `_drive` | once, none | gone |  | removed | src/w2m/file_bridge.py:114 |
| `FileBridge` | `checked` | shared, runtime lock | shared | files | nt::Files | src/w2m/file_bridge.py:127 |
| `FileBridge` | `drive` | once, none | once |  | nt::Files | src/w2m/file_bridge.py:115 |
| `FileBridge` | `fds` | shared, runtime lock | shared | files | nt::Files | src/w2m/file_bridge.py:124 |
| `FileBridge` | `next_fd` | shared, runtime lock | shared | files | nt::Files | src/w2m/file_bridge.py:128 |
| `FileBridge` | `read_only` | once, none | once |  | nt::Files | src/w2m/file_bridge.py:112 |
| `FileBridge` | `read_only_fds` | shared, runtime lock | shared | files | nt::Files | src/w2m/file_bridge.py:125 |
| `FileBridge` | `recorded` | once, none | once |  | nt::Files | src/w2m/file_bridge.py:110 |
| `FileBridge` | `recorded_fds` | shared, runtime lock | shared | files | nt::Files | src/w2m/file_bridge.py:126 |
| `FileBridge` | `root` | once, none | once |  | nt::Files | src/w2m/file_bridge.py:111 |
| `GuestThread` | `at` | once, none | gone |  | removed | src/w2m/scheduler.py:63 |
| `GuestThread` | `created_suspended` | once, none | once |  | nt::Thread | src/w2m/scheduler.py:68 |
| `GuestThread` | `exit_code` | shared, runtime lock | shared | runtime (guest word) | nt::Threads | src/w2m/scheduler.py:65 |
| `GuestThread` | `index` | once, none | once |  | nt::Thread | src/w2m/scheduler.py:62 |
| `GuestThread` | `mm` | once, none | gone |  | removed | src/w2m/scheduler.py:62 |
| `GuestThread` | `parent` | once, none | once |  | nt::Thread | src/w2m/scheduler.py:66 |
| `GuestThread` | `stack` | shared, runtime lock | shared | runtime (guest word) | nt::Threads | src/w2m/scheduler.py:64 |
| `GuestThread` | `start` | once, none | once |  | nt::Thread | src/w2m/scheduler.py:67 |
| `GuestThread` | `table` | once, none | gone |  | removed | src/w2m/scheduler.py:62 |
| `GuestThread` | `teb` | once, none | once |  | nt::Thread | src/w2m/scheduler.py:64 |
| `GuestThread` | `tid` | once, none | once |  | nt::Thread | src/w2m/scheduler.py:64 |
| `Machine` | `_changed` | once, none | gone |  | removed | src/w2m/kernel.py:126 |
| `Machine` | `_changes` | shared, _changed | gone |  | removed | src/w2m/kernel.py:127 |
| `Machine` | `_lock` | once, none | once |  | supervisor::Kernel | src/w2m/kernel.py:123 |
| `Machine` | `_running` | shared, none | gone |  | removed | src/w2m/kernel.py:124 |
| `Machine` | `_thread` | once, none | gone |  | removed | src/w2m/kernel.py:125 |
| `Machine` | `main` | once, none | gone |  | removed | src/w2m/kernel.py:116 |
| `Machine` | `names` | shared, Machine._lock | shared | kernel | supervisor::Kernel | src/w2m/kernel.py:112 |
| `Machine` | `next_id` | shared, Machine._lock | shared | kernel | supervisor::Kernel | src/w2m/kernel.py:113 |
| `Machine` | `next_pid` | once, none | once |  | supervisor::Kernel | src/w2m/kernel.py:114 |
| `Machine` | `next_tid` | shared, Machine._lock | shared | kernel | supervisor::Kernel | src/w2m/kernel.py:115 |
| `Machine` | `objects` | shared, Machine._lock | shared | kernel | supervisor::Kernel | src/w2m/kernel.py:110 |
| `Machine` | `pipe_ends` | shared, Machine._lock | shared | kernel | supervisor::Kernel | src/w2m/kernel.py:111 |
| `Machine` | `processes` | shared, Machine._lock (writes only) | shared | kernel | supervisor::Kernel | src/w2m/kernel.py:109 |
| `Machine` | `run_args` | once, none | gone |  | removed | src/w2m/kernel.py:107 |
| `Machine` | `spawn` | once, none | gone |  | removed | src/w2m/kernel.py:106 |
| `Machine` | `stop_requested` | shared, none | shared | atomic (set once) | supervisor::Kernel | src/w2m/kernel.py:119 |
| `Machine` | `stop_when` | once, none | once |  | supervisor::Kernel | src/w2m/kernel.py:120 |
| `Machine` | `vms` | shared, Machine._lock (writes only) | shared | kernel | supervisor::Kernel | src/w2m/kernel.py:108 |
| `Pipe` | `data` | shared, Machine._lock | shared | kernel | supervisor::Kernel | src/w2m/kernel.py:84 |
| `Pipe` | `readers` | shared, Machine._lock | shared | kernel | supervisor::Kernel | src/w2m/kernel.py:85 |
| `Pipe` | `writers` | shared, Machine._lock | shared | kernel | supervisor::Kernel | src/w2m/kernel.py:86 |
| `Process` | `exit_code` | shared, none (writer) / Machine._lock (reader) | shared | kernel | supervisor::Kernel | src/w2m/kernel.py:94 |
| `Process` | `handles` | shared, Machine._lock | shared | kernel | supervisor::Kernel | src/w2m/kernel.py:95 |
| `Process` | `held` | shared, Machine._lock | shared | kernel | supervisor::Kernel | src/w2m/kernel.py:97 |
| `Process` | `pid` | once, none | once |  | supervisor::Kernel | src/w2m/kernel.py:91 |
| `Process` | `result` | vcpu, none | gone |  | removed | src/w2m/kernel.py:96 |
| `Process` | `steps` | vcpu, none | gone |  | removed | src/w2m/kernel.py:93 |
| `Process` | `vm` | once, none | gone |  | removed | src/w2m/kernel.py:92 |
| `Reservation` | `allocation_protect` | once, runtime lock | once |  | memory::VirtualMemory | src/w2m/virtual_memory.py:51 |
| `Reservation` | `base` | once, runtime lock | once |  | memory::VirtualMemory | src/w2m/virtual_memory.py:49 |
| `Reservation` | `protect` | shared, runtime lock | shared | vm_map | memory::VirtualMemory | src/w2m/virtual_memory.py:53 |
| `Reservation` | `size` | once, runtime lock | once |  | memory::VirtualMemory | src/w2m/virtual_memory.py:50 |
| `Reservation` | `state` | shared, runtime lock | shared | vm_map | memory::VirtualMemory | src/w2m/virtual_memory.py:52 |
| `Reservation` | `type` | once, runtime lock | once |  | memory::VirtualMemory | src/w2m/virtual_memory.py:55 |
| `VM` | `_barrier` | once, none (immutable) | gone |  | removed | src/w2m/vm_backend.py:2406 |
| `VM` | `_changed` | once, none (it is the Condition) | once |  | nt::Scheduler | src/w2m/vm_backend.py:2449 |
| `VM` | `_changes` | shared, _changed | shared | sched | nt::Scheduler | src/w2m/vm_backend.py:2453 |
| `VM` | `_cmd` | vcpu, none (vCPU 0 / Machine thread only) | gone |  | removed | src/w2m/vm_backend.py:4970 |
| `VM` | `_environment_bytes` | once, none | once |  | host::Process | src/w2m/vm_backend.py:2826 |
| `VM` | `_final_census` | once, none | once |  | diagnostics::Final | src/w2m/vm_backend.py:2380 |
| `VM` | `_final_code_census` | once, none | once |  | diagnostics::Final | src/w2m/vm_backend.py:5356 |
| `VM` | `_first_tid` | once, none | once |  | host::Process | src/w2m/vm_backend.py:2497 |
| `VM` | `_flush_point` | once, none | once |  | cpu::Stubs | src/w2m/vm_backend.py:2511 |
| `VM` | `_idle` | vcpu, runtime lock | vcpu |  | cpu::VcpuThread | src/w2m/vm_backend.py:2495 |
| `VM` | `_max_steps` | once, none | once |  | host::Process | src/w2m/vm_backend.py:2496 |
| `VM` | `_next_table` | shared, runtime lock | shared | code | cpu::CodeCache | src/w2m/vm_backend.py:2425 |
| `VM` | `_page_set_removed` | shared, runtime lock | shared | code | cpu::CodeCache | src/w2m/vm_backend.py:2429 |
| `VM` | `_parked` | shared, _changed (writes); reads partly none | shared | sched | nt::Scheduler | src/w2m/vm_backend.py:2450 |
| `VM` | `_pending_init` | shared, runtime lock | shared | modules | nt::Modules | src/w2m/vm_backend.py:2381 |
| `VM` | `_private_regions` | once, none | once |  | memory::GuestMemory | src/w2m/vm_backend.py:2458 |
| `VM` | `_private_tables` | once, none | once |  | memory::GuestMemory | src/w2m/vm_backend.py:2457 |
| `VM` | `_random` | once, runtime lock (its state) | once |  | host::Process | src/w2m/vm_backend.py:2357 |
| `VM` | `_scan_log` | shared, runtime lock | shared | code | cpu::CodeCache | src/w2m/vm_backend.py:2435 |
| `VM` | `_stack` | once, none | once |  | host::Process | src/w2m/vm_backend.py:2849 |
| `VM` | `_stopping` | shared, _changed (writes); reads none | shared | atomic (set once) | host::Process | src/w2m/vm_backend.py:2448 |
| `VM` | `_stub_page` | shared, runtime lock | shared | code | cpu::CodeCache | src/w2m/vm_backend.py:2491 |
| `VM` | `_takers` | shared, runtime lock | shared | runtime (guest word) | nt::Threads | src/w2m/vm_backend.py:2452 |
| `VM` | `_trace_bound` | shared, runtime lock | shared | trace | diagnostics::Trace | src/w2m/vm_backend.py:2477 |
| `VM` | `_trace_dynamic` | shared, runtime lock | shared | trace | diagnostics::Trace | src/w2m/vm_backend.py:2479 |
| `VM` | `_trace_from` | once, none | once |  | cpu::Stubs | src/w2m/vm_backend.py:2483 |
| `VM` | `_trace_quiet` | shared, runtime lock | shared | trace | diagnostics::Trace | src/w2m/vm_backend.py:2478 |
| `VM` | `_view` | once, none | gone |  | removed | src/w2m/vm_backend.py:2397 |
| `VM` | `_wake_pending` | shared, none (by design) | shared | sched | nt::Scheduler | src/w2m/vm_backend.py:2455 |
| `VM` | `_words` | once, none | gone |  | removed | src/w2m/vm_backend.py:2400 |
| `VM` | `addrs` | shared, runtime lock | shared | code | cpu::CodeCache | src/w2m/vm_backend.py:2463 |
| `VM` | `block_census` | shared, runtime lock | shared | code | cpu::CodeCache | src/w2m/vm_backend.py:2490 |
| `VM` | `blocks` | shared, runtime lock | shared | code | cpu::CodeCache | src/w2m/vm_backend.py:2462 |
| `VM` | `blocks_dropped` | shared, runtime lock | vcpu |  | diagnostics::VcpuCounters | src/w2m/vm_backend.py:2459 |
| `VM` | `bridges` | shared, runtime lock | shared | code | cpu::CodeCache | src/w2m/vm_backend.py:2475 |
| `VM` | `call_trace` | once, none | once |  | cpu::Stubs | src/w2m/vm_backend.py:2346 |
| `VM` | `clock` | once, none | once |  | host::Process | src/w2m/vm_backend.py:2351 |
| `VM` | `clock_from_host` | once, none | once |  | host::Process | src/w2m/vm_backend.py:2352 |
| `VM` | `code_cursor` | shared, runtime lock (host and guest) | shared | runtime (guest word) | cpu::CodeCache | src/w2m/vm_backend.py:2461 |
| `VM` | `code_pages` | shared, runtime lock | shared | code | cpu::CodeCache | src/w2m/vm_backend.py:2428 |
| `VM` | `code_writes` | shared, runtime lock | vcpu |  | diagnostics::VcpuCounters | src/w2m/vm_backend.py:2441 |
| `VM` | `command_line` | once, none | once |  | host::Process | src/w2m/vm_backend.py:2360 |
| `VM` | `directory` | once, none | once |  | host::Process | src/w2m/vm_backend.py:2364 |
| `VM` | `dispatch_va` | once, none | once |  | cpu::Stubs | src/w2m/vm_backend.py:4259 |
| `VM` | `entry_va` | once, none | once |  | cpu::Stubs | src/w2m/vm_backend.py:2507 |
| `VM` | `environment` | once, none | once |  | host::Process | src/w2m/vm_backend.py:2361 |
| `VM` | `exception_call_va` | once, none | once |  | cpu::Stubs | src/w2m/vm_backend.py:4382 |
| `VM` | `exit_code` | shared, runtime lock | shared | atomic (set once) | host::Process | src/w2m/vm_backend.py:2368 |
| `VM` | `exit_ns` | vcpu, runtime lock | vcpu |  | diagnostics::VcpuCounters | src/w2m/vm_backend.py:2378 |
| `VM` | `exit_sites` | shared, runtime lock | vcpu |  | diagnostics::VcpuCounters | src/w2m/vm_backend.py:2374 |
| `VM` | `exits` | shared, runtime lock | vcpu |  | diagnostics::VcpuCounters | src/w2m/vm_backend.py:2372 |
| `VM` | `fault_give_backs` | shared, runtime lock | vcpu |  | diagnostics::VcpuCounters | src/w2m/vm_backend.py:2444 |
| `VM` | `files` | once, runtime lock (the FileBridge's own state) | once |  | host::Process | src/w2m/vm_backend.py:2349 |
| `VM` | `final` | vcpu, none (after host threads joined) | vcpu |  | cpu::VcpuThread | src/w2m/vm_backend.py:5125 |
| `VM` | `flags_cache` | shared, runtime lock | shared | code | cpu::CodeCache | src/w2m/vm_backend.py:2439 |
| `VM` | `foreground_held` | once, none | once |  | host::Process | src/w2m/vm_backend.py:2359 |
| `VM` | `gui` | once, none | once |  | host::Process | src/w2m/vm_backend.py:2358 |
| `VM` | `handovers` | shared, runtime lock | vcpu |  | diagnostics::VcpuCounters | src/w2m/vm_backend.py:2373 |
| `VM` | `host_requests` | shared, runtime lock | vcpu |  | diagnostics::VcpuCounters | src/w2m/vm_backend.py:2371 |
| `VM` | `image_name` | once, none | once |  | host::Process | src/w2m/vm_backend.py:2366 |
| `VM` | `inherited` | once, none | once |  | host::Process | src/w2m/vm_backend.py:2363 |
| `VM` | `instruction_clock` | once, none | once |  | host::Process | src/w2m/vm_backend.py:2356 |
| `VM` | `kicks` | shared, runtime lock | vcpu |  | diagnostics::VcpuCounters | src/w2m/vm_backend.py:2442 |
| `VM` | `ld` | shared, none (binding); runtime lock (contents) | shared | modules | nt::Modules | src/w2m/vm_backend.py:2345 |
| `VM` | `lift_cache` | shared, runtime lock | shared | code | cpu::CodeCache | src/w2m/vm_backend.py:2438 |
| `VM` | `lift_pages` | shared, runtime lock | shared | code | cpu::CodeCache | src/w2m/vm_backend.py:2440 |
| `VM` | `lock` | once, (is the guard) | once |  | nt::RuntimeLock | src/w2m/vm_backend.py:2404 |
| `VM` | `lockstep` | once, none | once |  | host::Process | src/w2m/vm_backend.py:2350 |
| `VM` | `machine` | once, Machine._lock (the write) | gone |  | removed | src/w2m/vm_backend.py:2492 |
| `VM` | `mem` | shared, none (binding); runtime lock (host writes) | once |  | memory::GuestMemory | src/w2m/vm_backend.py:2407 |
| `VM` | `miss_va` | once, none | once |  | cpu::Stubs | src/w2m/vm_backend.py:4255 |
| `VM` | `mm` | shared, none (binding); runtime lock (host stores) | once |  | memory::GuestMemory | src/w2m/vm_backend.py:2395 |
| `VM` | `modules` | shared, none (binding); runtime lock (contents) | shared | modules | nt::Modules | src/w2m/vm_backend.py:2476 |
| `VM` | `monitor_commands` | shared, runtime lock + none (steps) | vcpu |  | diagnostics::VcpuCounters | src/w2m/vm_backend.py:2379 |
| `VM` | `out` | shared, runtime lock | shared | output | host::Output | src/w2m/vm_backend.py:2369 |
| `VM` | `path` | once, none | gone |  | removed | src/w2m/vm_backend.py:2393 |
| `VM` | `pc_map` | shared, runtime lock | shared | code | cpu::CodeCache | src/w2m/vm_backend.py:2468 |
| `VM` | `process_group` | once, none | once |  | host::Process | src/w2m/vm_backend.py:2365 |
| `VM` | `process_id` | once, none | once |  | host::Process | src/w2m/vm_backend.py:2362 |
| `VM` | `remote_suspends` | shared, runtime lock | vcpu |  | diagnostics::VcpuCounters | src/w2m/vm_backend.py:2443 |
| `VM` | `resume_va` | once, none | once |  | cpu::Stubs | src/w2m/vm_backend.py:4262 |
| `VM` | `run_ns` | shared, runtime lock | vcpu |  | diagnostics::VcpuCounters | src/w2m/vm_backend.py:2377 |
| `VM` | `runtime` | once, none | once |  | cpu::Stubs | src/w2m/vm_backend.py:2414 |
| `VM` | `size` | once, none | once |  | memory::GuestMemory | src/w2m/vm_backend.py:2392 |
| `VM` | `slice_va` | once, none | once |  | cpu::Stubs | src/w2m/vm_backend.py:2506 |
| `VM` | `slow_sse` | shared, runtime lock | vcpu |  | diagnostics::VcpuCounters | src/w2m/vm_backend.py:2470 |
| `VM` | `slow_sse_classes` | shared, runtime lock | vcpu |  | diagnostics::VcpuCounters | src/w2m/vm_backend.py:2471 |
| `VM` | `switch_slice_va` | once, none | once |  | cpu::Stubs | src/w2m/vm_backend.py:2474 |
| `VM` | `switch_va` | once, none | once |  | cpu::Stubs | src/w2m/vm_backend.py:2473 |
| `VM` | `tail_hosts` | shared, runtime lock | shared | code | cpu::CodeCache | src/w2m/vm_backend.py:2466 |
| `VM` | `tail_left` | shared, runtime lock | shared | code | cpu::CodeCache | src/w2m/vm_backend.py:2467 |
| `VM` | `tcache` | shared, runtime lock | shared | code | cpu::CodeCache | src/w2m/vm_backend.py:2431 |
| `VM` | `tcache_file` | shared, runtime lock | shared | code | cpu::CodeCache | src/w2m/vm_backend.py:2432 |
| `VM` | `tcache_hits` | shared, runtime lock | vcpu |  | diagnostics::VcpuCounters | src/w2m/vm_backend.py:2434 |
| `VM` | `tcache_loaded` | shared, runtime lock | shared | code | cpu::CodeCache | src/w2m/vm_backend.py:2433 |
| `VM` | `tcache_new` | shared, runtime lock | vcpu |  | diagnostics::VcpuCounters | src/w2m/vm_backend.py:2434 |
| `VM` | `th` | once, none | gone |  | removed | src/w2m/vm_backend.py:2367 |
| `VM` | `thread_vcpus` | once, none | once |  | host::Process | src/w2m/vm_backend.py:2447 |
| `VM` | `threads` | shared, runtime lock (writers); none (Machine-thread readers) | shared | runtime (guest word) | nt::Threads | src/w2m/vm_backend.py:2489 |
| `VM` | `translations` | shared, runtime lock | shared | code | cpu::CodeCache | src/w2m/vm_backend.py:2469 |
| `VM` | `ttbr` | once, none | once |  | memory::GuestMemory | src/w2m/vm_backend.py:2424 |
| `VM` | `unimplemented` | shared, runtime lock + none (kernel) | shared | atomic (set once) | host::Process | src/w2m/vm_backend.py:2370 |
| `VM` | `vcpu_count` | once, none | gone |  | removed | src/w2m/vm_backend.py:2445 |
| `VM` | `vcpu_readback` | once, none | gone |  | removed | src/w2m/vm_backend.py:2456 |
| `VM` | `vcpus` | once, none (list); Channel._kick_lock for cross-thread kicks | once |  | memory::GuestMemory | src/w2m/vm_backend.py:2403 |
| `VM` | `virtual` | shared, none (binding); runtime lock (contents) | shared | vm_map | memory::VirtualMemory | src/w2m/vm_backend.py:2498 |
| `VM` | `vm_judged` | shared, runtime lock | vcpu |  | diagnostics::VcpuCounters | src/w2m/vm_backend.py:2430 |
| `VM` | `wakes` | shared, runtime lock | vcpu |  | diagnostics::VcpuCounters | src/w2m/vm_backend.py:2451 |
| `Vcpu` | `_census` | shared, runtime lock (partly) | gone |  | removed | src/w2m/vm_backend.py:1991 |
| `Vcpu` | `_pipe` | vcpu, none | gone |  | removed | src/w2m/vm_backend.py:1992 |
| `Vcpu` | `channel` | shared, none | vcpu |  | cpu::VcpuThread | src/w2m/vm_backend.py:1990 |
| `Vcpu` | `index` | once, none | vcpu |  | cpu::VcpuThread | src/w2m/vm_backend.py:1990 |
| `Vcpu` | `md` | once, none | vcpu |  | cpu::VcpuThread | src/w2m/vm_backend.py:1994 |
| `Vcpu` | `mm` | once, none | gone |  | removed | src/w2m/vm_backend.py:1990 |
| `Vcpu` | `st` | once, none | vcpu |  | cpu::VcpuThread | src/w2m/vm_backend.py:1994 |
| `Vcpu` | `vswap` | vcpu, runtime lock (in serve) / owner thread | vcpu |  | cpu::VcpuThread | src/w2m/vm_backend.py:1998 |
| `VirtualMemory` | `arena` | once, none | once |  | memory::VirtualMemory | src/w2m/virtual_memory.py:77 |
| `VirtualMemory` | `arena_size` | once, none | once |  | memory::VirtualMemory | src/w2m/virtual_memory.py:78 |
| `VirtualMemory` | `heap` | once, none | once |  | memory::VirtualMemory | src/w2m/virtual_memory.py:90 |
| `VirtualMemory` | `image_protect` | shared, runtime lock | shared | vm_map | memory::VirtualMemory | src/w2m/virtual_memory.py:84 |
| `VirtualMemory` | `images` | shared, runtime lock | shared | vm_map | memory::VirtualMemory | src/w2m/virtual_memory.py:80 |
| `VirtualMemory` | `mapped` | once, none | once |  | memory::VirtualMemory | src/w2m/virtual_memory.py:88 |
| `VirtualMemory` | `pristine` | shared, runtime lock | shared | vm_map | memory::VirtualMemory | src/w2m/virtual_memory.py:82 |
| `VirtualMemory` | `read` | once, none | gone |  | removed | src/w2m/virtual_memory.py:81 |
| `VirtualMemory` | `reservations` | shared, runtime lock | shared | vm_map | memory::VirtualMemory | src/w2m/virtual_memory.py:83 |
| `VirtualMemory` | `stack` | once, none | once |  | memory::VirtualMemory | src/w2m/virtual_memory.py:85 |
| `VirtualMemory` | `stacks` | shared, runtime lock | shared | vm_map | memory::VirtualMemory | src/w2m/virtual_memory.py:86 |
| `VirtualMemory` | `zero` | once, none | gone |  | removed | src/w2m/virtual_memory.py:79 |
