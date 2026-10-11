# Sprint 051 — threads

```yaml
---
id: 051
status: closed
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

- **The frontier passes CreateThread** (`store/runs/frontier-643a6283510c-1791708222148046000`): mono's first thread (id 8) is created, scheduled and running when the run stops at `ole32.dll!CoInitializeEx` after 20,590,160 instructions (46 s on the Mac); in the capture mono's new thread makes the same call (the com family).
- **The threads family equals Windows** (contract windows.thread.threads 0.1.0, 31 rows, runs 38124576301 and 38124578332, equal on Server 2025 and 2022) with five plants of named reach: creation, the attach and detach calls, the last error 187 a thread starts with, per-thread TLS and static TLS, exit codes, waits, suspension counts, GetThreadContext of a suspended spinning thread, CREATE_SUSPENDED, ExitThread, priorities, descriptions, SleepEx, AppPolicyGetThreadInitializationType.
- **The scheduler, one vCPU** (Decision 2026-10-11): each thread its own TEB (the runtime reads it from the state block's GS base), stack, static TLS and saved state; a wait that cannot complete retries its call through the bridge (state.schedule, hvc #12); time slices of 200,000 instructions once a second thread exists; waits, mutexes, critical sections, SRW locks and condition variables block instead of stopping the run.
- **Every earlier family still conforms** with threads in the runtime (15 re-recorded, all accepted).
- **Defects found and fixed:** the loader never called a DLL's TLS callbacks at process start, only its DllMain, since Sprint 046 (the static C runtime's TLS callback of UnityPlayer and mono among them); the probe's event log showed it (KIT_DIARY #42). The reference walker missed functions reached only through a pointer (a thread's start routine) and now roots address-taken code.
- **Not reached:** mono's collector suspending threads and reading their context in the game; the frontier stops before its first collection. The rows judge both calls; the game's use waits for the frontier.

