# Sprint 344 — Each thread's own values stay its own on two vCPUs

```yaml
---
id: 344
status: done
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

- **The Windows run:** run 45760720489 (`windows-2025`, Server 2025 10.0.26100.33828, an AMD EPYC 9V74),
  `probes/thread_values_race` as committed c3bbae356, three runs, each exit 0 in 0.019 to 0.057 s; its files in
  `store/probe-runs/45760720489/` and its binary in `store/binaries/c9b9847f83df….exe`, the run's `probe.sha256`. The
  three alike but `overlap`: `{"s":"slot","allocated":1}`, `{"s":"wait","ret":0}`,
  `{"s":"a","rounds":200000,"last_error_wrong":0,"tls_wrong":0,"tid_wrong":0}` and the same for `b`; `overlap`
  53,253 and 53,407, 52,972 and 52,525, 54,147 and 54,315.
- **Through `w2m-vm`, before the test** (`target/release/w2m-vm` on that binary, 21 runs on one vCPU and 21 on two,
  each under 2 s): every run `ExitProcess { code: 0 }`, every row but `overlap` Windows'. `overlap` on one vCPU 260 and
  260 in all 21, the loops meeting only at the time slices' switches; on two, 155,991 to 170,312 (A) and 158,350 to
  170,509 (B).
- **The floor,** 10,000: 38 times the one-vCPU figure, and under a fifteenth of the lowest two-vCPU figure, 155,991.
  The gates run tests side by side on this 18-core M5 Max; the margin lets the two loops overlap fifteen times less
  than measured before the judge fails.
- **The test** `run_loop::each_threads_last_error_tls_value_and_id_stay_its_own_on_two_vcpus`: the Windows run's three
  bodies alike but `overlap`; the probe through `w2m-vm` on two vCPUs, then on one, each `ExitProcess { code: 0 }` and
  its rows but `overlap` equal to `body-1.jsonl`'s; on two, both `overlap` counts above 10,000. Run alone it passed in
  1.75 s: on one vCPU every mismatch 0 and `overlap` 260 and 260, on two every mismatch 0 and 166,940 and 167,472. No
  mismatch on today's code, so no defect to report.
- **The walk fixture:** `target/release/image_walk` on the binary, 551 instructions, all walked, 137 block starts, no
  table, none undecodable; put in by `jq` in the file's order, the `.exe` keys sorted, then the `.dll`, that order
  first checked to reproduce the stored file byte for byte (`cmp`), the diff the one entry. The test's doc comment
  (`crates/image_walk/tests/fixtures.rs`) made 282 images and 641,894 instructions, summed by `jq`; 116 tables, as
  before.
- **Plant 1:** "thread_values_race's rows but overlap on 2 vCPUs as on Windows", left
  `{"s":"a","rounds":200000,"last_error_wrong":199999,"tls_wrong":199999,"tid_wrong":200000}` and
  `{"s":"b","rounds":200000,"last_error_wrong":14935,"tls_wrong":13757,"tid_wrong":20420}`. The store: RAX's, after
  the service, into the calling vCPU's state block, `guest.rs` 616-618, `mov_imm(16, state)` and
  `a.emit(&[a64::str(8, 0, 16, 0)])`, `state` being `PROCESS_VA + STATE_OFF`. GetLastError, TlsGetValue, TlsSetValue
  and GetCurrentThreadId each set RAX (`runtime.c` 97-110, `thread_storage.c` 89-110; only SetLastError is
  `W2M_VOID`). No address reaches vCPU 0's state block from another vCPU (`VcpuPages::private`, `lib.rs` 103-111;
  `Guest::private_chain` maps `PROCESS_VA + STATE_OFF` to the vCPU's own page), so the plant mapped one page,
  `PROCESS_VA + 0x100_0000`, to it in `Layout::new` (memory `MONITOR_SIZE + STATE_OFF`), and sent RAX's store of a
  bridge of class none, untraced, there (`mov_imm(17, …)`, `str x0, [x17]`). Both edits were written from a copy of
  `guest.rs`, restored by `cp` and checked by `cmp`. A, on the other vCPU, read RAX as the bridge saved it at entry
  in nearly every round. B, on vCPU 0, read 13,757 to 20,420 rounds wrong: the other vCPU's stores landed in its RAX
  slot between its own store and its reload. Run first with the test's one-vCPU run ahead, the plant left one vCPU's
  rows Windows' (`overlap` 260 and 260) and failed on two with the same message, A's counts 199,999, 199,999 and
  200,000 and B's 14,335, 14,385 and 20,802.
- **Plant 2:** "thread_values_race's rows but overlap on 2 vCPUs as on Windows", left
  `{"s":"a","rounds":200000,"last_error_wrong":53567,"tls_wrong":0,"tid_wrong":0}` and
  `{"s":"b","rounds":200000,"last_error_wrong":48275,"tls_wrong":0,"tid_wrong":0}`. The lines: `runtime.c` 103,
  `w2m_set_last_error((u32)code);` (the TEB's +0x68 through the state block's `gs_base`, `last_error.c` 8,
  `runtime.h` 39), and 109, `w2m_get_last_error()`. Written from a copy, a static `process_last_error` stored at 103
  and returned at 109, restored by `cp`, checked by `cmp`. The test first ran one vCPU ahead, and the plant failed
  there: "thread_values_race's rows but overlap on 1 vCPUs as on Windows", A's `last_error_wrong` 43. A time slice
  ended between A's set and its read, and B's set landed in between. The card names the two-vCPU run as the judge,
  so the test now holds that run first and the one-vCPU run after it. The quote above is from that order.
- **Files:** `crates/w2m_vm/tests/run_loop.rs` (the test), `crates/image_walk/tests/fixtures.rs` (its doc comment's
  totals), `store/translation-cache/walk-fixtures-2026-10-30.json` (one entry), `store/binaries/c9b9847f83df….exe`
  and `store/probe-runs/45760720489/`. The card's list does not name `fixtures.rs`; card 331's grade found the same.
  No service, bridge or runtime C changed.
- **Slips:** the first release build ran in a shell without `~/.cargo/bin` on its PATH and exited 127, "command not
  found: cargo"; it was rerun with it. The test's doc comment first said "20 runs each way" while quoting the 21st
  run's 170,312 and 170,509; it was corrected before any gate ran.

- **The gates:** the selection "Rust tests: all (an input outside one crate changed)"; nextest "378 tests run: 378 passed (2 slow), 1 skipped"; the timing stage "passed, 1034 ns an exit at load 2.18"; "gates: all passed". `exit 0 in 569 s	default	2026-11-26T05:36:32Z	sprint 344	c3bbae3567d4f7c4b59d68743042b0dec9f36aba	b31caec1adfc6911df3bcf909262a1b97398baacadde7bc4567b6148eb72ce82	root=bcb2e301a956e96e438fe2837764cb1e79c12a12	rust=all python=0 samples=0	base=2f44ffb54542bc696877e96bf19707a1b8b1da96`
