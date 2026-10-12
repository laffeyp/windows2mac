# Sprint 060 — the frontier on the record, the game's calls as a judge

```yaml
---
id: 060
status: open
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
   untraced (its `UNTRACED` list) are left untraced here too. The frontier writes the stream into its record; the
   record's `trace_sha256` and `guest_line_count` name it.
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

- The frontier: SESSION_INIT, EXECUTION_STARTED, its stop (SERVICE_UNIMPLEMENTED or EXECUTION_COMPLETED with the
  call stream's digest and line count), COMPARISON_COMPLETED against run 38176899195, SESSION_COMPLETED.

### Invariants

- Vocabulary 0.7. The lift refusal's tag is a proposal (`signals/proposals.json`), not used until a version adopts it.

## artifact contract

- created: `specs/signatures.json` and the script that writes it; the comparator and its tests
- modified: `scripts/run_frontier.py`, `src/w2m/vm_backend.py`, `WORKING_AGREEMENT.md`, `docs/plan.md`,
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
