# Sprint 338 — The runtime word's take behind one guard

```yaml
---
id: 338
status: active
phase: 6
pass_kind: functional
---
```

## scope

Change 1 of the host design is the first lock change step 6 builds: "a runtime call takes the lock of what it touches,
or none" (`docs/design-rust-host-2026-10-24.md` 95). Its first card puts the host's take and release of the runtime
word behind one guard. The Decision "Step 6 restored to the user's fix (b)" (2026-11-24) orders it as C1's first card,
"the runtime word's take and release behind one guard".

Today `crates/w2m_vm/src/run.rs` takes the word at nine service sites, by `take_word` (1268, 1369, 1422, 1545, 1663,
1734, 1793, 1882, 1914). Each pairs the take by hand with `monitor_code::host_release_runtime_word` "on every path
before the loop goes on or stops" (1913). A path that returns early without the release leaves the word held. Every
other vCPU then spins on it, or a run stalls until its time limit. Nothing but care at each of the nine sites stops
this. Ousterhout's *A Philosophy of Software Design* (2nd ed., 2021, chapter 10, "Define Errors Out of Existence")
names the cure: one place that cannot be paired wrongly. Rust's form of it is a guard released in `Drop` (*The Rust
Programming Language*, chapter 15.3, "Running Code on Cleanup with the Drop Trait").

- **The guard.**
  - `take_word` returns a guard rather than `Option<bool>`. It holds whether the host took the word, and it releases
    the word in its `Drop` when the host took it. When a vCPU already held the word (a miss inside the runtime), it
    releases nothing, so a service whose borrows end at its scope cannot leave the word held.
  - The stop read stays as it is. When the run has stopped, the take gives the word back and returns no guard, so the
    service still returns before it reads what the word guards.
  - `take_word_unchecked` (`turn`'s take after the Machine's wait, 692-700) returns the same guard.
- **The nine sites.**
  - Each of the nine sites uses the guard, and its hand-written `host_release_runtime_word` calls go. The service
    order does not change; where a site must release the word before a later step (a host request, a switch), it
    drops the guard there, explicitly.
  - The sites outside `run.rs` stay as they are and are listed in the result: the stop's `release_guests_word`, and
    the guest's own take in `monitor_code.rs`.
- **A test.**
  - A test fails when the word is left held after a served exit.
  - One example: a run of `sleeper` on two vCPUs that reads the run table's `RUNTIME_LOCK` free (neither host nor
    guest) after its stop.
  - Another: a text test of `run.rs` naming any `host_release_runtime_word` outside the guard's `Drop`.
  - Choose on the grounds and record it. The text test also keeps the hand pairing from coming back.
- **Not here:**
  - Any change to what each service locks (Change 1 proper: a lock per heap, none for the thread-local calls and
    QueryPerformanceCounter). That comes next, on top of this guard.
  - C2 and C5.

## prerequisites

- 337 (closed)

## context_files

- `crates/w2m_vm/src/run.rs`:
  - `take_word_unchecked` and `take_word` (686-716);
  - every site `grep -n 'take_word\|host_release_runtime_word' crates/w2m_vm/src/run.rs` names, each read whole with
    its service's early returns;
  - the unit tests at 2064-2130, which take and release the word directly.
- `crates/w2m_vm/src/monitor_code.rs`: `lock_runtime` (116), `host_take_runtime_word` (148),
  `host_take_runtime_word_waiting` (154), `host_release_runtime_word` (184), `release_guests_word` (192),
  `take_word_for_guest` (201).
- `docs/design-rust-host-2026-10-24.md`, "Change 1: a runtime call takes the lock of what it touches, or none" (from
  line 95).
- `docs/design-run-loop-on-several-vcpus-2026-10-31.md`, "The exits" and "How the threads share `Guest`".
- `crates/w2m_vm/tests/run_loop_lock.rs` (the text tests of `run.rs`' shape, the form a new one takes), and
  `crates/w2m_vm/tests/run_loop.rs`' two-vCPU tests (1015-1250).
- The Decision: `grep -n "Step 6 restored to the user's fix (b)" BLACKBOARD.md`.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands", with "A sprint closes in its work commit"
  and "no gate row is dropped".

## signal contract

### Emits

- None; how the host takes and releases the runtime word, no new event.

### Invariants

- Every test that passes today passes, the two-vCPU tests included, and the batch run stops where it is held.
- The runtime word is never held by the host after a service returns.

## artifact contract

### Files modified

- `crates/w2m_vm/src/run.rs`, and a test file in `crates/w2m_vm/tests/`

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - `grep -c host_release_runtime_word crates/w2m_vm/src/run.rs` before and after, quoted. After, it counts only the
    guard's `Drop` and the stop's give-back in `take_word`.
  - The new test passes.
  - `two_vcpus_end_each_guest_as_one_does`, `a_locked_model_step_loses_no_add_of_another_vcpus_aligned_lock` and the
    batch run pass as before.
- **Planted fault:**
  - (1) The guard's `Drop` made to release nothing. The new test fails by name. Quote the run-level judge's output, a
    two-vCPU guest stopping at its limit or the word read held, if the test chosen is the run-level one.
  - Before the work, read the judge's reach: the test runs the service whose guard is planted, so name which site's
    service the test's guest reaches.

## done criteria

The host takes and releases the runtime word through one guard, so no service path can leave it held. A test fails if
the guard does not release, and every test that passed before still passes.

## self-review checklist

- [x] Scope names one verifiable property: one guard releases the word; no hand-paired release left in `run.rs`.
- [x] context_files cite the take, the nine sites, the word's functions, Change 1, the shape tests and the two-vCPU tests.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the count before and after, the tests, one plant with its reach read.
- [x] Sweet spot: `run.rs` and one test; one concept, the guard.

## result

GATES-LINE
