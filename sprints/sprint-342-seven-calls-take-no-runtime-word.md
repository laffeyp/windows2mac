# Sprint 342 — Seven calls take no runtime word

```yaml
---
id: 342
status: active
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

GATES-LINE
