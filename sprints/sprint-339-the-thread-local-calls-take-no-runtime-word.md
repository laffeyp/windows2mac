# Sprint 339 — The thread-local calls and QueryPerformanceCounter take no runtime word

```yaml
---
id: 339
status: active
phase: 6
pass_kind: functional
---
```

## scope

Change 1 of the host design: "a runtime call takes the lock of what it touches, or none"
(`docs/design-rust-host-2026-10-24.md` 95-140). Its first lock class is **None**, "for what one thread owns or what
no one writes". The design names those calls, which include the three hottest of the game's import mix:

- GetCurrentThreadId, GetCurrentProcessId;
- TlsGetValue, TlsSetValue, FlsGetValue, FlsSetValue;
- GetLastError, SetLastError;
- QueryPerformanceCounter, which reads `CNTVCT_EL0`.

Today every bridge takes the runtime word around its call into the runtime's C: `guest::bridge` in
`crates/w2m_vm/src/guest.rs` (545-603) does `take_runtime`, `blr`, `release_runtime`. So one word serializes every
kernel32 call on every vCPU. Sprint 338 put the host's own take behind a guard; this card changes the guest's bridge
for the None class.

- **The work:**
  - **The lock class.** Each service in the runtime's service table (`crates/guest_runtime/src/services.rs`,
    `Service`) carries its lock class. This card gives it two values, the runtime word, which every service takes
    today, and none. Only the services named above are set to none. Change 1's other classes (a heap's lock, the TLS
    slots' lock, the handle table's, a sync object's, the modules') stay on the word until their own cards.
  - **The bridge.** For a service of class none, `guest::bridge` emits no `take_runtime` and no `release_runtime`.
    Everything else stays as it is, the budget's count and the register save and restore included. A traced call
    (`traced` is `Some`, through `w2m_traced`) still takes the word, because `w2m_traced` writes the process-wide trace
    area. The design moves that to an atomic sequence number in its own later card.
  - **The proof, before any change.** For each of the nine services, read every store the bridge's path makes between
    its take and its release, and every store the service's C makes (`tools/guest_runtime/thread_storage.c`,
    `process.c`, `runtime.c`). Show each one lands in the calling thread's own state (its TEB, its TLS or FLS slots, its
    last-error) or in the calling vCPU's own pages (its state block, its runtime stack; Sprint 328). Quote the lines.
    A store to shared state keeps that service on the word, with the reason recorded. Any shared read the service makes
    is named too, with why it needs no lock.
- **The measurement.** `run_measurements`' `--runtime-word` run of `qpc` takes the word at the `bridge` site once per
  served bridge call (`Site::Bridge`). Before and after, quote its count of `bridge` takings and its
  `runtime_word_hold_ns` rows. After, the takings fall by the number of QueryPerformanceCounter calls `qpc` makes. A
  new record, `store/runs/w2m-vm-measurements-<ns>`, holds the after figures, and `tests/measurements_record.rs` keeps
  passing on it.
- **Not here:**
  - The other lock classes.
  - The trace's atomic sequence.
  - G6, the game's hot-import mix from six threads, which measures the whole of Change 1 once its classes are built.

## prerequisites

- 338 (closed)

## context_files

- `docs/design-rust-host-2026-10-24.md` 95-140, Change 1 whole, its lock classes and order.
- `crates/w2m_vm/src/guest.rs`: `bridge` (545-603), `service_for` (199-215), and its two callers (917, 1346).
- `crates/w2m_vm/src/monitor_code.rs`: `take_runtime` (283), `release_runtime`, `lock_runtime` (116),
  `unlock_runtime` (219), `Site` (243-263).
- `crates/guest_runtime/src/services.rs` (`Service` and the table), and the C of the nine services in
  `tools/guest_runtime/` (`thread_storage.c`, `process.c`, `runtime.c`; find each by `grep -n`).
- `crates/w2m_vm/src/bin/run_measurements.rs` and `crates/w2m_vm/tests/measurements_record.rs` (Sprint 337), and
  `harness/w2m-vm-measurements.toml`.
- `crates/w2m_vm/tests/run_loop.rs` 1015-1250, the two-vCPU tests that must keep passing.
- The Decision: `grep -n "Step 6 restored to the user's fix (b)" BLACKBOARD.md`.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands", with "A sprint closes in its work commit"
  and "no gate row is dropped".

## signal contract

### Emits

- The new measurements record: MEASUREMENT_RECORDED `runtime_word_hold_ns_p50` and `runtime_word_hold_ns_p99` for
  `qpc` as before, their `variant` naming the takings, and the rest of Sprint 337's record unchanged in form.

### Invariants

- Every test passes as before: the two-vCPU tests, the batch run, the traced (`--call-trace`) runs and the family runs.
- A traced call takes the word as before.

## artifact contract

### Files modified

- `crates/guest_runtime/src/services.rs` (the lock class) and `crates/w2m_vm/src/guest.rs` (the bridge); a test.

### Files created

- `store/runs/w2m-vm-measurements-<ns>/`

### Command exit codes

- `scripts/gates.sh` returns 0
- `uv run substrate replay <the new record> --level 2` returns 0, and `check_records` over it prints no violation
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The nine services' proof quoted in the result, line by line.
  - `qpc`'s `bridge` takings before and after, and their difference against `qpc`'s QueryPerformanceCounter calls.
  - A test that fails if a service of class none takes the word. For example: a text test over the service table
    naming the nine, and a bridge test that assembles a none-class bridge and finds no `lock_runtime` in its code.
- **Planted faults:**
  - (1) One service outside the nine, CreateEventW, set to none. The test, holding the class set to the nine, fails
    by name.
  - (2) The bridge emitting `take_runtime` for a none-class service. The bridge test fails, and `qpc`'s takings return
    to the before count.

  Before the work, read each judge's reach: the bridge test assembles the very path the plant changes.

## done criteria

The nine thread-local services and QueryPerformanceCounter take no runtime word, each with a written proof that it
touches only its thread's or its vCPU's own state. `qpc`'s word takings fall by its QueryPerformanceCounter calls, on
a record, and two plants fail by name.

## self-review checklist

- [x] Scope names one verifiable property: the None class's services take no word, on a record.
- [x] context_files cite Change 1, the bridge, the word's code, the service table, the C, the measurement and the tests.
- [x] Emits: tags and values in 0.13 only.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the proof, the counts, the test; two plants, their reach read.
- [x] Sweet spot: `services.rs` and `guest.rs` plus a test and a record; one concept, the None class.

## result

GATES-LINE
