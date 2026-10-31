# Sprint 294 — The scheduling exits under the runtime word

```yaml
---
id: 294
status: done
phase: 6
pass_kind: functional
---
```

## scope

The first card of `docs/design-run-loop-on-several-vcpus-2026-10-31.md`. The switch stub releases the runtime word
before `hvc #12` (`guest.rs` `switch_stub`: `unlock_runtime`, then `hvc(HVC_SCHEDULE)`), and a slice's `hvc #4` comes
from a translated block, so both reach the host without the word; their services change `threads` and the turn
(`threads::schedule`, `set_budget`, `wake_expired`, `hand_over`), which another vCPU's services change under it.
`hvc #13`'s service changes the host compiler's tables, and whether its vCPU holds the word is not established. The
miss service, `hvc #1`, takes the word itself (Sprint 286).

- **The work:**
  - `run.rs`: one helper takes the word for a service that changes shared state, counting a service found outside it,
    for `hvc #1`, `#4`, `#12` and `#13`; each releases it on every path before the loop goes on or stops;
  - a process's turn (`hand_over`) waits with the word let go and takes it again after, so no wait holds it;
  - `Outcome.misses_outside_word` renamed `served_outside_word`, its count over all four.
- **Not here:** `Guest` divided (the design's second card); a second vCPU.

## prerequisites

- 293 (closed)

## context_files

- `docs/design-run-loop-on-several-vcpus-2026-10-31.md` ("The exits", "The cards" 1)
- `crates/w2m_vm/src/run.rs` 540-600 (`hand_over`), 796-1110 (the exits); `crates/w2m_vm/src/monitor_code.rs` 140-170
- `crates/w2m_vm/tests/run_loop.rs` (`qpc_runs_past_its_first_decline_compiling_as_the_python_host`, `sleeper_...`,
  `model_steps_...`)

## signal contract

### Emits

- None; a lock's coverage, held by tests.

### Invariants

- Every run as held: the batch run, the guests, the family runs, the run loop's tests.

## artifact contract

### Files modified

- `crates/w2m_vm/src/run.rs`, `crates/w2m_vm/tests/run_loop.rs`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `served_outside_word` 0 on the qpc, sleeper and model-steps runs; the sleeper's `hvc #12`
  hand-overs served, counted in `out.exits`; every run as held.
- **Planted faults**, each judge looked up to reach the plant's line and to assert what it breaks, and each plant
  changing what the code does:
  - (1) `hvc #12` served without the take. Judge: the sleeper's test, whose run hands the vCPU over by `hvc #12`
    (it asserts `out.handovers` against the frozen host's, `run_loop.rs` 826-844), given an assertion of
    `served_outside_word` 0.
  - (2) the helper counting without taking the word. Judge: the qpc test's assertion of `served_outside_word` 0
    (`run_loop.rs` 244), its misses then served outside the word.
- **What no test sees, said so:** a turn's wait with the word held blocks no one on one vCPU, a child process waiting
  on its own word in its own memory, so it has no plant until a second vCPU shares the word; `hvc #13` reaches no test
  through an exit (`run_loop.rs` 576 and 619 call `code_written` directly).

## done criteria

No service that changes shared state is served outside the runtime word, and no wait holds it.

## self-review checklist

- [x] Scope names one verifiable property: the four services under the word.
- [x] context_files cite the design, the exits, the take and the tests.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each judge looked up; what no test sees named.
- [x] One concept: the services' coverage by the word.

## result

- **The services** (ca7f60bf2): `take_word`, the runtime word's take with a count of a service found outside it, for
  `hvc #1`, `#4`, `#12` and `#13`; `turn`, `hand_over` with the word let go for the wait; `#12` releasing after its
  last read of the threads, `#4` once after a labelled block, `#13` around `code_written`;
  `Outcome.served_outside_word`, renamed from `misses_outside_word`; the sleeper test asserting it 0.
- **Files the card did not name,** a departure: `crates/w2m_vm/src/threads.rs` and `monitor_code.rs`, for plant 1's
  judge. The assertion reaches `crate::run::own_word`, the scheduler calling up into the run loop that calls it; the
  owner's read belongs beside `runtime_word_held` in `monitor_code.rs`, with one `LOCK_OWNER` where two hold 0xB0
  today (`run.rs` 34, `guest_runtime/src/layout.rs` 26): card 295, which divides `Guest` and touches both.
- **No take nested in a hold:** the take's callers are the four services and `turn`, which takes after releasing,
  and the drain's take (`run.rs` 466-471), before any service; nothing the services call takes the word.
- **Expected values:** `served_outside_word` 0 on the qpc and sleeper runs; 173 tests in the pre-commit run, the
  batch-run step first, the game, the guests and the families as held.
- **The gates:** `scripts/gates.sh` exit 0 in 952 s, kept in `store/gates/passes.tsv` as sprint 294 at 6458bbe12 with root=fca899046 and "rust=crates python=0 samples=0", rdeps(signal_report) and rdeps(w2m_vm), 211 tests, base=6e52ab880, Sprint 293's close; multi-vCPU: not run (Python gates not run)
- **Plant 1,** restored by `cp` and checked by `cmp`: `hvc #12` served without its take (`run.rs` 951). The card's
  judge could not see it, found before it ran: the count reads the holder inside `take_word`, so a site without the
  take counts nothing. The close's record check refused the close for the plant's unquoted result, and a judge was
  added (6458bbe12): `threads::schedule` asserts, in debug builds, that the word is held by the host or the calling
  vCPU (`monitor_code::runtime_word_held`), the first piece of the guard the design's second card puts over `served`.
  The sleeper test failed at `threads.rs` 687, "the scheduler entered without the runtime word".
- **Plant 2,** restored by `cp` and checked by `cmp`: `take_word` reading the holder without taking the word; the qpc
  test failed, "services served outside the runtime word left: 6 right: 0", and the sleeper test, "left: 15 right:
  0", its misses and its `hvc #12` hand-overs counted.
