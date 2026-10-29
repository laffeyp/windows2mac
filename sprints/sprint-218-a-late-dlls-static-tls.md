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
