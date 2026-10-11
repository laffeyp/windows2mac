# Sprint 051 — threads

```yaml
---
id: 051
status: open
phase: 5
pass_kind: functional
---
```

## scope

Sprint 050 left the frontier at 20,584,207 instructions, at `KERNEL32.dll!CreateThread` (mono's first thread, trace seq 1250). The work list's threads family holds what the game calls (`specs/work_list.json`, captures 38113342549 and 38113343972): CreateThread 28 calls, SetThreadPriority 37, SetThreadDescription 24, AppPolicyGetThreadInitializationType 1, GetExitCodeThread, SuspendThread, GetThreadContext and ResumeThread 111 each (mono's collector stopping the world), SleepEx 41,341, SwitchToThread 475,374; 35 threads at the peak. Decision 2026-10-11: the scheduler runs on one vCPU first.

1. **The rows.** A threads probe on Server 2025 and 2022 in the game's forms: CreateThread's handle and id, a thread's exit code (STILL_ACTIVE, then its return), a wait on a thread, SuspendThread's and ResumeThread's previous counts, GetThreadContext of a suspended thread (its registers where the probe parked it), SetThreadPriority and GetThreadPriority, SetThreadDescription, SleepEx(0) and SwitchToThread's answers, the last error and TLS kept per thread, DLL_THREAD_ATTACH and TLS callbacks for each new thread.
2. **Per-thread state.** Each guest thread has its own TEB (its TLS slots, last error, stack bounds, static TLS pointer), its stack (the thread-stack contract's layout, the PE header's SizeOfStackReserve), and a state block: x86 registers and flags, XMM and YMM upper halves, MXCSR, x87 control word.
3. **The scheduler (one vCPU).** The host switches threads at instruction boundaries: when a thread blocks in a wait, sleeps, yields, exits, or uses up its time slice (the step budget). Waits block instead of stopping the run. A suspended thread is not scheduled; GetThreadContext reads its state block.
4. **The services:** the family's ten functions; ExitThread and a start routine's return; waits on thread handles; DllMain(DLL_THREAD_ATTACH, DETACH) and TLS callbacks run on the new thread.

## prerequisites

- 050 (closed: CI run on 3b709e65; main 2e61bfab)

## context_files

- `src/w2m/vm_backend.py` (VM.run, the TEB and state block), `tools/guest_runtime/runtime.h`, `tools/guest_runtime/sync_objects.c`, `tools/guest_runtime/thread_queries.c`, `tools/guest_runtime/thread_storage.c`, `src/w2m/virtual_memory.py` (the stack), `src/w2m/loader.py` (TLS, DllMain), `specs/guest_state.json`, `contracts/windows.memory.thread_stack`, `probes/thread_handles`, `probes/context` (Sprint 034); Microsoft Learn: CreateThread, SuspendThread, GetThreadContext, SwitchToThread, SleepEx, DllMain

## signal contract

### Emits

- Per family: the item run. Per frontier run: EXECUTION_STARTED, then SERVICE_UNIMPLEMENTED or EXECUTION_COMPLETED.

### Invariants

- Vocabulary 0.7; no new tags.

## artifact contract

- created: `probes/threads/`, the threads contract, the guest runtime's thread services, the host scheduler
- modified: `src/w2m/vm_backend.py`, `tools/guest_runtime/sync_objects.c`, `tools/guest_runtime/runtime.h`, `src/w2m/loader.py`
- gates 0 and CI `success` on the closing commit

## observation contract

- **The threads family equals Windows** on its probe's rows on Server 2025 and 2022, with plants of named reach.
- **Every earlier family still conforms** with threads in the runtime.
- **The frontier passes CreateThread:** mono's threads start, and mono's collector suspends, reads and resumes them; the next stop is recorded.

## done criteria

The threads family accepted; the frontier runs past CreateThread with more than one guest thread scheduled.

## observed
