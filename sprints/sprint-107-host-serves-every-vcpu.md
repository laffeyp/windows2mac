# Sprint 107 — the run table and the host for several vCPUs

```yaml
---
id: 107
status: active
phase: 5
pass_kind: architecture
---
```

## scope

The second sprint of "The runtime under several vCPUs" (docs/design-several-vcpus.md). Sprint 106 put one lock
around the runtime; guest threads still run on vCPUs 1.. only under a test harness, and only code that needs no
host. This sprint makes the scheduler and the host serve every vCPU:

- **The run table:** a thread running on a vCPU is marked in its entry (`reserved0`: the vCPU's index + 1, 0 when it
  runs nowhere), so `w2m_switch` never takes a thread another vCPU runs; the current thread, the slice's count and
  PAUSEs move to each vCPU's private monitor page; vCPU 0's are mirrored into the run table header for the host.
- **The host:** each vCPU's channel served on its own host thread, one Python lock around the exit handlers;
  `current_tid` per vCPU.
- **Shared words a vCPU must read live, found in 106:** RECON_COUNT and the reconstruction table (each vCPU's private
  page holds a stale copy of the count, so a fault on vCPU 1 could not find its x86 address), the clock words
  (CLOCK_WARP moves), the process-wide start words: to a process page, each written with one 64-bit store after a
  barrier. The code cache: a block the host writes may be reached through the table by a vCPU that did not miss;
  the instruction-cache maintenance that covers it, looked up in the Arm ARM before any choice.

## prerequisites

- 106.

## context_files

- docs/design-several-vcpus.md; tools/guest_runtime/threads.c, runtime.c (guest_rip_at); src/w2m/vm_backend.py
  (`steps`, `_schedule`, `_switch`, the host requests); src/w2m/scheduler.py.

## signal contract

### Emits

- None new.

### Invariants

- Vocabulary unchanged; at one vCPU every run as before.

## artifact contract

Found on opening (2026-10-17), each item with the code it rests on:

- **Two words move to the shared run table header.** Of the private monitor page's words, RECON_COUNT (0x40;
  runtime.c `guest_rip_at`, the host's `_recon_append`) and CLOCK_WARP (0x98; threads.c `w2m_switch`, process.c's
  counter, the host's `warp_to_deadline`) change during a run; they move to RUN_TABLE + 0xC0 and + 0xC8, in a
  cache line apart from the lock's (0x80). CLOCK_MODE, CLOCK_RATE, CLOCK_EPOCH, START_MODE and FOREGROUND_MODE are
  written by `_init_process` before any vCPU starts, so each private copy holds them; RUN_END stays per vCPU.
- **The host takes the runtime lock.** `_schedule`, `_switch`, `warp_to_deadline`, `_wake_expired` and the thread
  services' host side write the run table, which `w2m_switch` writes under the lock on another vCPU; the slice stub's
  HVC #4 with switching off reaches the host holding nothing. The host takes the same word through a small C
  helper (`__atomic_exchange_n`, acquire; `__atomic_store_n`, release) called by ctypes `CDLL`, which releases the
  GIL around the call (Python 3.12 ctypes documentation), so a host thread spinning on the lock does not stop the
  host thread that serves its holder. An exit made inside the runtime's C (HVC #8, #9) is served under the lock the
  vCPU already holds.
- **The run table per vCPU.** `current`, `current_entry`, `slice_count` and `slice_pauses` move to each vCPU's
  private monitor page (the switch stub reads them x28-relative); an entry's `reserved0` holds the index + 1 of the
  vCPU running it, and `w2m_switch` and the host's `_switch` skip such a thread.
- **The host serves every channel.** One host thread per vCPU runs `steps` for its channel; the threads share the
  VM under one Python lock around each exit's handler; `current_tid` becomes per vCPU.
- `tests/test_vcpus_threads.py` (hvf): the threads family's probe with its threads on two vCPUs; the plant.

## observation contract

- **Judge:** the threads family's probe on two vCPUs accepts against its Windows rows; a guest fault on vCPU 1
  dispatches to its handler.
- **Plant:** the RUNNING mark left out: two vCPUs run one thread, and the family's rows differ.

## done criteria

Every vCPU runs guest threads with services, faults and switches; CI green.

## observed
