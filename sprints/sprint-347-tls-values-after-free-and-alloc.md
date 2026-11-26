# Sprint 347 — Another thread's TLS value after TlsFree and TlsAlloc, as Windows gives it

```yaml
---
id: 347
status: active
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-4's fifteenth review, Low 1, recorded as the Decision "TlsFree's reach settled by a Windows row". It found
that w2m's TlsFree clears the freed index in the calling thread's TEB alone (`tools/guest_runtime/thread_storage.c`,
`k32_TlsFree`). TlsAlloc does the same: it clears only the caller's slot (`k32_TlsAlloc`, `*slot = 0`). Microsoft
Learn documents TlsAlloc as "When a TLS index is allocated, its storage slots are initialized to NULL" (TlsAlloc,
Remarks), and the return-value section reads "The slots for the index are initialized to zero". The TlsFree page says
nothing of what a free does to any thread's value. So a thread that set an index, which another thread then freed and
allocated again, may read its old value in w2m where Windows reads 0.

- **The probe.** `probes/tls_free_reach/` is committed with this card and compiles against the mingw-w64 headers. A
  worker sets its value at an index, and so does the main thread. The main thread frees the index, and the worker
  reads it, along with the last error. The main thread allocates again and notes whether it got the same index. The
  worker reads again, and the main thread reads its own slot. Each step waits on an event, so no row depends on
  scheduling. The pass runs once at a TEB slot (below 64) and once at an expansion slot (64 and up).
- **Its Windows rows.** Run it on the Windows runner by `.github/workflows/windows-probe.yml`, three runs. Keep the
  binary and the runs' files and add the walk fixture's entry, as card 345 did. The rows must read alike in the three
  runs; if they do not, stop and say so. Before any change, run the probe through `w2m-vm` on one vCPU and quote where
  its rows differ from Windows'.
- **The change, as the rows require.** Make TlsAlloc and TlsFree give every thread's slot what Windows gives it,
  zeroing an index in every live thread's TEB, or its expansion array, where Windows reads 0:
  - do it under the slots' lock (Sprint 345), never holding it across `w2m_unimplemented` or `w2m_logged`;
  - find how the runtime C reaches every live thread's TEB (`tools/guest_runtime/threads.c`, the run table) and quote
    it;
  - a slot another thread reads at the same moment with TlsGetValue, which takes no lock, is that application's race,
    as it is on Windows. State that with the lines.

  If the rows show Windows leaving the worker's value, change nothing for that call and record the row as the
  contract.
- **The judge.** A `run_loop.rs` test runs the probe through `w2m-vm` on one vCPU and on two, and holds every row to
  Windows'.
- **Not here:**
  - FLS, whose FlsFree already clears every thread's block;
  - any other lock class.

## prerequisites

- 346 (closed)

## context_files

- `probes/tls_free_reach/tls_free_reach.c` and `probe.json` (this card's commit).
- `tools/guest_runtime/thread_storage.c` whole (the slots' lock, Sprint 345), and `tools/guest_runtime/threads.c`'s run
  table and thread records.
- `crates/w2m_vm/tests/run_loop.rs`: card 345's test, `program_through_w2m_vm`.
- `crates/guest_runtime/tests/python_image.rs` (`DEPARTURES`, already naming `thread_storage.c`; extend its reason).
- `.github/workflows/windows-probe.yml`; card 345's result
  (`sprints/sprint-345-the-thread-slots-take-their-own-lock.md`) for the Windows run, the stored binary and the walk
  fixture's entry.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None; what two kernel32 services write.

### Invariants

- Every existing gate passes unchanged, the family runs among them (`contracts/windows.thread_storage.*`).
- The slots' lock is never held across `w2m_unimplemented` or `w2m_logged`.

## artifact contract

### Files modified

- `tools/guest_runtime/thread_storage.c`, `crates/w2m_vm/tests/run_loop.rs`, `crates/guest_runtime/tests/python_image.rs`,
  `crates/image_walk/tests/fixtures.rs` (its totals), `store/translation-cache/walk-fixtures-2026-10-30.json` (one
  entry), `store/binaries/` (the probe's binary), `store/probe-runs/<run id>/`.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The Windows run's id, its runner and CPU, and its three runs' rows.
  - The probe's rows through `w2m-vm` before the change, each difference named.
  - After the change, the rows on one vCPU and on two, equal to Windows'.
  - The lines that reach every live thread's TEB.
- **Planted faults:**
  - (1) TlsAlloc clearing the caller's slot alone, as before. The test fails on `worker_after_again`, if Windows
    reads 0 there.
  - (2) TlsFree's clearing limited to the caller, if the rows require more. The test fails on `worker_after_free`.

  If a plant has no row to fail because Windows leaves the value, say so and name it. Before the change, read each
  judge's reach: the test failing on the rows w2m gives today.

## done criteria

Another thread's TLS value after TlsFree and after TlsAlloc of the same index is what Windows gives, at a TEB slot and
at an expansion slot, on one vCPU and on two.

## self-review checklist

- [x] Scope names one verifiable property: another thread's TLS value after a free and a new allocation, as Windows.
- [x] context_files cite the probe, the slot C and the run table, the judge's form, the Windows run's procedure.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: Windows' rows, w2m's rows before and after; two numbered plants, their reach read.
- [x] Sweet spot: `thread_storage.c` and a test, with the stored probe; one concept, a TLS index's reach.

## result

GATES-LINE
