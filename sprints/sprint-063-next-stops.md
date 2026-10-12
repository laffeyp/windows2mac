# Sprint 063 — the next stops: kernel.appcore and the calls to seq 3426

```yaml
---
id: 063
status: open
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
