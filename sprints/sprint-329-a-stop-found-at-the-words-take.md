# Sprint 329 — A stop found at the word's take

```yaml
---
id: 329
status: active
phase: 6
pass_kind: functional
---
```

## scope

Deferred (Sprint 327) and reviewer-3's Low 2 at 327. At a run's stop, `monitor_code::release_ended_vcpus_word` lets go
an ended vCPU's guest's hold of the runtime word, so a vCPU waiting on the word is not left waiting for ever. The
waiter's service then reads what the word guards (the code cursor, the reconstruction and dispatch tables, the run
table), which the ended vCPU's guest may have left half-written when `hv_vcpus_exit` stopped it, before any turn's
start finds the run's stop flag. Every service takes the word through `take_word` (`run.rs` 685; ten calls in
`vcpu_loop` and `take_a_thread`) or, after each exit, `host_take_runtime_word` directly (`run.rs` 1287).

- **The work:** `take_word` takes the run's stop (`RunStop`) and returns `Option<bool>`: `None` when, after the take,
  the run has stopped, the word given back if the host took it; each caller returns before its service reads,
  `vcpu_loop` with `Ok(None)`, the vCPU ending recording nothing; the after-exit take goes through `take_word`. The
  type makes each caller say what a stop does.
- **Not here:** `turn`'s take (`run.rs` 814), the Machine's wait: a process with a machine link runs on one vCPU
  (`MACHINE_ON_ONE`, Sprint 325), so no other vCPU there leaves state half-written; card 4f, which gives the Machine
  its own thread, takes it.

## prerequisites

- 328 (closed)

## context_files

- BLACKBOARD Deferred (Sprint 327), the word released at a stop; reviewer-3's Low 2 at 327; `crates/w2m_vm/src/run.rs`
  `take_word`, `RunStop`, `vcpu_loop`, `take_a_thread`, `run_on_vcpu`; `crates/w2m_vm/src/monitor_code.rs`
  `host_take_runtime_word`, `release_ended_vcpus_word`

## signal contract

### Emits

- None; where a stopped run ends a service.

### Invariants

- Every run as held, on one vCPU and two.

## artifact contract

### Files modified

- `crates/w2m_vm/src/run.rs` (its unit tests among it)

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** a unit test of `take_word`: with the run's stop not set and the word free, `Some(true)` and the
  word the host's; with the stop set and the word free, `None` and the word free after; with the stop set and the
  word held by this vCPU's guest, `None` and the word still its guest's. `qpc`, `sleeper` and `twoready` on two vCPUs
  and `statsize` through `w2m-vm` as held; every run on one vCPU as held.
- **Planted faults:** (1) the stop's check taken out of `take_word`: the unit test's stopped case reads `Some(true)`;
  (2) the word not given back when a stopped take finds the host took it: the unit test reads the word still the
  host's. Each judge's reach read before this card's commit (WORKING_AGREEMENT, Sprint 284): the unit test calls
  `take_word` itself, with a `RunStop` it sets, on a guest's memory, and reads the word after.

## done criteria

No service reads the word's state once its run has stopped: `take_word` finds the stop before the service runs.

## self-review checklist

- [x] Scope names one verifiable property: a stopped run's take ends the service.
- [x] context_files cite the Deferred entry, the review and the takes.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each judge's reach read.
- [x] One concept: the stop at the word's take.

## result

