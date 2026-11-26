# Sprint 342 — Seven calls take no runtime word

```yaml
---
id: 342
status: done
phase: 6
pass_kind: functional
---
```

## scope

This card picks up Change 1's None class (`docs/design-rust-host-2026-10-24.md` 95-140), after Sprint 339's proof and
Sprint 341's ring. The Decision "Sprint 339's None class stopped at its proof, and the order after it" (2026-11-25)
names this card. Sprint 339 proved seven calls touch only the calling thread's own state, the shared call ring apart:

- GetCurrentThreadId and GetCurrentProcessId;
- GetLastError and SetLastError;
- TlsGetValue and TlsSetValue;
- QueryPerformanceCounter, in clock mode 0, reading `CNTVCT_EL0`.

Sprint 341 made the ring safe for concurrent callers. This card has those seven take no runtime word.

- **The work:**
  - **The lock class.** Each service in the runtime's service table (`crates/guest_runtime/src/services.rs`,
    `Service`) carries a lock class, with two values: the runtime word, which every service takes today, and none.
    Only the seven above are none.
  - **The bridge.** For a none-class service, `guest::bridge` (`crates/w2m_vm/src/guest.rs` 545-603) emits no
    `take_runtime` and no `release_runtime`, and its path is otherwise unchanged. A traced call, through `w2m_traced`,
    still takes the word.
  - **QueryPerformanceCounter's clock mode.** It is none only when the run's clock mode is 0. Clock mode 1 writes the
    transport page through `w2m_host`, and mode 2 reads the shared warp page (`process.c` 14-44; Sprint 339's proof).
    Find where the mode is known (`W2M_CLOCK_MODE_OFF`, set by the host at load), and decide the form on the grounds,
    with the lines read:
    - the bridge chooses at build time from the run's start options; or
    - QPC stays on the word unless the mode is known 0 when the bridges are built.
  - **FlsGetValue and FlsSetValue stay on the word.** They move to the TLS and FLS slots' lock with Change 1's next
    class.
  - **The proof re-read.** Sprint 339's result holds the proof for these seven. Re-read each one's C, and the bridge's
    path between take and release, against the tree as it stands after Sprint 341. Quote any line that changed, so the
    proof is of the code that ships.
- **The measurement.** `run_measurements`' `--runtime-word` run of `qpc` takes the word at the `bridge` site. Before,
  from `store/runs/w2m-vm-measurements-1795656377000000000`: `qpc`'s bridge hold over "bridge; 2 takings". After, the
  `bridge` takings fall by the QueryPerformanceCounter calls `qpc` makes in clock mode 0. Make a new record, replay it
  at level 2 and run `check_records` over it. `tests/measurements_record.rs` keeps passing on it.
- **A guest-level judge of the ring.** Sprint 341's Deferred names this card as the one that lets a guest's own calls
  judge the ring. A two-vCPU run of a guest whose threads call none-class services, such as `twoready` or the threads
  probe (read which calls each makes), is held to two things: its ring's `count` equals its `per_api` total, and the
  exception area is unchanged.
- **Not here:**
  - The other lock classes.
  - The trace's atomic sequence number.
  - G6.

## prerequisites

- 341 (closed)

## context_files

- `docs/design-rust-host-2026-10-24.md` 95-140, Change 1.
- Card 339's result (`sprints/sprint-339-the-thread-local-calls-take-no-runtime-word.md`), the proof.
- Card 341's result (`sprints/sprint-341-the-call-ring-safe-for-concurrent-callers.md`), the ring and its test.
- `crates/w2m_vm/src/guest.rs`: `bridge` (545-603), `service_for` (199-215), and its two callers (917, 1346).
- `crates/w2m_vm/src/monitor_code.rs`: `take_runtime`, `release_runtime`, `Site`.
- `crates/guest_runtime/src/services.rs` (`Service` and the table).
- The C of the seven: `tools/guest_runtime/runtime.c`, `process.c` (the clock, 14-60), `last_error.c` and
  `thread_storage.c`.
- `crates/w2m_vm/src/bin/run_measurements.rs`, `crates/w2m_vm/tests/measurements_record.rs` and
  `harness/w2m-vm-measurements.toml`.
- `crates/w2m_vm/tests/run_loop.rs`: the two-vCPU tests and Sprint 341's ring test.
- The Decision: `grep -n "Sprint 339's None class stopped at its proof" BLACKBOARD.md`; and the Deferred entry naming
  card 342, `grep -n "Re-visit: card 342" BLACKBOARD.md`.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands", with "A sprint closes in its work commit"
  and "no gate row is dropped".

## signal contract

### Emits

- The new measurements record: MEASUREMENT_RECORDED `runtime_word_hold_ns_p50` and `runtime_word_hold_ns_p99` for
  `qpc`, their `variant` naming the takings, and the rest of Sprint 337's record in its form.

### Invariants

- Every test passes as before: the two-vCPU tests, the batch run, the traced runs and the family runs.
- A traced call and every service outside the seven take the word as before.

## artifact contract

### Files modified

- `crates/guest_runtime/src/services.rs` (the lock class) and `crates/w2m_vm/src/guest.rs` (the bridge); tests.

### Files created

- `store/runs/w2m-vm-measurements-<ns>/`

### Command exit codes

- `scripts/gates.sh` returns 0
- `uv run substrate replay <the new record> --level 2` returns 0, and `check_records` over it prints no violation
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The seven's proof re-read and quoted.
  - `qpc`'s `bridge` takings before and after, against its QueryPerformanceCounter calls.
  - A test over the service table naming the seven none-class services.
  - A bridge test that assembles a none-class bridge and finds no `lock_runtime` in its code.
  - The guest-level ring check on two vCPUs.
- **Planted faults:**
  - (1) CreateEventW set to none. The table test fails by name.
  - (2) The bridge emitting `take_runtime` for a none-class service. The bridge test fails, and `qpc`'s takings return
    to the before count.

  Before the work, read each judge's reach.

## done criteria

The seven calls take no runtime word, and QueryPerformanceCounter only in clock mode 0. Each is proved on the code that
ships. `qpc`'s word takings fall by its QueryPerformanceCounter calls on a record, a guest's own calls on two vCPUs keep
the ring exact, and two plants fail by name.

## self-review checklist

- [x] Scope names one verifiable property: seven services take no word, on a record and under two vCPUs.
- [x] context_files cite Change 1, both proofs, the bridge, the table, the C, the measurement and the tests.
- [x] Emits: tags and values in 0.13 only.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the proof, the counts, three tests; two numbered plants.
- [x] Sweet spot: `services.rs` and `guest.rs` plus tests and a record; one concept, the None class.

## result

Done. The seven services are of lock class none (`crates/guest_runtime/src/services.rs` `Lock`, `NO_LOCK`,
`Service::lock_in`), and their bridges take no runtime word (`crates/w2m_vm/src/guest.rs` `bridge`, 601-615);
QueryPerformanceCounter is none only when the monitor's `CLOCK_MODE` word reads 0 as the bridges are built. The re-read
found one shared store Sprint 339's proof did not name, closed in `runtime.c`.

- **The proof, re-read on the tree after Sprint 341.** From Sprint 339's tree (6db5fcdfd) to 99b63cb80 only
  `runtime.c` changed among the proof's files: `w2m_logged` (now 84-94) does "`__atomic_fetch_add(&r->per_api[api], 1,
  __ATOMIC_RELAXED)`", "`u64 i = __atomic_fetch_add(&r->count, 1, __ATOMIC_RELAXED)`", "`if (i < r->capacity)`" and
  stores the record at `rec[i]`. `guest.rs`, `monitor_code.rs`, `process.c`, `last_error.c` and `thread_storage.c` are
  unchanged. The bridge's path between take and release is still `mov_imm(16, target)`, `blr(16)` and
  `budget_from_count` (`guest.rs` 608-610), registers and one load from x28's own monitor page. The rest of each
  service is Sprint 339's proof: GetCurrentThreadId and GetCurrentProcessId read the TEB (`runtime.c` 97, `process.c`
  95); SetLastError and GetLastError write and read the TEB's +0x68 (`last_error.c` 8, 10); TlsGetValue and
  TlsSetValue the TEB's +0x68 and the calling thread's own slot (`thread_storage.c` 58-65, 96, 108);
  QueryPerformanceCounter `*(u64 *)out = counter_now()` (`process.c` 55), in mode 0 `cntvct_el0` and `cntfrq_el0`
  (46).
- **A store the proof had not named.** QueryPerformanceCounter's "`if (out == 0) w2m_unimplemented(...)`"
  (`process.c` 54) and TlsSetValue's "`if (!slot) w2m_unimplemented(...)`" (`thread_storage.c` 107) reach
  `w2m_unimplemented`, whose `host_call` writes the shared transport page and makes `hvc #8`. The host serves `hvc #8`
  through `GuestLock::serve`, which asserts "a served exit's lock taken without the runtime word" (`run.rs` 806-810).
  So `w2m_unimplemented` now holds the word first (`runtime.c` 57-76, `hold_runtime_word`): a load of the word, and
  if this vCPU's owner does not hold it, a compare-and-swap of 0 for the owner, acquire, as `lock_runtime` does. In the
  built image (`build/guest_runtime_rust/ad301163...bin`, `_w2m_unimplemented` at 0x7f0004182024) it is "`ldr w10,
  [x9]; cmp w10, w8; b.eq; mov w10, #0x0; casa w10, w8, [x9]; cbnz w10`", inline, the word at 0x7f0009db9080
  (`MONITOR_VA + RUN_TABLE + 0x80`). `W2M_RUNTIME_LOCK_OFF` is held to `monitor_code::RUNTIME_LOCK` by
  `the_cs_runtime_word_is_the_monitor_codes`; `python_image.rs`' `DEPARTURES` names the change with Sprint 342.
  `_w2m_logged` moves from 0x7f0004182050 to 0x7f0004182080.
- **QueryPerformanceCounter's clock mode: the second form.** `process::write` sets `CLOCK_MODE` once at the load from
  the start options (`process.rs` 280-287: 1 for `clock_from_host`, 2 for `instruction_clock`, else 0), before any
  bridge is built; no host code writes it after but a unit test (`threads.rs` 1723, 1752). Both bridge builders read
  that word on vCPU 0's page (`clock_mode_at_load`, `guest.rs` 548), and `Service::lock_in` keeps QueryPerformanceCounter
  on the word in modes 1 and 2.
- **The take's words branched over, not dropped.** With them dropped, `qpc_runs_past_its_first_decline_compiling_as_the_python_host`
  failed: "block 0x140001000's code address, left: \"0x7f00000625b8\", right: \"0x7f0000062568\"", 20 words lower.
  `branched_over` emits a `b` past as many words, the rest `UDF #0`; the judge passes.
- **The tests.** `the_seven_services_of_lock_class_none_are_the_proofs` builds the table from the runtime's own C and
  holds the none class to the seven names, QueryPerformanceCounter's lock in modes 0, 1 and 2, and FlsGetValue,
  FlsSetValue and CreateEventW on the word. `a_bridge_of_lock_class_none_takes_no_runtime_word` assembles a none-class
  bridge, timed and not, and finds neither `lock_runtime`'s CASA nor `unlock_runtime`'s STLR, its length the word's; a
  word-class bridge and a traced none-class bridge hold both.
- **`qpc`'s word takings.** Before (`store/runs/w2m-vm-measurements-1795656377000000000`): `bridge` hold over "bridge;
  2 takings" (p50 3417 ns, p99 4625 ns) and "bridge; across an exit; 1 takings" (5375 ns), wait over "bridge; 3
  takings". After (`store/runs/w2m-vm-measurements-1795665127000000000`): "bridge; 1 takings" (7125 ns), "bridge;
  across an exit; 1 takings" (6625 ns), wait over "bridge; 2 takings". Three takings to two, against `qpc.c`'s one
  QueryPerformanceCounter call (GetStdHandle's and WriteFile's remain). `uv run substrate replay` at level 2 exit 0;
  `check_records`: "1 records; 0 violations".
- **The ring on two vCPUs, judged by a guest's own calls.** `the_threads_probes_own_calls_keep_the_ring_exact_on_two_vcpus`
  (`run_loop.rs` 1242) runs the threads probe in this process as `w2m-vm` runs it, on two vCPUs to ExitProcess 0:
  "the ring's count 6556726, per_api's total 6556726, the none-class calls [(\"GetCurrentThreadId\", 21),
  (\"GetLastError\", 30), (\"SetLastError\", 29), (\"TlsGetValue\", 2), (\"TlsSetValue\", 2)], runs by vCPU [269, 217],
  the exception area's first change None". `twoready` calls none of the seven (`twoready.c`: CreateEventA,
  CreateThread, SetEvent, WaitForSingleObject, WriteFile, GetStdHandle, ExitProcess). The probe's none-class calls are
  84 of 6,556,726, most of the rest the main thread's SwitchToThread loop under the word, so this judge holds the
  ring's count and capacity on a real run past 10,751 records; Sprint 341's test remains the race's judge.
- **Plant 1** "assertion `left == right` failed: the services of lock class none left:
  [\"KERNEL32!CreateEventW\", \"KERNEL32!GetCurrentProcessId\", ...]": CreateEventW added to `NO_LOCK` (`[&str; 8]`),
  `the_seven_services_of_lock_class_none_are_the_proofs` failing by name; restored by `cp`, checked by `cmp`.
- **Plant 2** "a bridge of class none, timed false: lock_runtime's CASA true and unlock_runtime's STLR true": the
  bridge taking and releasing the word for class none (`let word = ... || lock == Lock::None`), and `qpc`'s
  `--runtime-word` run under it counting 3 `bridge` takings again (15 in all, 14 without the plant); restored by `cp`,
  checked by `cmp`, and the rebuilt `w2m-vm` counts 2.
- **The gates** (copied from `tail -1 store/gates/passes.tsv`): `exit 0 in 566 s	default	2026-11-26T04:04:30Z	sprint 342	99b63cb80ad90b180dd92b6adf43cbc5c67eecb2	ce3d9efc0ad896d3f1b67fd1d65b26660f4a661269acc495b1352a147c931484	root=9de7e6386f1b3a5607b7e296527c28b54fbad306	rust=all python=0 samples=0	base=96add7c35bd51f9fee749844ff9d567f5fdbb0b6`
