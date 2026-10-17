# Sprint 096 — a PAUSE yields the slice when another thread is ready

```yaml
---
id: 096
status: active
phase: 5
pass_kind: functional
---
```

## scope

Roadmap step 2, second link: spinning. Sprint 094 measured the main thread retiring 63.8% of the batch run's
instructions with 233.6 million PAUSEs. On one vCPU a spinning thread waits for a thread that cannot run until the
spinner's 20,000-instruction slice ends. This sprint makes a PAUSE end the running thread's slice when another thread
is ready, so the thread it waits for runs; with no other thread ready, a PAUSE stays a counted no-op.

**Known first.**

- Hypervisors do this for spinning guests: PAUSE-loop exiting gives VMX "PLE_Gap: upper bound on the amount of time
  between two successive executions of PAUSE in a loop" and "PLE_Window: upper bound on the amount of time a guest is
  allowed to execute in a PAUSE loop", and the processor "triggers a VM exit if total time exceeds the PLE_Window"
  (Zhai Edwin, "KVM: VMX: Add support for Pause-Loop Exiting", 2010-03-05); KVM's directed yield has "a VCPU that spins
  give the remainder of its timeslice to another VCPU in the same guest before yielding the CPU - one that is runnable
  but got preempted, hopefully the lock holder" (Rik van Riel, LKML, 2011-01-14). On one vCPU every other ready thread
  is such a preempted holder, so w2m yields at the first PAUSE (window 0) and only when one is ready.
- The slice (vm_backend.py, REG_BUDGET and `_slice_stub`): each block entry is SUBS x23, x23, #n and B.MI to its stop;
  the count is the limit word minus x23; the stub switches when the count reaches SLICE_UNTIL, which threads.c
  `slice_until` and the host's `_slice_until` set to 0 when another unsuspended thread is ready.
- PAUSE is lowered as a count only (Sprint 068 item J). The compiler inside the VM builds blocks from a stencil table
  derived from compile_insn, so a changed lowering reaches VM-compiled blocks only if they leave PAUSE to the host.

## the change

- compile_insn's `pause`: after the count, when SLICE_UNTIL is 0, the limit word becomes the count (limit − x23) and
  x23 becomes 0, so the next block entry ends the slice with the count exact.
- block_compiler.c: a block the VM compiles ends before a PAUSE, and a block that starts at one goes to the host (a new
  refusal cause, "pause"), so every PAUSE runs compile_insn's lowering.

## prerequisites

- 095 (the wake path; its timing runs repeat with this sprint's on a quiet Mac).

## context_files

- src/w2m/vm_backend.py (compile_insn's `pause`, `_slice_stub`, REG_BUDGET); tools/guest_runtime/block_compiler.c,
  threads.c; sprints/sprint-068 item J, sprint-094.

## signal contract

### Emits

- None new; the frontier's records and the families' item runs as before.

### Invariants

- Vocabulary unchanged. The instruction count stays exact across a yield (the clock reads it).

## artifact contract

- vm_backend.py, block_compiler.c; tests: a yield at a PAUSE ends the slice with the count unchanged, and no yield
  with no other thread ready; `scripts/gates.sh` exit 0; CI `success` on the closing commit.

## observation contract

- **Judges:** all 68 families accept (every one runs through the compiler); the batch run conforms (168 of 168 lines,
  0 value divergences); the class runs of PAUSE accept.
- **Plant:** a yield that zeroes x23 without moving the limit, so the count jumps by the slice's remainder. Reach: the
  instruction clock; the test that holds the count across a yield must fail it.
- **Measure:** PAUSEs on the batch run to its stop fall from 508 to 608 million (Sprint 094, to the stop) by at least
  10 times; seconds to the stop, two runs each with 095's, the load average at each start under 4.

## done criteria

PAUSE yields when another thread is ready, every judge holds, the spin census and the seconds to the stop reported,
CI green.

## observed
