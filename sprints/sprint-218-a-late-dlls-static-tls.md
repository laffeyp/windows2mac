# Sprint 218 — A late DLL's static TLS

```yaml
---
id: 218
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence after Sprint 214" as annotated in Sprint 217 (reviewer-3's Highs 1 and 2 of its review of
Sprints 215-217), and the Deferred entry on the helpers' errors, due since Sprint 215.

- **Where the run stops:** the batch run stops at row 6,698 of 10,505, inside UnityPlayer's LoadLibraryW of
  steam_api64.dll. The DLL has static TLS, and `modules::load_dll` refuses a DLL loaded after the start that has
  static TLS. The refusal comes back as a string, so `host_request` maps it to `HostError::Failed`, and the record
  reads `faulted` for a request the host does not serve.
- **Typed errors:** `modules` returns a typed error. `module`, `proc` and `load_dll` tell a refusal (not served yet, a
  Windows DLL w2m does not model) from a failure (a file unreadable, a PE that does not parse). `host_request` maps the
  refusal to `NotServed` and the failure to `Failed`, so a refusal ends the run as unserved or killed, not faulted.
- **The judge first:** the family `windows.loader.late_tls` (contract 0.1.0, accepted; runner runs 38229484497,
  38229488362, 38229493876 and 38229497641, probes `probes/late_tls`, binaries 960385e1… and 8b7a25e1… in
  `store/binaries`) is ported to `family_runs.rs` and the gates' family loop. Each binary runs with the
  `late_tls_dll.dll` it had beside it on the runner (`scripts/family_run.py` 1295-1298: 960385e1… with 3afacd2a…,
  8b7a25e1… with f32fa39b…, both stored), as two rows of `store/family_runs/beside-2026-10-28.tsv`. Its rows are
  judged before the loader changes.
- **The change:** static TLS for a DLL loaded after the start, as the contract's rows say. The DLL takes the next
  index, written at its AddressOfIndex. Every thread not ended, the loading one and those started before the load,
  gets its own block, copied from the template with its zero fill, in a pointer array that does not move. The frozen
  host's `_late_static_tls` (vm_backend 3953-3978) is followed where it agrees with the rows; a departure is named
  with its row.
- **The stream:** the batch run goes past row 6,698. Its next stop is held, the census rows updated, and the record
  reads the new stop's kind.

## prerequisites

- 217 (closed)

## context_files

- `crates/w2m_vm/src/modules.rs` (`module`, `load_dll`, `proc`), `crates/w2m_vm/src/host_request.rs` (HOST_MODULE,
  HOST_PROC), `crates/w2m_vm/src/threads.rs` (each thread's TEB), `crates/w2m_vm/tests/family_runs.rs`
- `contracts/windows.loader.late_tls/0.1.0.json`, `probes/late_tls/late_tls.c`; `src/w2m/vm_backend.py` 3030-3060
  (`_thread_static_tls`), 3906-3978 (`_load_dll`, `_late_static_tls`)
- BLACKBOARD Deferred (the helpers' errors), `store/batch_run/first-stop-2026-10-28.tsv`

## signal contract

### Emits

- None new: the batch run's record carries the new stop. A refusal from `modules` ends the run as unserved or with
  SERVICE_UNIMPLEMENTED, not faulted.
- *(Annotated in the work, reviewer-3's Low 3 of its review of Sprint 218: "None new" is not an Emits for code that
  runs, WORKING_AGREEMENT 294. The batch run's record writes EXECUTION_COMPLETED with `exit_status` "killed",
  `guest_trace` false and `unobserved` naming the call stream's 10,597 rows and the stop "StepLimit: … steps at pc …";
  then COMPARISON_COMPLETED with `verdict` "conforms" and no `first_divergence`; and SESSION_COMPLETED with `outcome`
  "completed". It writes no SERVICE_UNIMPLEMENTED, since no call is in flight at the stop.)*

### Invariants

- The stream's rows 0-6,697 are unchanged.
- The four module families judged today still conform.

## artifact contract

### Files modified

- `crates/w2m_vm/` (`src/modules.rs`, `src/host_request.rs`, `src/threads.rs` if a thread's array is read there,
  `tests/family_runs.rs`, `tests/batch_run.rs`), `scripts/gates.sh` (the family loop),
  `store/batch_run/first-stop-2026-10-28.tsv`, `store/family_runs/module-stops-2026-10-28.tsv`,
  `store/family_runs/beside-2026-10-28.tsv`,
  `docs/step-4-census-2026-10-28.md`

### Command exit codes

- `cargo nextest run -p w2m_vm`, the batch run under substrate with `--strict`, `check_records` and
  `scripts/gates.sh` return 0

## observation contract

- **Expected values:** every row of `windows.loader.late_tls` on both probe binaries; the stream past row 6,698, equal
  to the reference row for row up to its next stop; a refusal's record reads killed or unserved.
- **Planted faults:** (1) the index one too high: the family's row names it; (2) a thread started before the load left
  without its block: the family's row names it; (3) a refusal mapped to `Failed`: the batch record's test names the
  exit status `faulted`.

## done criteria

The batch run loads steam_api64.dll with its static TLS as Windows' rows say, runs past row 6,698, and its record
names its next stop by its true kind.

## self-review checklist

- [x] Scope names one verifiable property: a late DLL's static TLS served as Windows serves it.
- [x] context_files cite the contract, the probe, the frozen host's lines and the code.
- [x] Emits: none new, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the stop at steam_api64.dll, with its judge and its error type.

## result

- **The judge first:** `windows.loader.late_tls` is the fifth family in `family_runs.rs` and the gates' loop, which
  now takes each family's contract id. Its two binaries run with the `late_tls_dll.dll` each had beside it (two rows
  of `beside-2026-10-28.tsv`). Before the change, both stopped at "LoadLibrary late_tls_dll.dll: static TLS in a DLL
  loaded after the start, not served by the Rust host yet", the game's stop.
- **Typed errors:** `modules::ModuleError` is `NotServed` or `Failed`, and `From<ModuleError> for HostError` maps
  each to its own kind. The refusals are: a path the bridge refuses, an API set's unobserved host, ntdll not modelled,
  full system slots, a Windows DLL w2m does not model, TLS callbacks in a late DLL, an import a loaded DLL does not
  export, and GetProcAddress on a module w2m did not hand out. A file unreadable, a PE that does not parse, and a
  mapping or guest error are failures. The module unit tests check each refusal's kind.
- **The change:** `threads::late_static_tls` ports `_late_static_tls` (vm_backend 3953-3978): the next index at
  AddressOfIndex, and a block of the template and its zero fill in the slot of every thread not ended. It runs from
  `load_dll` after the imports and before the snapshot, as vm_backend 3939 does. Both late_tls binaries print all
  seven contract rows and end at ExitProcess 0: 14 of 14 module-family runs conform.
- **The stream:** the batch run equals all 10,505 reference rows and runs 92 past them, 10,597 calls, none in
  flight, none dropped. It stops at the VM's step budget, `Exit("max_steps: 1099511627829 of 1099511627776 at pc
  0x7f0001435e30")`, after 486.7 s; from the review's Low 1 the stop file writes it "StepLimit: 1099511627829 of
  1099511627776 steps at pc 0x7f0001435e30". The comparison against Windows over the 10,597 calls: conforms, 0 defects, 0
  past the cut, 0 scheduling, 24 values Windows varies. The held stop file and census rows 5a, 6 and 7 are updated.
- **Plant 1** (the index one too high): the probe 960385e1… ended with an access violation, and `family_runs.rs`
  named it: "stopped at \"ExitProcess { code: 3221225477 }\", which store/family_runs/module-stops-2026-10-28.tsv
  does not hold". Restored byte for byte.
- **Plant 2** (blocks only for the first thread): the same stop, named the same way, since the worker started before
  the load read an empty slot. Restored byte for byte.
- **Plant 3** (a module refusal mapped to `Failed`): the unit test `a_module_refusal_is_not_served_and_a_failure_is_a_fault`
  failed, "assertion failed: matches!(refused, HostError::NotServed(m) if m.contains(\"does not model\"))". Restored
  byte for byte.
- **Departures:** (1) HOST_FILETIME (request 14) is served, the wall clock as a FILETIME, as vm_backend 3843-3844
  serves it and GetSystemTimeAsFileTime answers; the late_tls DLL's C runtime asks it as it starts, and the probes
  stopped there once the TLS was served. (2) A `max_steps` stop is the harness's own limit: `StopKind::StepLimit` and
  `RunEnd::StepLimit` record it killed, as `run_frontier.py` 71 and 405 class it (`HARNESS_STOPS`); it had read
  faulted. `run.rs`, `record.rs` and `batch_record.rs` change for it, outside the card's file list. (3) `batch_run.rs`
  holds every frozen-host row when the Rust stream runs past them; the rows past them have only the comparison against
  Windows as their judge. (4) Plants 1 and 2 were caught as a stop the judge does not hold, not as a differing row:
  the probe faults before it prints the rows. (5) Plant 3's judge is a unit test, not `batch_record.rs`, since the game
  no longer stops at a refusal. (6) The frozen harness stopped its run at 34.9 s, when "the log holds the reference's
  168 lines and the main thread its calls" (`run_frontier.py` 306); the Rust harness has no such stop and runs to the
  step budget, 486.7 s, which the gates' batch run now takes each time. (7) The game's own case lies outside the
  contract's rows: the rows hold a program without TLS that loads one TLS DLL, index 1; in the game, UnityPlayer.dll
  holds index 1 from the start (`llvm-objdump -p`: TLS directories in UnityPlayer.dll at 0x16836c0 and steam_api64.dll
  at 0x30b80, none in the exe), so steam_api64.dll takes index 2 by the frozen host's rule (vm_backend 3959), which no
  row shows. The DLL reads its index back from AddressOfIndex, so the value itself is not what the game depends on.
  The done criteria's "as Windows' rows say" claims more than the rows hold; an Open question names the row that
  would settle it (reviewer-3's Medium 2 of its review of Sprint 218).
- **After reviewer-3's review of Sprint 218** (the gates run then going was stopped with no pass line, and run again):
  - **Medium 1:** "ended" is read from the run table's state (`threads::ended`, `E_STATUS == ENDED`), as `suspend`
    reads it and as the frozen host's `t.status == "ended"` does, not from the exit code. A unit test holds a thread
    ended with code 259, and one running after THREAD_END set its code. Plant 2 was set again on the new filter and
    caught the same way.
  - **Medium 3:** `From<String> for ModuleError` is gone, so each site names its kind. `late_static_tls` returns
    `ModuleError`: a full array and the arena's refusals are `NotServed`, as the frozen host's Unimplemented
    (3961, 3971, 3976). `place_image`'s refusal and the full loader area are `NotServed`; I/O, parsing, mapping and
    guest errors are `Failed`. The unit test `a_late_dll_past_the_tls_array_is_refused` holds the full array's kind.
  - **Low 1:** `Stop::TimeLimit` and `Stop::StepLimit { count, max, pc }` replace the two `Exit` texts read back.
    The stop file's text for the step budget is "StepLimit: … steps at pc …", and the held stop file follows.
  - **Low 3:** the Emits annotation above.
