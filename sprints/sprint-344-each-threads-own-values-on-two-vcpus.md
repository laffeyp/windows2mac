# Sprint 344 — Each thread's own values stay its own on two vCPUs

```yaml
---
id: 344
status: active
phase: 6
pass_kind: functional
---
```

## scope

Sprint 342 took the runtime word off seven services of lock class none. Their judges show the word gone and the call
ring intact. None shows the risk the class takes: one thread reading the last error, TLS value or thread id of
another thread served on another vCPU at the same moment. Reviewer-4's thirteenth review, Medium 1, raised this.

The review's first fix is already held, with no new code:
- `run_loop::a_thread_suspended_on_another_vcpu_stops_before_suspend_returns` (Sprint 330) holds the threads
  probe's whole output on two vCPUs to its output on one;
- the family runs hold the one-vCPU output, row by row, to `contracts/windows.thread.threads/0.2.0.json`, whose
  `worker.start` (`tid_matches` 1, `last_error` 187, `tls` 0) and `main.after_worker` (`last_error` 57005, `tls` 85)
  rows are Windows'.

Those rows are a handful of calls made at different moments. This card adds the judge under load.

- **The probe:** `probes/thread_values_race/` is committed with this card, and compiles against the mingw-w64
  headers as the gates check. Threads A and B each run 200,000 rounds at once. Each round a thread:
  - sets its last error to its own base plus the round and reads it back;
  - sets one shared TLS slot to the same value and reads it back;
  - reads its own thread id.

  Each mismatch is counted. Each thread also counts the rounds in which the other's round count moved. That count
  shows how far the two loops overlapped, and it is printed in its own `overlap` row.
- **Its Windows rows:**
  - build and run it on the Windows runner, `.github/workflows/windows-probe.yml`, three runs;
  - keep the binary under `store/binaries/` by its sha256 and the runs' files under `store/probe-runs/<run id>/`, as
    card 331 kept `locked_race` (its result, lines 125-128);
  - add the binary's entry to `store/translation-cache/walk-fixtures-2026-10-30.json` as card 331 did (its result,
    lines 189-196).

  Every row but `overlap` must read alike in the three runs, with each mismatch count 0. If they do not, stop and say
  so.
- **The judge:** a test in `crates/w2m_vm/tests/run_loop.rs` runs the probe through `w2m-vm` on one vCPU and on two,
  as `a_locked_model_step_loses_no_add_of_another_vcpus_aligned_lock` runs `locked_race`:
  - each run's rows but `overlap` are held to Windows' rows;
  - on two vCPUs, both of `overlap`'s counts are held above a floor, which shows the loops ran at once.

  Measure `overlap` on one vCPU and on two before fixing the floor, and quote both. The floor must sit well above the
  one-vCPU figure and well below the two-vCPU one.
- **Not here:**
  - FlsGetValue and FlsSetValue, which stay on the word;
  - Change 1's other classes;
  - any change to a service. If the judge finds a mismatch on today's code, that is a defect for this card to report
    with its evidence, and its fix is the next card's.

## prerequisites

- 343 (closed)

## context_files

- `probes/thread_values_race/thread_values_race.c` and `probe.json` (this card's commit).
- `probes/threads/threads.c` 79-157 and `contracts/windows.thread.threads/0.2.0.json`'s `worker.start` and
  `main.after_worker` rows.
- `crates/w2m_vm/tests/run_loop.rs`: `a_thread_suspended_on_another_vcpu_stops_before_suspend_returns`,
  `a_locked_model_step_loses_no_add_of_another_vcpus_aligned_lock` and `program_through_w2m_vm`.
- `crates/w2m_vm/src/guest.rs` `bridge` (its stores to the vCPU's state block, and its class none path, Sprint 342),
  and `crates/guest_runtime/src/services.rs` `NO_LOCK`.
- `tools/guest_runtime/thread_storage.c` (TlsGetValue, TlsSetValue) and `tools/guest_runtime/process.c` (GetLastError,
  SetLastError, GetCurrentThreadId).
- `.github/workflows/windows-probe.yml`, and card 331's result (`sprints/sprint-331-every-other-vcpu-stopped-for-a-locked-model-step.md`
  125-128, 186-198) for the Windows run, the stored binary and the walk fixture.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None; a probe and a test.

### Invariants

- No service, bridge or runtime C changes, and every existing gate passes unchanged.

## artifact contract

### Files modified

- `crates/w2m_vm/tests/run_loop.rs`, `store/binaries/` (the probe's binary), `store/probe-runs/<run id>/`, and
  `store/translation-cache/walk-fixtures-2026-10-30.json` (one entry, and the test's doc comment's totals).

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The Windows run's id, its runner and CPU, and its three runs' rows quoted.
  - The test's rows on one vCPU and on two quoted, every mismatch 0.
  - `overlap` on one vCPU and on two quoted, and the floor's grounds.
- **Planted faults:**
  - (1) One none-class store of the calling vCPU's own state pointed at vCPU 0's page. Find that store in the bridge,
    or in the service its proof (card 339) names, and quote it. The test fails on two vCPUs on a mismatch count above
    0.
  - (2) SetLastError writes one process-wide word, not the TEB, and GetLastError reads it. The test fails on two vCPUs
    on `last_error_wrong` above 0.

  Before the work, read each judge's reach. Name the line each plant changes, and the row and count it moves.

## done criteria

The new probe's rows on one vCPU and on two equal Windows', with no thread reading another's last error, TLS value or
thread id over 200,000 overlapped rounds each, and both plants make the test fail by a mismatch.

## self-review checklist

- [x] Scope names one verifiable property: no thread reads another's own values on two vCPUs under load.
- [x] context_files cite the probe, the earlier judges, the bridge and services, and the Windows run's procedure.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: Windows' rows, the test's rows, the overlap; two numbered plants, their reach read.
- [x] Sweet spot: one probe, one test, its stored inputs.

## result

GATES-LINE
