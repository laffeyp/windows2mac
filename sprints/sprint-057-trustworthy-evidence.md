# Sprint 057 — trustworthy evidence

```yaml
---
id: 057
status: closed
phase: 5
pass_kind: functional
---
```

## scope

A family's acceptance goes stale when its candidate's sha256 changes (`tests/test_family_runs.py`). The candidate is
the family's guest file plus `host_files`, a list kept by hand: Sprint 056 found file_search and display missing
heap.c and guest_stubs.py, so an edit to heap.c left file_search accepted on code it had not run. Sprint 055 found a
plant that changed the parent's runtime while the child process ran the unplanted one, and the plant run reported
nothing (KIT_DIARY #46).

1. **The runtime files, derived.** The runtime's object files (the ones `guest_runtime.build` links) give each file's
   defined and undefined external symbols. A family's runtime files are the closure, over those references, of its
   guest file and of the files that define the services its probe binaries import (`pe.parse(...).imports` and
   delay imports, through `guest_runtime.service_table`). Each object's included headers (clang `-MD`) join it.
2. **The host files, derived where the C says.** Each host request a reached file makes (`w2m_host(W2M_HOST_*` and
   `W2M_FILE_*`) maps to the Python that answers it, in one table beside the request numbers; a gate refuses a request
   the C makes that the table lacks. `host_files` keeps only Python that no request reaches (the loader's, for one),
   and a gate refuses a `.c` file there.
3. **Each process checked.** An item run or a plant run fails when a process it ran (the main one or a child the
   Machine started) holds a runtime image other than the one built from the candidate's sources.

## prerequisites

- 056 (closed: CI run 38183215347 on 79f13466; main 6507b04e)

## context_files

- `scripts/family_run.py` (`candidate_sha256`, `run_in_vm`, the plant loop), `src/w2m/guest_runtime.py`,
  `src/w2m/kernel.py` (`spawner`, `Machine.processes`), `src/w2m/vm_backend.py` (the HOST_* numbers),
  `tests/test_family_runs.py`; KIT_DIARY #46

## signal contract

### Emits

- Per family whose candidate changes: the item run. A family's rows do not change; its candidate sha256 does.

### Invariants

- Vocabulary 0.7; no new tags.

## artifact contract

- modified: `src/w2m/guest_runtime.py` (object symbols and headers), `scripts/family_run.py` (derived candidate,
  process check), `src/w2m/vm_backend.py` (the request table), `tests/test_family_runs.py`
- created: `tests/test_family_dependencies.py`
- added (found here): a gate that every plant's text is in its file once (`tests/test_family_runs.py`); six plants re-anchored
- `store/runs/family-runs.json`: every family whose candidate changed, run again
- gates 0 and CI `success` on the closing commit

## observation contract

- **An edit to heap.c makes file_search stale**: a test changes heap.c's text in `sources` and finds file_search's
  candidate sha256 changed.
- **A plant on a child process not built from the planted source fails the run**: with the spawner made to drop
  `runtime_sources` (055's defect), time_and_error_mode's plant run fails, naming the child process.
- **Every family accepted** on its derived candidate.

## done criteria

The two observations above hold as tests; all 56 families accepted on derived candidates; `host_files` holds no `.c`.

## observed

- **The candidate is derived.** `guest_runtime.objects` reads each runtime file's object (the ones the image links):
  external symbols defined and used, file-local ones, and the headers clang `-MD` lists. A family reaches its guest
  file, the files defining the services its probe binaries import (`vm_backend.service_for`, factored out of
  `_bridge`), the files defining the symbols the host names (`w2m_image_entry`, `w2m_cpuid`, `w2m_exception`,
  `w2m_threads_blocked`), and every file defining a symbol a reached file uses. Families reach 16 to 27 of the 44 files:
  runtime.c's entry and exception paths pull in a core of 16 for every family. `w2m_threads_blocked` is a common
  symbol (N_UNDF with a size), which the reader counts as a definition.
- **The Python is derived from the requests.** `vm_backend.REQUEST_FILES` names the Python answering each of the 33
  W2M_HOST_*/W2M_FILE_* requests; a gate refuses a request the C makes that the table lacks. Every family whose files
  make a request reaches vm_backend.py, so an edit there makes every family stale; a full rerun of 56 item runs and
  their plants took 4 min 7 s on this Mac.
- **`host_files` shrank to two entries** (loader.py for loader and static_tls). Every runtime file a family had listed
  by hand is in its derived set; the derivation added files no hand list held (heap.c and guest_calls.c for registry,
  among others). A gate refuses a `.c` or a derived file there.
- **An edit to heap.c makes file_search stale** (`test_an_edit_to_heap_c_makes_file_search_stale`); an edit to a file
  a family does not reach leaves it fresh (windows.c and registry).
- **A plant on a child not built from the planted source fails.** `run_in_vm` compares each process's runtime image
  with the one built from the candidate's sources and replaces the stop when one differs. With the spawner made to drop
  `runtime_sources` (055's defect), the `error_mode_not_inherited` plant run fails naming process 4100; with the check
  removed, the same run reports "as named": the silent pass of KIT_DIARY #46.
- **Six plants had rotted.** The sweep of every family's plants found six whose text had left its file when the code
  changed after the plant was written (slim_locks sleep_leaves_error, system_info version_build_off, sync_objects auto_event_stays_set and
  create_leaves_error, thread_stack limits_swapped, thread_handles thread_signals). Only a local plant run finds that;
  no gate did. Each was re-anchored on the code that replaced its text, planting the same fault, and is caught as named;
  `test_every_plant_finds_its_text_once` now refuses a lost anchor (KIT_DIARY #48).
- **All 56 families accepted** on derived candidates; every plant caught as named.
- **Left to Sprint 058:** `vm_backend._RUNTIME_SYMBOLS` is one table for the whole host, rewritten by each VM built;
  a parent's code compiled after its child was built takes `w2m_cpuid`'s address from the child's image. The images
  are equal in every run the check above passes, so no run is wrong today.
- **Closed on CI run 38188148154** (success on f7b474c7).
