# Sprint 352 — A handle's lookup holds a reference on its object, and every blocked wait keeps its objects

```yaml
---
id: 352
status: active
phase: 6
pass_kind: functional
---
```

## scope

Card 2 of `docs/design-handles-and-sync-objects-2026-11-26.md` ("The cards that follow", lines 344-353; the rule at
166-231). Sprint 350 built the two counts, `handles` and `refs`, in `struct object`. It gave a reference to the waiter
entries of a semaphore, an auto-reset event, a mutex and a wait for all.

This card makes the rule whole, still under the runtime word:
- **The lookup.** `object_of` takes a reference, and every caller drops it when done with the object. There are 13
  calls in `tools/guest_runtime/sync_objects.c`. Its users in `file.c` (264, 345-352) and `processes.c` go through
  `w2m_object_*` and drop likewise. The design names this card as one concept over three files.
- **Every blocked wait.**
  - A wait of any kind that blocks keeps its objects' references in its entry: a manual-reset event, a thread, a
    process, and a wait for any of several among them.
  - Its retry, `WaitForMultipleObjects` included, reads its objects from the entry, not by its handles again.
  - So a handle closed during any wait leaves the wait on its object, and the entry is FREE only at `refs` 0.
- **WaitForMultipleObjects on a closed handle.** It stops the run today (`sync_objects.c` 461: `if (!os[i])
  w2m_unimplemented(...)`). Microsoft Learn's WaitForMultipleObjects documents WAIT_FAILED with the last error for an
  invalid handle; quote the page. Answer WAIT_FAILED and ERROR_INVALID_HANDLE, as `wait_one` does at 413-418, for a
  handle that names no object when the call begins. Hold the answer to a Windows row.
- **The Windows rows.** A probe committed by the Supervisor as a "Sprint 352 card amendment:" commit (Sprint 349's
  kind). Write it under `probes/` and send its path.
  - **Defined, compared.** WaitForMultipleObjects of a closed handle beside a valid one, its return and last error.
  - **Recorded, not compared.** A worker waits on a manual-reset event. The main thread duplicates its handle, closes
    the original, sets the event through the duplicate, and records the worker's return and last error (the design's
    row for card 2; Microsoft Learn leaves a wait on a handle closed during it undefined).
  - A wait for any of a closed event and an open one, the open one then set, its index returned; recorded, not
    compared.
- **The judge.** A `run_loop.rs` test runs the probe on one vCPU and on two, holding the defined rows to Windows' and
  the recorded ones to their documented range. It must fail on today's code: the stop at 461, or a wait that does not
  end.
- **The family runs on two vCPUs (Sprint 351) and the batch run pass unchanged.**
- **Not here:** the handle table's lock, the sync objects' lock, the woken mark.

## prerequisites

- 351 (closed)

## context_files

- `docs/design-handles-and-sync-objects-2026-11-26.md` 135-365 (the design and the cards).
- `tools/guest_runtime/sync_objects.c` whole; `tools/guest_runtime/file.c` 255-270 and 340-360; the `w2m_object_*`
  calls in `tools/guest_runtime/processes.c` (`grep -n 'w2m_object_' tools/guest_runtime/processes.c`).
- `probes/wait_closed_handle/` and its test in `crates/w2m_vm/tests/run_loop.rs`
  (`a_semaphore_made_after_a_waited_handle_is_closed_keeps_its_unit_on_one_and_two_vcpus`) for the probe's and the
  judge's form.
- `crates/guest_runtime/tests/python_image.rs` (`DEPARTURES`, naming `sync_objects.c`; extend its reason, and name
  `file.c` and `processes.c` if they change).
- `.github/workflows/windows-probe.yml`; card 350's result for the Windows run, the stored binary and the walk
  fixture's entry.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None; when an object's entry is freed, and what a wait reads.

### Invariants

- The one-vCPU and two-vCPU family runs, the batch run and Sprint 350's tests pass unchanged.
- No lock is added; every change runs under the runtime word.

## artifact contract

### Files modified

- `tools/guest_runtime/sync_objects.c`, `file.c` and `processes.c` as the drops need, `crates/w2m_vm/tests/run_loop.rs`,
  `crates/guest_runtime/tests/python_image.rs`, the probe's stored binary, runs and walk fixture entry, and any held
  line the gates name.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - Each `object_of` call and the line that drops its reference.
  - Each wait's return paths and what each drops.
  - Microsoft Learn's WaitForMultipleObjects text quoted.
  - The Windows run's rows.
  - The test's rows on one vCPU and two, before the change and after.
- **Planted faults:**
  - (1) One caller's drop removed, a reference left. A test shows the entry never freed: a create after enough
    open-and-close cycles fails, or the probe's rows differ.
  - (2) The retry of WaitForMultipleObjects reading its handles again, not its entry. The probe's test fails.
  - (3) The stop at 461 restored. The probe's test fails on the stop.

  Before the change, read each judge's reach.

## done criteria

Every lookup of a sync object holds a reference until its caller is done, every blocked wait keeps its objects until
it ends, a handle closed during any wait leaves the wait on its object, and WaitForMultipleObjects of a closed handle
answers as Windows does instead of stopping the run.

## self-review checklist

- [x] Scope names one verifiable property: the lookup's reference and every wait's, as the design's card 2.
- [x] context_files cite the design, the runtime C, the probe and judge form, and the Windows run's procedure.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the drops, the paths, the rows; three numbered plants, their reach read.
- [x] Sweet spot: the design names it one concept over three files, as here.

## result

GATES-LINE
