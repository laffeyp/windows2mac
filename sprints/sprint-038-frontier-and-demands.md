# Sprint 038 — the frontier and the game's demands

```yaml
---
id: 038
status: open
phase: 3
pass_kind: functional
---
```

## scope

Roadmap `docs/plan.md`, Sprint 038. Measure what Hail to the Rainbow needs, as signals, from both sides:

1. **The frontier as a topology** (`scripts/run_frontier.py`): one substrate run loads the game's executable in w2m and runs it until it stops; a stop on a Windows service w2m lacks emits `SERVICE_UNIMPLEMENTED` (vocabulary 0.6, P15; `BRIDGE_CALL_UNMAPPED` is for Steam interface methods, v0.2 §4.2), any other stop `EXECUTION_COMPLETED` with its exit status, each with the instruction count and the guest address; a `next_item` producer, triggered by the stop, records the item the frontier needs next. (Starting item runs and resuming on their acceptance arrives with the pools, Sprints 041 and 045.)
2. **The runner's view** (`tools/jit_capture` gains an import-call log): the launcher hooks the IAT entries of the game's images and counts each Windows function called, per thread; the run records peak and total thread counts (a Toolhelp snapshot each second). CPUID reads are not observable on Windows from user mode (CPUID does not trap without a hypervisor), so they are recorded by w2m when it executes CPUID, from Sprint 040's guest CPUID profile on; this sprint records which instruction-set extensions the startup capture's code uses (VEX or not), the visible effect of Mono's CPUID check. A play-time capture: the game with its graphics device on the runner's software renderer (no `-nographics`) and scripted input past the main menu, for a fixed time.
3. **Emitters** for work already done and repeated: `DISTRIBUTION_RESOLVED`, `DISTRIBUTION_ACQUIRED` and `ACQUISITION_FAILED` from a SteamCMD acquisition driver (`scripts/acquire_depot.py`; run for both depots, which verify in place); `CONTRACT_PROPOSED`, `CONTRACT_ACCEPTED`, `CONTRACT_REVISED` and `EVIDENCE_RECORDED` from a contract-registry run over `contracts/` (the accepted versions and their supersedes chain); `PROBE_AUTHORED` for each probe binary in `store/binaries/` and each runner probe.
4. **`src/w2m/record.py`**: one helper for a session's SESSION_INIT, events and SESSION_COMPLETED, used by the new scripts.

## prerequisites

- 037; Decisions 2026-10-09 (review progress applied; roadmap)

## context_files

- `docs/plan.md`; `docs/review-2026-10-09-progress.md` §3.2 N1, N5, N9, §6; vocabulary 0.5 payloads for the tags above; `scripts/run_candidate_loop.py` (topology shape); `tools/jit_capture/jit_capture.c`; `.github/workflows/windows-game.yml`

## signal contract

### Emits

- harness plane: `SERVICE_UNIMPLEMENTED` (frontier; vocabulary 0.6); `DISTRIBUTION_*`; `CONTRACT_*`, `EVIDENCE_RECORDED`; `PROBE_AUTHORED`. Each first emission of its tag on a real run.

### Invariants

- Vocabulary 0.6 (P15 `SERVICE_UNIMPLEMENTED`, P16 `acquisition_tool` += `steamcmd`): reading the payloads showed no tag for a frontier stop and no SteamCMD tool value. Every new record replays (`scripts/check_replay.py`).

## artifact contract

### Files created / modified

- created: `scripts/run_frontier.py`, `scripts/acquire_depot.py`, `scripts/record_contracts.py`, `scripts/record_probes.py`, `src/w2m/record.py`, tests
- modified: `tools/jit_capture/jit_capture.c` (format 3: import calls, threads), `src/w2m/jit_capture.py`, `.github/workflows/windows-game.yml` (play-time mode)

### Command exit codes

- `scripts/gates.sh` → 0

## observation contract

### Expected runtime signals

- Frontier: the first stop is `SERVICE_UNIMPLEMENTED` for `KERNEL32.dll!GetSystemTimeAsFileTime` after 13 instructions (the hand measurement of 2026-10-09).
- Runner call log: includes the functions w2m serves that the game calls (at least GetCurrentThreadId and QueryPerformanceCounter, which the CRT start-up reaches) and a total of distinct functions at most the 634 statically imported plus those resolved by GetProcAddress; peak thread count is a positive integer on the record.
- Acquisition: both depots verify against Steam (`validate`), `DISTRIBUTION_ACQUIRED` names build 24247624 and 8552776.
- Contracts: one `CONTRACT_ACCEPTED` per accepted contract version in `contracts/`, and one `CONTRACT_REVISED` per supersedes link.
- Play-time capture: more methods than the startup capture's 4,500, or the Unity log says why play did not start.

## done criteria

The game's next need, its Windows calls, its threads and its play-time code are on the record, and the vocabulary's acquisition, contract and probe tags fire on real runs.

## self-review checklist

- [x] Scope concrete. [x] Sources named. [x] No new vocabulary. [x] Observation contract states outcomes. [x] File count recorded (about 12; one concept: measure the game's demands).

## observed
- Vocabulary 0.6 (P15 `SERVICE_UNIMPLEMENTED`, P16 `steamcmd`): reading the payloads showed `BRIDGE_CALL_UNMAPPED` is for Steam interface methods; the roadmap and the review had planned to misuse it. 62 tags; `check_vocab.py` 0.
- Frontier (`store/runs/frontier-643a6283510c-1791588877289222000`): SESSION_INIT, EXECUTION_STARTED, `SERVICE_UNIMPLEMENTED` KERNEL32.dll!GetSystemTimeAsFileTime after 13 instructions (return address 0x14000135A), SESSION_COMPLETED — as expected.
- Acquisition (`store/runs/acquire-1699230-1791588962620913000`, `acquire-409720-1791588973967301000`): Hail to the Rainbow build 24247624, depot 1699231, 15,584,319,636 bytes; BioShock 2 Remastered build 8552776, depot 409721, 21,582,474,233 bytes on disk (the depot's 21,417,322,317 plus redistributables from shared depots). A first regex resolver returned BioShock's shared depots 228983 and 1523211; a KeyValues parser fixed it and a test pins it (stored app info). The auth_required path is tested with a stub, not live (each failed login counts against Steam's lockout).
- Contracts (`store/runs/contracts-1791589065188495000`): 37 EVIDENCE_RECORDED, 7 CONTRACT_PROPOSED, 5 CONTRACT_ACCEPTED, 3 CONTRACT_REVISED each followed by DEPENDENTS_INVALIDATED (last-error 0.1.0 → 0.2.0 invalidates 3 accepted candidates). Probes: 11 PROBE_AUTHORED; runner builds of identical source differ byte for byte.
- Runner runs 38005533923 and 38005541766 failed in unpack: the game's release also held Sprint 033's capture asset, the workflow downloaded every asset, and zstd rejected it after 10 minutes. Fixed: only the parts `release.json` names are fetched and unpacked, in order.
- Runner run 38006665419 (batch, 180 s, AMD EPYC 7763): 803 imports hooked in UnityPlayer and Mono, 213 called (kernel32 163, user32 25, advapi32 9, ole32 8), 77,318,157 calls; 331 names resolved with GetProcAddress (CreateDXGIFactory among them); peak 32 threads, 35 distinct; capture 4,500 methods (564 Assembly-CSharp).
- Runner run 38006671518 (play, 300 s): Unity created its D3D11 device on "Microsoft Basic Render Driver" (no GPU), FMOD fell back to silent output, the video decoder to software, `SteamAPI_Init` failed; 258 imports called (user32 54), 62,682,380 calls; 341 names resolved, including D3D11CreateDevice, CreateDXGIFactory2, XInputGetState; peak 41 threads, 45 distinct — under the 64-vCPU cap but within 23 of it; capture 5,309 methods (703 Assembly-CSharp), more than startup's 4,500, as expected. The job ended red after the capture uploaded (the key-sending loop; to read when play mode moves to the local VM). Logs and import tables in `store/game-runs/{38006665419,38006671518}/`; captures are assets of the private game release.
- Hot paths: GetCurrentThreadId 27.8M, TlsGetValue 23.3M, QueryPerformanceCounter 16.9M calls in 180 s; Decision 2026-10-09 keeps them in the guest.

