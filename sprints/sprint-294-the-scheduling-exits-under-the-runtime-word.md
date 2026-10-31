# Sprint 294 — The scheduling exits under the runtime word

```yaml
---
id: 294
status: active
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

- **The gates:** GATES-LINE
