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

1. **The frontier as a topology** (`scripts/run_frontier.py`): one substrate run loads the game's executable in w2m and runs it until it stops; a stop on an import with no service emits `BRIDGE_CALL_UNMAPPED`, any other stop `EXECUTION_FAILED`, each with the instruction count and the guest address; a `next_item` producer, triggered by the stop, records the item the frontier needs next. (Starting item runs and resuming on their acceptance arrives with the pools, Sprints 041 and 045.)
2. **The runner's view** (`tools/jit_capture` gains an import-call log): the launcher hooks the IAT entries of the game's images and counts each Windows function called, per thread; the run records peak and total thread counts (a Toolhelp snapshot each second). CPUID reads are not observable on Windows from user mode (CPUID does not trap without a hypervisor), so they are recorded by w2m when it executes CPUID, from Sprint 040's guest CPUID profile on; this sprint records which instruction-set extensions the startup capture's code uses (VEX or not), the visible effect of Mono's CPUID check. A play-time capture: the game with its graphics device on the runner's software renderer (no `-nographics`) and scripted input past the main menu, for a fixed time.
3. **Emitters** for work already done and repeated: `DISTRIBUTION_RESOLVED`, `DISTRIBUTION_ACQUIRED` and `ACQUISITION_FAILED` from a SteamCMD acquisition driver (`scripts/acquire_depot.py`; run for both depots, which verify in place); `CONTRACT_PROPOSED`, `CONTRACT_ACCEPTED`, `CONTRACT_REVISED` and `EVIDENCE_RECORDED` from a contract-registry run over `contracts/` (the accepted versions and their supersedes chain); `PROBE_AUTHORED` for each probe binary in `store/binaries/` and each runner probe.
4. **`src/w2m/record.py`**: one helper for a session's SESSION_INIT, events and SESSION_COMPLETED, used by the new scripts.

## prerequisites

- 037; Decisions 2026-10-09 (review progress applied; roadmap)

## context_files

- `docs/plan.md`; `docs/review-2026-10-09-progress.md` §3.2 N1, N5, N9, §6; vocabulary 0.5 payloads for the tags above; `scripts/run_candidate_loop.py` (topology shape); `tools/jit_capture/jit_capture.c`; `.github/workflows/windows-game.yml`

## signal contract

### Emits

- harness plane: `BRIDGE_CALL_UNMAPPED` or `EXECUTION_FAILED` (frontier); `DISTRIBUTION_*`; `CONTRACT_*`, `EVIDENCE_RECORDED`; `PROBE_AUTHORED`. Each first emission of its tag on a real run.

### Invariants

- No vocabulary change. Every new record replays (`scripts/check_replay.py`).

## artifact contract

### Files created / modified

- created: `scripts/run_frontier.py`, `scripts/acquire_depot.py`, `scripts/record_contracts.py`, `scripts/record_probes.py`, `src/w2m/record.py`, tests
- modified: `tools/jit_capture/jit_capture.c` (format 3: import calls, threads), `src/w2m/jit_capture.py`, `.github/workflows/windows-game.yml` (play-time mode)

### Command exit codes

- `scripts/gates.sh` → 0

## observation contract

### Expected runtime signals

- Frontier: the first stop is `BRIDGE_CALL_UNMAPPED` for `KERNEL32.dll!GetSystemTimeAsFileTime` after 13 instructions (the hand measurement of 2026-10-09).
- Runner call log: includes the functions w2m serves that the game calls (at least GetCurrentThreadId and QueryPerformanceCounter, which the CRT start-up reaches) and a total of distinct functions at most the 634 statically imported plus those resolved by GetProcAddress; peak thread count is a positive integer on the record.
- Acquisition: both depots verify against Steam (`validate`), `DISTRIBUTION_ACQUIRED` names build 24247624 and 8552776.
- Contracts: one `CONTRACT_ACCEPTED` per accepted contract version in `contracts/`, and one `CONTRACT_REVISED` per supersedes link.
- Play-time capture: more methods than the startup capture's 4,500, or the Unity log says why play did not start.

## done criteria

The game's next need, its Windows calls, its threads and its play-time code are on the record, and the vocabulary's acquisition, contract and probe tags fire on real runs.

## self-review checklist

- [x] Scope concrete. [x] Sources named. [x] No new vocabulary. [x] Observation contract states outcomes. [x] File count recorded (about 12; one concept: measure the game's demands).

## observed
