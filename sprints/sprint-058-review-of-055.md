# Sprint 058 — a review of Sprint 055

```yaml
---
id: 058
status: active
phase: 5
pass_kind: functional
---
```

## scope

Sprint 055 added a second process: `src/w2m/kernel.py` (the Machine and the objects processes share),
`tools/guest_runtime/processes.c`, the inheritance list in `sync_objects.c` and the inherit page in `vm_backend.py`.
A full read of those files (2026-10-11, on d5ecffe8) found seven defects:

| # | Defect | Where |
|---|---|---|
| 1 | a process that ends keeps its handles on the host's objects: a pipe whose last writer exits never breaks, and its reader waits forever (Windows closes an ending process's handles) | `kernel.py` |
| 2 | a child gets the environment the parent started with, not the parent's current one (CreateProcessW: "If this parameter is NULL, the new process uses the environment of the calling process") | `kernel.py` spawner |
| 3 | a child starts in the program's directory, not the parent's current one ("If this parameter is NULL, the new process will have the same current drive and directory as the calling process") | `kernel.py`, `paths.c` |
| 4 | more than 64 inheritable handles: the rest are left out without a stop | `sync_objects.c` `w2m_object_inheritable` |
| 5 | object ids and process ids are looked up in one key space: a handle on object 4100 closes or references process 4100 | `kernel.py` `_ref`, `_close` |
| 6 | the pipe's buffer grows without bound: a writer never waits (CreatePipe's size is ignored) | `kernel.py`, `processes.c` |
| 7 | the runtime's symbol addresses are one table for every VM in the host: a VM's CPUID calls the last-built VM's `w2m_cpuid` | `vm_backend.py` `_RUNTIME_SYMBOLS` |

1. **A failing check first, for each:** Machine tests with stand-in VMs (1, 5, 6), a native harness under
   AddressSanitizer (4), two VMs with different runtime images (7), and probe rows (2, 3, 6).
2. **Windows rows** (the processes probe): a second child, started after the parent set a variable and changed its
   directory, prints both down a pipe and exits holding the write end; a pipe of UnityPlayer's size (0, game run
   38176899195 seq 9) filled a byte at a time by a thread, and one write larger than it holds.
3. **The fixes**, judged by the checks and the rows.
4. **One gap noted for 059:** CreateProcessW with STARTF_USESTDHANDLES and bInheritHandles FALSE (Microsoft Learn:
   the handles "must be inheritable and the function's bInheritHandles parameter must be set to TRUE") stops the run.

## prerequisites

- 057 (closed: CI run 38188148154 on f7b474c7; main d5ecffe8)

## context_files

- `src/w2m/kernel.py`, `tools/guest_runtime/processes.c`, `sync_objects.c`, `paths.c`, `environment.c`,
  `src/w2m/vm_backend.py`; `probes/processes/processes.c`; Microsoft Learn: CreateProcessW, CreatePipe

## signal contract

### Emits

- The processes family's item run and plants; every family whose candidate changes, its item run.

### Invariants

- Vocabulary 0.7; no new tags.

## artifact contract

- created: `tests/test_kernel.py` (the Machine checks)
- modified: the files in the table; `probes/processes/processes.c` and the processes contract (0.2.0);
  `tests/test_runtime_native.py`; `scripts/family_run.py`
- gates 0 and CI `success` on the closing commit

## observation contract

- **Each check fails on d5ecffe8's code and passes after its fix.**
- **processes 0.2.0 equals Windows** on Server 2025 and 2022 with the new rows; its plants caught as named.

## done criteria

All seven fixed and checked; processes re-accepted at 0.2.0; every family accepted.
