# Sprint 060 — the frontier on the record, the game's calls as a judge

```yaml
---
id: 060
status: closed
phase: 5
pass_kind: functional
---
```

## scope

The ground-up review of 2026-10-11 and its verification (`docs/review-2026-10-11-ground-up.md`,
`docs/review-2026-10-11-ground-up-verification.md`; Decision 2026-10-11 user) found that every Windows family is judged
on rows its writer saw (F2 open), and that the game's own 3,496 calls in batch run 38176899195 judge nothing. They also
found that the frontier's records are not on the record. `run_frontier.py` writes to the ignored `runs/`, 15 of the 20
frontier records in `store/runs/` name a dirty tree, and each record carries `trace_sha256=EMPTY`.

1. **Frontier records on the record.** `run_frontier.py` writes to `store/runs/` and refuses to start from a tree with
   uncommitted changes to anything the run executes; a scratch run (`--scratch`) writes to `runs/` and says so in its
   record. An unresolved API set stops as SERVICE_UNIMPLEMENTED, the tag the frontier topology already subscribes to.
2. **w2m's call stream.** w2m records the calls UnityPlayer.dll and mono-2.0-bdwgc.dll make through their imports and
   through names they resolve with GetProcAddress, as `tools/jit_capture/call_trace.h` records them on Windows: for the
   first 32 calls of each function, the sequence number taken on entry, the thread, eight integer arguments (four
   registers, four stack slots), the return value and the last error after the call. The functions call_trace.h leaves
   untraced (its `UNTRACED` list) are left untraced here too. The frontier writes the stream as a guest trace in the
   content store (`store/traces/<sha256>.jsonl`: GUEST_TRACE_OPENED, a GUEST_THREAD_STARTED per thread, a
   GUEST_CALL_RETURNED per call, GUEST_TRACE_CLOSED), ingested by `trace.py` into TRACE_INGESTED; the raw values the
   comparator reads go beside it (`store/frontier-calls/<execution id>.tsv`, in the Windows trace's columns).
   (Amended 2026-10-11 before the build: vocabulary 0.7 has the tags, and SERVICE_UNIMPLEMENTED carries no digest.)
3. **Signatures.** Each traced function's parameter and return types come from win32metadata 71.0.30-preview
   (`Windows.Win32.winmd` in NuGet package Microsoft.Windows.SDK.Win32Metadata; package sha256 b1f4ae72…2a3b, winmd
   8cce5eb3…e616), read with dnfile 0.18.0, and are written once to `specs/signatures.json` with the package's
   version and digest. A function the game calls and the metadata does not describe is listed by name.
4. **The comparator.** For each function, w2m's calls are compared in order with Windows's, each value by the relation
   its type allows. An integer, flag or enumeration must be equal. A pointer must agree on null or non-null. A handle
   must agree on null, on INVALID_HANDLE_VALUE, or on being a pseudo-handle. A BOOL or HRESULT return must be equal; a
   last error must be equal where the function's metadata says it sets one. The frontier records one COMPARISON_COMPLETED
   against the reference run, its first_divergence naming the function, the call's index and the field. The divergences
   are listed in full in the card's observed section, each with a disposition.
4a. **The capture's conditions, reproduced** (amended 2026-10-11 after the first three comparisons, before the
   work). The first runs showed divergences that are the capture's, not the game's:
   - **The trace window.** `jit_capture.c` hooks after mono's and UnityPlayer's DllMain and opens at UnityMain;
     w2m's trace opens at UnityMain and closes around a traced image's DllMain.
   - **Hook labels.** An image's own import slots are labeled with the image, and GetProcAddress's answers with
     "GetProcAddress"; a pointer resolved before the hooks is the real function and is never traced.
   - **The monitor thread.** The capture's own thread is excluded in the contract.
   - **Values the system assigns.** Ids, slots and atoms map one to one, and clocks are not compared.
   - **Addresses in integer types.** An integer that carries an address (a range base, a message's lParam, a
     window's user data) agrees on null.
   - **The machine's paths.** The runner unpacked the game at `D:\w2m\game` (12 characters to w2m's 11 at
     `C:\w2m\app`), so every length derived from a path differed by one. A new reference run starts the game
     through a junction at `C:\w2m\app`, with TMP and TEMP at w2m's temp directory, and becomes the reference.
5. **The documents.**
   - **K1 Decision.** One Decision with grounds on parallel worktrees.
   - **G3 Decision.** One Decision stating the runtime architecture as built, with waits, scheduling, virtual memory
     and shared objects in the host, superseding item (3) of 2026-10-09.
   - **K2 proposal.** A vocabulary proposal for the lift refusal (K2).
   - **WORKING_AGREEMENT.** Brought current: vocabulary 0.7, the substrate clone, and the registry rows.
   - **The plan.** Its stale rows cleaned, and a row for the §12.1 product slice and the `w2m` command (G1).

## prerequisites

- 059 (closed: CI run 38195353233 on 4a27afe9; main bae2a4da)

## context_files

- `scripts/run_frontier.py`, `tools/jit_capture/call_trace.h`, `src/w2m/vm_backend.py` (the import thunks, the ring),
  `store/game-runs/38176899195/jit_capture.bin.trace.tsv` and `.strings.tsv`; `substrate/src/substrate/projections/inspect.py`;
  win32metadata 71.0.30-preview; the review and its verification

## signal contract

### Emits

- The frontier: SESSION_INIT, EXECUTION_STARTED, its stop (SERVICE_UNIMPLEMENTED, or EXECUTION_COMPLETED with the
  guest trace's digest and line count), TRACE_INGESTED, COMPARISON_COMPLETED against run 38176899195,
  SESSION_COMPLETED. The guest trace: GUEST_TRACE_OPENED, GUEST_THREAD_STARTED, GUEST_CALL_RETURNED,
  GUEST_TRACE_CLOSED.

### Invariants

- Vocabulary 0.7. The lift refusal's tag is a proposal (`signals/proposals.json`), not used until a version adopts it.

## artifact contract

- created: `specs/signatures.json` and `scripts/signatures.py`; `tools/guest_runtime/call_trace.c`;
  `scripts/call_compare.py` and its tests; `contracts/windows.game.calls/0.1.0.json` (the comparison's relations and
  reference)
- modified: `scripts/run_frontier.py`, `src/w2m/vm_backend.py`, `.github/workflows/windows-game.yml`,
  `WORKING_AGREEMENT.md`, `docs/plan.md`,
  `signals/proposals.json`, BLACKBOARD
- gates 0 and CI `success` on the closing commit

## observation contract

- **The frontier's record in `store/runs/`** names a clean commit, holds the call stream's digest and a
  COMPARISON_COMPLETED against run 38176899195 with its first divergence, and stops where 059 stopped.
- **Planted faults:** a wrong return in a served function and a dropped last error are each caught by the comparison
  at the call they change.

## done criteria

The frontier records its call stream and its comparison on the record; the plants caught; the divergences listed with
dispositions; the documents current.

## observed

- **The frontier is on the record.** Its records go to `store/runs/` and name a clean commit. The last three name
  e9f3a6e2; a run from a dirty tree is refused, with a test. The appmodel API set now stops as SERVICE_UNIMPLEMENTED,
  with its return address; the check failed first, with `unimplemented` None. The frontier stops where 059 stopped,
  after 57,409,405 instructions with no fault.
- **The record carries the call stream.** `call_trace.c` records UnityPlayer's and mono's calls as `call_trace.h`
  does: 1,769 guest-trace lines over 22 threads, ingested to TRACE_INGESTED, with the raw values in
  `store/frontier-calls/`. Then a COMPARISON_COMPLETED against `game-run-38203364352`, which
  `scripts/record_game_run.py` put on the record as a `windows_reference` execution. Game runs had no execution on
  the record before.
- **Signatures** for 220 Windows functions come from win32metadata 71.0.30-preview through dnfile 0.18.0. Two that
  the game calls are absent from the metadata: DllGetVersion and RtlVerifyVersionInfo.
- **Most first divergences were the capture's, not the game's.** 790 at first, then 487, 160, 55. Each class was
  traced to its cause before any relation changed:
  - **The window.** The capture hooks after mono's and UnityPlayer's DllMain and opens at UnityMain
    (`jit_capture.c:189–239`).
  - **Labels.** Each hooked image's slots, and "GetProcAddress" for answers resolved after the hooks. A pointer
    resolved before them is never traced.
  - **The monitor thread.** The capture's own thread (6636, then 5368) makes DllMain thread-attach calls only.
  - **Assigned values.** Thread and process ids, TLS and FLS slots, atoms and clocks.
  - **Addresses in integer types.**
  - **A defect in my comparator.** `call_trace.h` prints the last error in decimal (line 286), which the first
    reader took for hex. That produced every "error" divergence of the first runs.
  - **The machine's paths.** The runner unpacked the game at `D:\w2m\game`, one character longer than
    `C:\w2m\app`. Run 38203364352 starts the game through a junction at `C:\w2m\app`, with w2m's temp directory,
    and its GetModuleFileNameW and GetTempPathW answer w2m's lengths (0x22, 0x20).
- **The plants are caught at the call they change.**
  - `thread_priority_off_by_one` at UnityPlayer's GetThreadPriority call 0 (Windows 0xfffffffe, w2m 0xffffffff).
  - `envvar_error_dropped` at mono's GetEnvironmentVariableW call 0 (Windows 203, w2m 3). The wrong error also sent
    mono down another path, and that run stopped at CreateFileA after 3.65 million instructions.
- **55 divergences remain against run 38203364352, with dispositions:**
  - **Defect, FLS shared by every thread (46).** `thread_storage.c` keeps FLS values in process-wide statics
    ("TLS and FLS on one thread", Sprint 044), unchanged when threads came in Sprint 051. A new thread's first
    FlsGetValue reads another thread's pointer where Windows answers NULL and 87 (rows `fls.get.fresh`,
    `fls2.unset`). The C runtime's per-thread data is shared.
  - **Defect, the time zone's names are empty (2).** Mono converts StandardName: Windows gives 27 bytes ("Coordinated
    Universal Time"), w2m 1. `time_and_error_mode.c:14` says the names are "not in the rows (w2m writes them empty)",
    an input accepted without a row (KIT_DIARY #47).
  - **Defect, EnumDisplayMonitors answers FALSE (1),** where Windows answers TRUE.
  - **Defect, LoadIconA(exe, 103) answers an icon (2),** where Windows fails with 1813
    (ERROR_RESOURCE_TYPE_NOT_FOUND): the exe has no icon 103.
  - **Defect, CreateWindowExW answers NULL (1)** after sending the window the three messages Windows sends (0x24,
    0x81, 0x83). Windows answers the window.
  - **w2m's choice, one processor (1).** `system_info.c` reports one processor and the runner four. Unity starts six
    job workers on Windows and four in w2m, so CreateThread call 20 pairs different threads.
  - **The machine, the profile's name (3).** Mono's `C:\Users\runneradmin\.mono` is 8 characters longer than
    `C:\Users\w2m\.mono` (0x3f against 0x37). The laptop (Decision 2026-10-11, user) can run as w2m.
- **Every family is accepted, every plant caught as named,** after `vm_backend.py` and the runtime changed. The
  Sprint 042 file run conforms. 1,907 tests pass, and `scripts/gates.sh` passes.
- **The documents.** Decisions on parallel worktrees (from 062) and on the architecture as built; proposal P18;
  WORKING_AGREEMENT at vocabulary 0.7 with its registry rows; the plan's stale rows corrected, the host share in its
  model-step row, and a row for `w2m steam run`.
- **Closed on CI run 38205517561** (success on 104f1db8).
