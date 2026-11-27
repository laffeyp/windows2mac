# Sprint 369 — Named sync objects the game declares: CreateMutexW, OpenMutexW, OpenEventW, OpenEventA, OpenSemaphoreW, SignalObjectAndWait

```yaml
---
id: 369
status: done
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

- **Windows' rows.** `probes/named_sync_objects` (card amendment 7adce5f00), its six Learn pages quoted with their URLs
  in the source's header and its rows first written from them in `expected.jsonl`, null where a page is silent. Windows
  run 46053401854 (gha-windows-2025-x64-10.0.26100.33828-Server, AMD EPYC 9V45 96-Core Processor, probe sha256
  446f77bc42c8f3f56c54513fff3ed245d2a42135b9683b622cca0a3372db069f), three runs byte for byte alike, kept flat in
  `store/probe-runs/46053401854/` and the binary under `store/binaries/`. Every cell a page states agreed. The run
  settled the silent cells: an open of an absent name gives NULL and 2 for every kind; a name another kind holds gives 6
  to every open, as to every make; a failed signal gives WAIT_FAILED with 298 or 288; and a closed handle, to signal or
  to wait on, gives WAIT_FAILED and 6, with the event to signal left unset.
- **w2m against them.** w2m-vm ran the runner's probe.exe 3 of 3 times on one vCPU and 3 of 3 on two, and each run printed
  `body-1.jsonl` byte for byte. In `contracts/windows.sync.named_objects/0.1.0.json`, exact_rows holds the 14 rows. The
  family run joins as the 274th, `named_sync_objects 46053401854`, imports column "-". `family_runs.rs` under
  `W2M_REQUIRE_CASE_RUNS=1` reports "274 of 274 family runs conform, 69 of 69 families in every run" and "on two vCPUs:
  274 of 274 held as on one vCPU, 274 of them conforming".
- **The services.** In `tools/guest_runtime/sync_objects.c`: CreateMutexW (named, owned or not), OpenMutexW,
  OpenSemaphoreW, OpenEventW and OpenEventA, a named CreateSemaphoreW, and SignalObjectAndWait. CreateEventW now refuses
  a mutex's or a semaphore's name. SetEvent, ReleaseSemaphore, ReleaseMutex and SignalObjectAndWait's signal share one
  state change, `signal_object`. The names live in the host: `crates/w2m_vm/src/kernel.rs` NAMED_OPEN (13), with its
  enum in `tools/guest_runtime/runtime.h`, and EVENT_OPEN answers NAME_OTHER_KIND for another kind's name. Ids 301-306 in
  `api_ids.txt`. In `services.rs`, `HANDLES_LOCK` grows to 15 (the make and the four opens) and `SYNC_LOCK` to 9
  (SignalObjectAndWait). Tests: `the_fifteen_services_of_lock_class_handles_are_the_handle_tables`,
  `the_nine_services_of_lock_class_sync_objects_are_the_waits_and_releases`, the ids 301 and 306, and kernel.rs's
  `events_mutexes_and_semaphores_share_one_namespace`.
- **Files beyond the card, the class's** (reported to main when found): `crates/w2m_vm/src/kernel.rs` and `runtime.h`
  (the shared namespace); `crates/w2m_vm/src/bin/family_known.rs`, `crates/w2m_vm/tests/family_runs.rs` and the new
  `store/family_runs/joined-2026-11-27.tsv` (a family that joins on the Rust host, after the frozen index's 273);
  `crates/guest_runtime/tests/python_image.rs` DEPARTURES; and `store/translation-cache/walk-fixtures-2026-10-30.json`
  with `crates/image_walk/tests/fixtures.rs`, 289 images and 646,177 instructions.
- **Plant 1** "named_sync_objects	46053401854	ended; row "event.absent": ansi_error reference 2, target 0
  (windows.sync.named_objects@0.1.0)	(which store/family_runs/sweep-held-2026-10-29.tsv does not hold)",
  `family_runs::every_family_run_conforms_or_is_held_by_what_it_waits_on` exit 100; live 19:14:32-19:16:01 UTC:
  - backup `scratchpad/plants/369/sync_objects.c.bak`;
  - `open_event`'s refusal narrowed by Edit to `second == W2M_NAME_OTHER_KIND`, so an absent name answered a handle;
    the grep found the plant at line 432;
  - every family run on one vCPU and two (the gates' `family_runs` step), then the judge under
    `W2M_REQUIRE_CASE_RUNS=1`;
  - restored by `cp`; `cmp` equal, and no "PLANT" left.
- **Plant 2** "named_sync_objects	46053401854	ended; row "saw.semaphore": after reference 258, target 0
  (windows.sync.named_objects@0.1.0)	(which store/family_runs/sweep-held-2026-10-29.tsv does not hold)", the same judge
  exit 100; live 19:16:01-19:17:43 UTC:
  - SignalObjectAndWait's wait moved by Edit ahead of its signal, the signal made only once the wait returned (lines
    1048, 1051, 1078);
  - the family runs, then the judge;
  - the probe's handoff row also differed, `"ready":258` against Windows' 0, and the run ended rather than hanging, its
    main thread's wait having a 5000 ms limit;
  - restored by `cp`; `cmp` equal.
  - After both plants the family runs and the judge ran again on the restored tree: 274 of 274 conform, on one vCPU and
    two.
- **Decisions** (agent-resolved; grounds on BLACKBOARD at the close):
  - (a) The names of mutexes and semaphores are the host's, in the namespace of the named events, and the state of each
    object stays in the guest's table. Another process's open of such a name stops the run by name.
  - (b) An opened handle gets every right its kind has, and a mask wider than the kind's all-access stops the run.
  - (c) The new family joins through `joined-2026-11-27.tsv`; the frozen extract and index stay untouched.
  - (d) SignalObjectAndWait looks up both handles before it signals, and its retry, which finds the thread's waiter
    entry, does not signal again.
  - (e) OpenEventA's name is widened byte to UTF-16 unit, and a byte past ASCII stops the run.
- **G3's two-vCPU batch run in these gates:** 142.85 s to the stop, `run_seconds` of
  `build/batch_run/record-2-vcpus-1795807369000000000`, against the laptop's 90 to 102 s (BLACKBOARD 182) and Sprint
  368's 175.2 to 233.4 s; its `load_at_start` "load averages 2.25 2.52 2.59; busy: mds_stores 93%; other w2m: none".
- **Gates:** `exit 0 in 612 s	default	2026-11-27T19:31:07Z	sprint 369	7adce5f0079baea8017280eb50e7b2abf6d9198e	cff3aa5aea77fac591749e845c72517ce16dd322741a8a98fb70e9477b934617	root=31c33b26e229110e6cf31787e18eefde46462824	rust=all python=0 samples=0	base=52a3ad5466c7adf65b2a0c43281dee7566202974`
