# Sprint 061 — every divergence of the game's calls closed

```yaml
---
id: 061
status: open
phase: 5
pass_kind: functional
---
```

## scope

Sprint 060's frontier compared the game's calls in w2m with its calls on Windows (game run 38203364352) and found 55
divergences. The user decided on 2026-10-11 that every one is closed by matching the reference (Decision 2026-10-11,
user). The divergences fall into seven causes:

1. **A call that runs guest code is recorded too early (2).** CreateWindowExW and EnumDisplayMonitors run the window
   procedure or the callback through a redirect (`guest_calls.c`). The trace records the service's first return (0),
   while the game receives the window and TRUE through `w2m_return_to`. Such a record completes there, matched by
   thread and stack pointer.
2. **FLS is shared by every thread (46).** `thread_storage.c` keeps FLS values in process-wide statics. They become
   the thread's, as the TEB keeps TLS: a new thread's FlsGetValue answers NULL and 87 until the thread sets a value
   (rows `fls.get.fresh`, `fls2.unset`). FlsFree clears the index on every thread.
3. **The time zone's names are empty (2).** The time_and_error_mode probe records StandardName and DaylightName on
   both runners, and w2m answers them.
4. **LoadIconA on a missing icon answers an icon (2).** The windows probe records LoadIconA on the probe's own
   module with an id it lacks (Windows in the game: NULL, 1813), and w2m answers as Windows does.
5. **One processor (1).** w2m reports the runners' processors: the count, the masks and the layout the system_info
   and topology rows record. Unity then starts six job workers, as on Windows.
6. **The runner's user (3).** The profile is `C:\Users\runneradmin`, as on the runner. TEMP stays
   `C:\Users\w2m\AppData\Local\Temp`, where the reference run put it.
7. **The frontier is rerun and compared until no divergence remains.** Any divergence the earlier ones hid is closed
   the same way.

## prerequisites

- 060 (closed: CI run 38205517561 on 104f1db8; main 28928375)

## context_files

- `tools/guest_runtime/call_trace.c`, `guest_calls.c`, `thread_storage.c`, `time_and_error_mode.c`, `windows.c` or
  `resources.c`, `system_info.c`, `topology.c`, `known_folders.c`; `src/w2m/file_bridge.py`, `vm_backend.py`;
  `sprints/sprint-060-frontier-on-the-record.md` (observed); contracts windows.thread_storage.*, windows.system.*

## signal contract

### Emits

- Each changed family's item run (SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED per runner binary,
  CANDIDATE_ACCEPTED, SESSION_COMPLETED). Each new probe's PROBE_AUTHORED. The frontier's EXECUTION_STARTED, its
  stop, TRACE_INGESTED and COMPARISON_COMPLETED.

### Invariants

- Vocabulary 0.7; no new tags.

## artifact contract

- modified: the files in context_files that the causes name; the probes time_and_error_mode and windows (new rows);
  their contracts (minor versions); `scripts/family_run.py`; tests that fail before each fix
- gates 0 and CI `success` on the closing commit

## observation contract

- **The frontier's COMPARISON_COMPLETED against game run 38203364352 has verdict `conforms`.**
- **Every family is accepted, with every plant caught as named.**

## done criteria

No divergence between the game's calls in w2m and on Windows, up to the frontier's stop; every family accepted.
