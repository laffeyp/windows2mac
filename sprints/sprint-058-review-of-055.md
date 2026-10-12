# Sprint 058 — a review of Sprint 055

```yaml
---
id: 058
status: closed
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

- created: `tests/test_kernel.py` (the Machine checks), `tests/test_vm_symbols.py`
- modified: the files in the table; `probes/processes/processes.c` and the processes contract (0.2.0);
  `tests/test_runtime_native.py`; `scripts/family_run.py`
- gates 0 and CI `success` on the closing commit

## observation contract

- **Each check fails on d5ecffe8's code and passes after its fix.**
- **processes 0.2.0 equals Windows** on Server 2025 and 2022 with the new rows; its plants caught as named.

## done criteria

All seven fixed and checked; processes re-accepted at 0.2.0; every family accepted.

## observed

- **Each check failed on d5ecffe8 for the reason it names and passes now.** On the old code:
  - the Machine tests got status 1 (the reader waits) where the broken pipe is 2;
  - object ids 1, 2, 3 ... overlapped process ids 4096 and 4100;
  - the child got no environment or directory from its parent;
  - a pipe took 10,000 bytes at once;
  - the native harnesses went on past 64 handles (exit 0, not 88), reported 6,000 of 10,000 bytes written without
    waiting, and answered the program's directory;
  - the first of two VMs faulted in the second image's padding at its CPUID (ESR 0x2000000).
- **Windows decided the pipe and the inheritance** (runs 38189162864, 38189164581; four executions agree):
  - the child reads the parent's variable as set just before the start, and its current directory is the parent's;
  - the parent's read answers ERROR_BROKEN_PIPE once the writer's process ends holding its end;
  - a pipe of size 0 takes 4096 one-byte writes, then the writer waits;
  - a 10,000-byte write is still waiting with 9,000 left to read and returns with 4,000 left, so a write returns once
    its rest fits.

  A first round (38189082105, 38189083883) could not tell that from "returns once all is read"; the 6,000-byte row
  was added for it.
- **The fixes:**
  - the Machine records each process's handles on its objects and closes them when the process ends;
  - object ids start at 2^32;
  - PROCESS_CREATE passes the parent's environment block as it is now (`VM.environment_now`) and its working
    directory (an inherited-list entry, KIND_DIRECTORY; the inherit page at 0x800; paths.c reads it);
  - `w2m_object_inheritable` stops at a 65th handle;
  - the pipe holds 4096 bytes, and WriteFile blocks and is retried with its progress in the thread's scratch;
  - CreatePipe with a size of its own stops the run;
  - `compile_insn` takes the compiling VM's symbols, and the shared table is gone.
- **Found while running the plants:**
  - `host_patched` rebound names only in modules named w2m*, so the two kernel plants changed nothing: family_run
    holds its own `Machine` and `spawner` (KIT_DIARY #49);
  - a deadlock raised out of `Machine.run`, so the run's output was lost and the plant run crashed. The main VM's run
    now stops there, as on any Unimplemented.

  Both are fixed and tested.
- **processes 0.2.0 equals Windows** with 35 rows (CreateProcessW's and CreatePipe's pages cited).
- **The reaches.** Its ten plants are caught as named after three reaches were amended, each for a recorded reason:
  - broken_pipe_as_end_of_file and exit_code_unread also change writer.pipe, which reads to a broken pipe and prints
    the writer's exit code;
  - write_reports_part deadlocks the probe before it prints, so it changes every row.
- **Every family accepted, every plant as named.** The 56 item runs and their plants passed twice: once after the
  fixes and once after the symbol spelling changed vm_backend.py. The Sprint 042 file run was re-run (file.c
  changed), conforms, and its plants are caught as named.
- **The frontier** stops where Sprint 055 left it: GetCurrentDirectoryA after 57,411,545 instructions.
- **Left:** CreateProcessW with STARTF_USESTDHANDLES and bInheritHandles FALSE still runs instead of stopping (the
  card's item 4, for 059's start).
- **Closed on CI run 38190593421** (success on 56ffb04b).
