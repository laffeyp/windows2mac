# Sprint 059 — the process parameters

```yaml
---
id: 059
status: closed
phase: 5
pass_kind: functional
---
```

## scope

The frontier stops after 57,411,545 instructions at GetCurrentDirectoryA, which no Windows run calls: a fault
before it, a read of address 8 at UnityPlayer RVA 0x143a6ca (`mov rcx, [PEB+0x20]; cmp dword [rcx+8], 0`), sends
UnityPlayer down its crash path. PEB+0x20 is ProcessParameters (RTL_USER_PROCESS_PARAMETERS), and w2m leaves it NULL.
A scan of the game's modules for `gs:[0x60]` followed by `+0x20` finds four direct reads, all of Flags (+8), by the
C runtime linked into UnityPlayer.dll (two), the game's exe and UnityCrashHandler64.exe; mono reads none. Every other
use goes through functions w2m serves.

1. **A probe** (`probes/process_parameters`, a GUI program as the game is) records the block on both runners: as the
   parent the runner starts with redirected handles, and as a child started as Steam starts the game (an empty
   STARTUPINFO, nothing inherited). For each start it records the header and flags, the console handle, the standard
   handles, the current directory, the DLL path, the image path, the command line, the environment and its size, the
   window fields against GetStartupInfoW, the title and desktop, the drive directories, and the block's allocation.
   It also records the block after SetCurrentDirectoryW, SetStdHandle and SetEnvironmentVariableW. Offsets are
   phnt's (Process Hacker's reverse-engineered `ntrtl.h`); ImagePathName (0x60) and CommandLine (0x70) agree with
   `winternl.h` on Microsoft Learn.
2. **w2m fills the block** as the rows show, for a console start, a GUI start and a child, and keeps it current where
   the rows show Windows does.
3. **The 058 gap:** CreateProcessW with STARTF_USESTDHANDLES and bInheritHandles FALSE stops the run (Microsoft
   Learn: the handles "must be inheritable and the function's bInheritHandles parameter must be set to TRUE").
4. **The frontier** runs past RVA 0x143a6ca without the fault; the next stop is recorded with the functions before it
   that w2m does not serve.

## prerequisites

- 058 (closed: CI run 38190593421 on 56ffb04b; main 2e2a8a01)

## context_files

- `src/w2m/vm_backend.py` (`_init_process`, the PEB), `tools/guest_runtime/paths.c`, `console.c`, `environment.c`,
  `processes.c`; game run 38176899195 (the batch trace); store/runs/frontier-643a6283510c-1791757748625534000

## signal contract

### Emits

- PROBE_AUTHORED for process_parameters; its item run (SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED per
  runner binary, CANDIDATE_ACCEPTED, SESSION_COMPLETED); every stale family's item run; the frontier's
  EXECUTION_STARTED and its stop.

### Invariants

- Vocabulary 0.7; no new tags.

## artifact contract

- created: `probes/process_parameters/`, `contracts/windows.process.parameters/0.1.0.json`
- modified: `src/w2m/vm_backend.py`, the runtime files that keep the block current, `scripts/family_run.py`
- gates 0 and CI `success` on the closing commit

## observation contract

- **process_parameters equals Windows** on Server 2025 and 2022, its plants caught as named.
- **The frontier passes RVA 0x143a6ca** with no fault logged there.

## done criteria

The family accepted; the frontier past the fault, with its next stop and the unserved functions before it listed.

## observed

- **Windows answered on both images** (runs 38194049597 on Server 2025, build 26100; 38194051809 on 2022, build
  20348; two executions each, all agreeing where no run decides the value):
  - **Header.** Flags are 0x4001 in every start, and DebugFlags, ConsoleFlags and LoaderThreads are 0.
    MaximumLength equals Length. Both grow with the strings: the child's block is 0x7c longer than the parent's, and
    2025's is 8 bytes longer than 2022's. ProcessGroupId is neither 0 nor the process's own id.
  - **Console handle.** NULL in both starts of this GUI program.
  - **Standard handles.** GetStdHandle's (NULL in the Steam-like start), and SetStdHandle changes them in the block.
  - **Current directory.** The directory and a final backslash, in a 0x208-byte buffer outside the block, with a
    non-null handle. SetCurrentDirectoryW moves it to another buffer.
  - **Strings inside the block.** DllPath is empty. ImagePathName is GetModuleFileNameW's path. CommandLine's buffer
    is the pointer GetCommandLineW returns. WindowTitle's and DesktopInfo's buffers are the pointers GetStartupInfoW
    returns, the desktop `winsta0\default`.
  - **Environment.** Outside the block; GetEnvironmentStringsW returns a copy. EnvironmentSize equals the bytes in
    use, and SetEnvironmentVariableW gives the block a new address and the new size.
  - **Window fields.** GetStartupInfoW's: all 0 for the child, the runner's for the parent.
  - **Allocation.** The block sits in the process heap: AllocationBase is GetProcessHeap(), MEM_PRIVATE, read-write.
- **w2m fills the block as the rows show.** `vm_backend._init_parameters` writes it at the start of the heap's arena,
  and heap.c's first block follows it. The services now read it:
  - console.c: the standard handles and the command line;
  - process.c: GetStartupInfoW's desktop, title and window fields;
  - environment.c: the block, its size and version; a change moves the block to the other half of an area doubled
    to 32 KB;
  - paths.c: the working directory, with SetCurrentDirectoryW alternating two buffers.

  A child's directory now goes straight into its block, replacing Sprint 058's inherit-page path; its process group
  is its parent's.
- **process_parameters 0.1.0 equals Windows**, 30 rows. The derivation keeps relations where the run decides the
  value: lengths, the heap's region size, and the parent's window values, title, handles' nullness and environment
  version, which are the runner's start. Its six plants are caught as named.
- **Found on the way:**
  - the host wrote an empty environment as one NUL, while GetEnvironmentStringsW counts two (row empty.heap); the
    family's empty start showed it as EnvironmentSize below the bytes in use;
  - the host-side candidate build needed a PEB stand-in;
  - test_hot_services lost GetStartupInfoW's module-path request, now answered from the block.
- **The 058 gap closed:** STARTF_USESTDHANDLES with bInheritHandles FALSE stops the run, with a native check that
  failed first (exit 0, not 88).
- **The frontier** runs 57,409,401 instructions with no fault dispatched (fault log: 0). It stops where the batch
  trace's seq 1768 is: the C runtime's new thread loads api-ms-win-appmodel-runtime-l1-1-2 with
  LOAD_LIBRARY_SEARCH_SYSTEM32 (Windows: kernel.appcore.dll) for AppPolicyGetThreadInitializationType. Every function
  the trace calls before it is served. The 20 unserved after it, to seq 3422, are plan row 060.
- **Every family accepted, every plant caught as named;** the Sprint 042 file run conforms.
- **Closed on CI run 38195353233** (success on 4a27afe9).
