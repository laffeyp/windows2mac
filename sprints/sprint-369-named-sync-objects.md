# Sprint 369 — Named sync objects the game declares: CreateMutexW, OpenMutexW, OpenEventW, OpenEventA, OpenSemaphoreW, SignalObjectAndWait

```yaml
---
id: 369
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "G3 met at Sprint 368, and the order after it" serves first the waits and threads of
`docs/unserved-declared-imports-2026-11-27.md`, imports the game declares, the Windows batch run never called, and w2m
does not serve, so that no G3 run in a later card's gates stops by name on one (the user's Decision of 2026-10-16,
"knowable first"). This card serves the first family of them, the named sync objects:
- mono-2.0-bdwgc.dll's CreateMutexW, OpenMutexW, OpenEventW, OpenSemaphoreW and SignalObjectAndWait;
- UnityPlayer.dll's OpenEventA and SignalObjectAndWait.

- **Windows' rows first.** A probe, `probes/named_sync_objects`, in the form of `probes/mutex_abandoned`, records each
  call's return, last error and effect on Windows: a named mutex created, opened by name, owned and released; a named
  event and a named semaphore opened by name with their access masks, and opened when absent
  (ERROR_FILE_NOT_FOUND); a name used by an object of another kind (ERROR_INVALID_HANDLE); SignalObjectAndWait on each
  kind of object it takes, its alertable flag false, with a timeout of 0 and a wait ended by another thread. Its
  expected rows are written from Microsoft Learn's pages for each function, quoted with their URLs; the Windows run
  settles them. Main commits the probe as a card amendment (`probes/` only) and runs the Windows workflow; the builder
  holds w2m to the run's rows.
- **The services**, in `tools/guest_runtime/sync_objects.c` beside CreateEventW, CreateSemaphoreExW and CreateMutexA,
  on the named-object table and the lock classes the handles design built (Sprints 352-356), each new service taking
  the next id in `crates/guest_runtime/src/api_ids.txt`.
- **The family run.** The probe joins `store/family_runs/known-2026-10-29.tsv` as a family, held to its contract's
  `exact_rows` on one vCPU and two by `family_runs.rs`.
- **Not here:** the timers, registered waits and thread calls of the class (the cards after this one).

## prerequisites

- 368 (closed)

## context_files

- `docs/unserved-declared-imports-2026-11-27.md`.
- `tools/guest_runtime/sync_objects.c` (the named objects, CreateEventW, CreateMutexA, CreateSemaphoreExW, the waits),
  `crates/guest_runtime/src/services.rs` (the lock classes and `api_ids.txt`'s reader), `crates/guest_runtime/src/api_ids.txt`.
- `probes/mutex_abandoned/` (a probe's form), `.github/workflows/windows-probe.yml`, `store/family_runs/known-2026-10-29.tsv`,
  `crates/w2m_vm/tests/family_runs.rs` 1-120, and one family contract under `contracts/` as the form.
- `docs/design-handles-and-sync-objects-2026-11-26.md`: the named objects and the lock classes.
- `grep -n "G3 met at Sprint 368\|The runtime's API ids fixed" BLACKBOARD.md`: the Decisions.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None: services and a family run; no emit site changes. The new family's run records are held to its contract.

### Invariants

- Every run as held: the batch run on one vCPU and two (G3), the 273 family runs on one vCPU and two, the guests, the
  exit ledger. No existing id in `api_ids.txt` moves.

## artifact contract

### Files created

- `probes/named_sync_objects/` (source, built image, expected rows) and its family contract under `contracts/`.

### Files modified

- `tools/guest_runtime/sync_objects.c`, `crates/guest_runtime/src/services.rs` and `api_ids.txt` (the services and their
  ids), `store/family_runs/known-2026-10-29.tsv` (the family), with others only where the class list requires, reported
  to main when found. One concept: the named sync objects the game declares.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The class first: each of the six functions' Learn page quoted, and the probe's rows.
  - The Windows run's rows, and w2m's run of the probe held to them on one vCPU and two.
  - Every existing run as held, G3's two-vCPU batch run among them.
- **Planted faults:**
  - (1) OpenEventW answering a handle for a name that does not exist: the family run's row differs from Windows'.
  - (2) SignalObjectAndWait signalling after the wait instead of before: the probe's handoff row differs, or its run
    hangs to its limit.

## done criteria

The six named-sync-object functions the game declares are served and held to Windows' own rows on one vCPU and two;
every other run is unchanged.

## self-review checklist

- [x] Scope names one verifiable property: six functions served, held to Windows' rows.
- [x] context_files cite the class, the services, a probe's form and the family judge.
- [x] Emits: none, with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the probe's rows on Windows and w2m; numbered plants.
- [x] One concept: the named sync objects.

## result

GATES-LINE
