# Sprint 338 — The runtime word's take behind one guard

```yaml
---
id: 338
status: done
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

- **The guard:** `run.rs` `RuntimeWordGuard` (695), `#[must_use]`, holding the view and whether the host took the
  word; its `Drop` (701) releases the word when it did and nothing when the vCPU's guest held it already.
  `take_word_unchecked` (715) returns the guard; `take_word` (729) returns `Option<RuntimeWordGuard>`, the stop read
  unchanged, and on a stop drops the guard (737), so the give-back is the `Drop`'s release, not a second hand-written
  one. `turn` (834) takes the guard by value beside the lock's guard and hands a new one back: a guard that took the
  word is dropped before the wait (853) and the word taken again after it; one that took nothing is kept across the
  wait, as the hold was.
- **The count:** `grep -c host_release_runtime_word crates/w2m_vm/src/run.rs`, 16 before and 12 after. Split at
  `#[cfg(test)]`: the code 11 before (`take_word`'s give-back, `turn`'s release, the nine sites) and 1 after, the
  guard's `Drop`; the tests 5 before and 11 after, the six added being the text test's own strings.
- **The nine sites**, each `let Some(word) = take_word(...)` and its hand release replaced by `drop(word)` at the same
  place, after the lock's guard, so the order is as before: `take_a_thread` (1300, dropped 1310, before the thread's
  load or the park); the turn's start (1399, 1416, inside the timed part); an exit's diagnostics (1450, 1539, after the
  section's lock); `hvc #13` (1571, 1582); `hvc #12` (1687, `mut word` through `turn`'s Idle loop, 1717); `hvc #10` and
  `#11` (1756, 1772, before `stepped?`); `hvc #4` (1813, `mut word` through `turn`'s Slice and Idle calls, 1863);
  `hvc #9` (1900, 1920); `hvc #1` (1930, 1972). No site had a `return` or `?` between its take and its release, so no
  path's order changed; a panic inside a service now releases the host's hold as it unwinds, where before the word
  stayed held.
- **Left as they are** (the guest's holds, not the host's take): `HaltAtEnd`'s `release_guests_word` at a vCPU's end
  and `kick_and_wait`'s `release_guests_word` and `take_word_for_guest`, both in `run.rs`; `monitor_code.rs`'s
  `lock_runtime` and `unlock_runtime`, the guest's own take and release, and `host_take_runtime_word`,
  `host_release_runtime_word` themselves.
- **The tests:** `run_loop.rs` `the_runtime_word_is_free_after_each_served_exit`: `sleeper` on one vCPU and on two,
  each run on a thread of its own, reads `RUN_TABLE + RUNTIME_LOCK` 0 after its ExitProcess, and holds the reach: turn
  starts above 0 and `hvc #1` and `#12` exits above 0, vCPU 1 running on two. Read on the tree before the change: one
  vCPU 23 turns' starts, exits `{1: 14, 8: 8, 12: 1, 15: 3}`; two vCPUs 25, `{1: 14, 8: 8, 12: 3, 15: 3}`, runs by vCPU
  `[13, 12]`. So the guest reaches five of the nine sites (the turn's start, an exit's diagnostics, `hvc #1`, `hvc
  #12`, `take_a_thread`) and the planted `Drop` through every one; not `hvc #4`, `#9`, `#10`/`#11` or `#13`. In
  `run.rs`'s own tests: `the_word_is_released_only_by_its_guards_drop`, a text test naming by line any
  `host_release_runtime_word(` call before `#[cfg(test)]` outside the guard's `Drop`, with fixtures for a hand pairing,
  the `Drop`'s own release and a comment; `a_stopped_runs_take_of_the_word_ends_its_service` now drops a taken guard and
  reads the word 0, and drops a guard over the guest's own hold and reads the hold kept.
- **Plant 1**, "sleeper on 1 vCPUs did not end in 120 s: a take waits on a runtime word left held" (the guard's `Drop`,
  704, `let _ = self.mem;` in place of the release; `the_runtime_word_is_free_after_each_served_exit` FAIL [120.031s]); the run's own limit of 60 s did
  not stop it, the host's take spinning outside guest code, which the limit cancels. Beside it
  `a_stopped_runs_take_of_the_word_ends_its_service` FAIL, "the word released by the guard's drop left: 4294967295
  right: 0" (`HOST_OWNER`), and the text test passing, as it judges the text and not the release. Made from a cp
  backup, restored by cp and checked by cmp ("cmp equal"), before the gates.
- **The gates:** one run, after the plant, on the restored tree; `signal_report --check` exit 0 before it.

exit 0 in 366 s	default	2026-11-26T02:03:17Z	sprint 338	ba288384588449a90e4df4adc5ff8c163b862770	c878882f5572426970b2e74c44998dc8ef5e0bf7f71843ea162266c8b036baaa	root=9fffa122ddd06efad951901c17490401dc20044f	rust=crates python=0 samples=0	base=fa47573ca63ec21c5286610f7171ec2d1aca5a13
