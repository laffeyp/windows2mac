# Sprint 063 — the next stops: kernel.appcore and the calls to seq 3426

```yaml
---
id: 063
status: closed
phase: 5
pass_kind: functional
---
```

## scope

The frontier stops at the C runtime's load of `api-ms-win-appmodel-runtime-l1-1-2`. In game run 38206263976
(seq 1764–1765), Windows answers that load with kernel.appcore.dll at 0x7ffd45840000, then
AppPolicyGetThreadInitializationType(-6, out) answers 0. After it, the run calls 20 functions w2m does not serve, in
five independent groups (recounted against `service_for`, first seq of each):

| Group | Functions (first seq) | Guest file |
|---|---|---|
| A. single calls | EnumProcessModules (2128), GetConsoleOutputCP (2211), SetDllDirectoryW (2915) | module.c, console.c |
| B. SList | InterlockedPopEntrySList (2705), InterlockedFlushSList (3002), InterlockedPushEntrySList (3210) | a new slist.c |
| C. registry writes | RegCreateKeyW (2734), RegSetValueExA (2740), RegQueryValueExA (2891), RegDeleteValueA (2895) | registry.c |
| D. windows and messages | SetWindowLongPtrA (2592), PeekMessageA (2945), GetMessageExtraInfo (2948), TranslateMessage (2949), DispatchMessageA (2950), RegisterDeviceNotificationW (3382), GetFocus (3383), SetFocus (3384), LoadImageW (3426) | windows.c |
| E. DXGI | CreateDXGIFactory (3300) | a new dxgi.c |

1. **The API set.** `api-ms-win-appmodel-runtime-l1-1-2` is hosted by kernel.appcore.dll (the capture's
   GetProcAddress hook names it), and kernel.appcore's AppPolicyGetThreadInitializationType is the service w2m
   already has (`threads.c`).
2. **The groups in parallel worktrees** (Decision 2026-10-11 on parallel worktrees), one agent and one branch per
   group. Each group follows the families' pipeline:
   - a probe of the game's own arguments (the trace's) and the boundary rows the code accepts (KIT_DIARY #47);
   - two runner images;
   - a contract;
   - the guest C;
   - plants with named reaches.
   The groups merge one at a time into this branch, and each merged group's item run is repeated after the merge.
   The shared points are `scripts/family_run.py`, `scripts/work_list.py` and `vm_backend.py`'s tables.
3. **The frontier** runs after each merge. Its comparison with game run 38206263976 stays `conforms`; a divergence
   it finds is closed in the group that caused it.

## prerequisites

- 062 (closed: CI run 38209558822; main 8c648d77)

## context_files

- `store/game-runs/38206263976/jit_capture.bin.trace.tsv` and `.strings.tsv` (the calls and their arguments);
  `src/w2m/modules.py` (API_SET_HOSTS), `src/w2m/vm_backend.py` (`service_for`); `tools/guest_runtime/registry.c`,
  `windows.c`, `messages.c`, `module.c`, `console.c`; `scripts/family_run.py`; `scripts/work_list.py`

## signal contract

### Emits

- Each group's PROBE_AUTHORED and item run (SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED per runner
  binary, CANDIDATE_ACCEPTED, SESSION_COMPLETED); the frontier's records with TRACE_INGESTED and
  COMPARISON_COMPLETED.

### Invariants

- Vocabulary 0.7; no new tags.

## artifact contract

- created: each group's probe and contract; `tools/guest_runtime/slist.c`, `dxgi.c`
- modified: `src/w2m/modules.py`, `src/w2m/vm_backend.py`, the guest files in the table, `scripts/family_run.py`,
  `scripts/work_list.py`
- gates 0 and CI `success` on the closing commit

## observation contract

- **Every group's family equals Windows on both images,** its plants caught as named.
- **The frontier passes seq 3426** or stops at a call this card does not list, and its comparison with game run
  38206263976 conforms.

## done criteria

The five groups accepted and merged; the frontier past seq 3426 with a conforming comparison, its new stop recorded.

## observed

- **The API set.** `api-ms-win-appmodel-runtime-l1-1-2` is kernel.appcore.dll, and its
  AppPolicyGetThreadInitializationType is `threads.c`'s service. The check failed first. The frontier then ran
  102,817,503 instructions to EnumProcessModules (seq 2128), its comparison conforming over 2,152 calls.
- **The five groups, each built in its own worktree by its own agent** (Decision 2026-10-11 on parallel worktrees),
  then merged one at a time:
  - **A, module_search 0.1.0 and console 0.3.0.** EnumProcessModules walks PEB.Ldr, which `_load_dll` now keeps
    current; SetDllDirectoryW adds a directory for bare names; GetConsoleOutputCP answers 0 with error 6 with no
    console. Probe runs 38210455471 and 38210458062, and 38210455282 and 38210458372.
  - **B, interlocked_lists 0.1.0.** SList headers as both images lay them out; the sequence step is Server 2025's.
    Runs 38210349431 and 38210351696.
  - **C, registry 0.3.0.** The game's PlayerPrefs writes under `HKCU\Software\NSS\Hail to the Rainbow`; the
    registry lasts one run. Runs 38210383833 and 38210386046.
  - **D, windows 0.3.0.** The message loop's posted messages and WM_QUIT, SetFocus's messages in the game's order,
    the device notification, and LoadImageW. Runs 38210803867 and 38210806334.
  - **E, dxgi 0.1.0.** A factory with two Microsoft Basic Render Driver adapters and one output, as the GPU-less
    runners give. Runs 38210591842 and 38210594155.
  After the last merge, 60 families are accepted with every plant as named, 2,066 tests pass, and the gates pass.
- **A defect found on the way: the frontier never kept the game's log.** `run_frontier.py` copied unity.log after
  `close_all` had removed drive C:, so no run since Sprint 054 kept it. Fixed. The log now holds 13 lines, 12 equal
  to Windows'; the 13th is the assembly reload's time: 113.9 s against 0.101 s.
- **Not met: the frontier does not pass seq 3426.** A run of 4,000,000,009 instructions (689 s, record
  `frontier-643a6283510c-1791787123623878000`) reaches none of groups A–E's functions after EnumProcessModules. Its
  comparison conforms over 2,333 calls. At the stop:
  - **Where the threads stand.** The main thread is in UnityPlayer's deserialization loop at RVA 0xde12d0, which
    reads 4-byte values into 0x98-byte records. The 24 workers are in WaitForSingleObjectEx.
  - **The call counts.** Between 1 and 4 billion instructions, HeapAlloc stays at 164,024 calls while
    GetCurrentThreadId grows from 0.45 to 2.46 million.
  The game works and is slow, so the criterion moves to plan row 064 (speed), by the partial-pass rule (Decision
  2026-10-11, agent-resolved).
- **Closed partial on CI run 38215049553** (success on 71c0d80a).
