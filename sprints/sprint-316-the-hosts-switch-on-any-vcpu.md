# Sprint 316 — The host's switch on any vCPU

```yaml
---
id: 316
status: done
phase: 6
pass_kind: functional
---
```

## scope

The run-loop design's card 4c3, first part (`docs/design-run-loop-on-several-vcpus-2026-10-31.md`, card 4; split at this
card into 4c3a, this, and 4c3b, the threads). The host's thread switch (`threads.rs` `switch`, "for vCPU 0 of one
process") marks the thread it loads held by vCPU 0, writing 1 into the run entry's `vcpu` word (`E_VCPU`, 878), and
refuses to load one another vCPU holds (864-869); but it picks the next ready thread without skipping one held
elsewhere, so on two vCPUs it would stop the run where the C runtime's own switch picks another
(`tools/guest_runtime/threads.c` 68, `held_elsewhere`; 293-294, the claim with `this_vcpu()`, the vCPU's `LOCK_OWNER`).

- **The work:** the switch for the vCPU whose pages are `served.vcpu`: its owner value read as the C reads it
  (`vcpu_owner`, the vCPU's `LOCK_OWNER`), written into the loaded thread's `vcpu` word in place of 1; the pick skipping
  a ready thread whose `vcpu` word is another vCPU's, as `held_elsewhere` does; the load's refusal kept for a word that
  changed between the pick and the load. One vCPU runs as before, its owner 1.
- **Not here:** the threads that run a second vCPU (4c3b); the parked vCPU's wait.

## prerequisites

- 315 (closed)

## context_files

- the design's card 4 and "The threads"; `threads.rs` (`schedule`, `switch`, the load at 852-882); `threads.c` 41,
  65-68, 285-295; `monitor_code::vcpu_owner`

## signal contract

### Emits

- None; the switch's choice.

### Invariants

- Every run as held; one vCPU.

## artifact contract

### Files modified

- `crates/w2m_vm/src/threads.rs`; a unit test of the pick; the design's card 4 split

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** every run as held, the sleeper run's hand-overs among them; a unit test of two ready threads
  after the current one, the first marked held by owner 2: the switch on owner 1 loads the second and writes 1 into its
  word; the same with no other ready thread: no thread loaded from another vCPU.
- **Planted faults:** (1) the pick without the skip: the unit test fails, the load's refusal naming the thread; (2) the
  owner written as 1 whatever the vCPU: a unit test on owner 2 reads 1 in the loaded thread's word.

## done criteria

The host's switch serves any vCPU's exit as the C runtime's switch does, never loading a thread another vCPU holds.

## self-review checklist

- [x] Scope names one verifiable property: no thread loaded that another vCPU holds.
- [x] context_files cite the design, the host's switch and the C runtime's.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each changing what its judge reads.
- [x] One concept: one switch's choice of thread.

## result

- **The switch** (`threads.rs` `switch`): `own`, the served vCPU's `LOCK_OWNER` word read whole as `this_vcpu` reads
  it (790); `here`, a thread whose `vcpu` word is 0 or `own`; the pick of a ready thread and the timed waiters both
  restricted to `here`; the load's refusal comparing with `own` and naming both owners (874); the loaded thread
  claimed with `own` (889). `add_thread`'s claim of the first thread at load stays 1, vCPU 0's owner, which the run
  loop writes into `LOCK_OWNER` at its start (`run.rs` 814).
- **Found for 4c3b:** `private_chain` copies vCPU 0's monitor data page, `LOCK_OWNER` with it; the card that starts
  vCPU `k` writes `k + 1` there before its first run, as `run.rs` 814 does for vCPU 0.
- **The unit test** `threads::tests::the_switch_passes_a_thread_another_vcpu_holds_and_claims_with_its_own_owner`
  (1237): three threads, the first current; on owner 1, the second held by owner 2 is passed by and the third loaded,
  the claims after (2, 1); on owner 2, the second loaded and claimed with 2.
- **Plant 1,** restored by `cp` and checked by `cmp`: `here` taken out of the pick; the test failed on the load's
  refusal, "thread 8 loaded on vCPU owner 1 while vCPU owner 2 has it".
- **Plant 2,** restored likewise: the claim written as 1; the test failed, "the loaded thread claimed with this vCPU's
  owner", left 1, right 2.
- **The gates:** `scripts/gates.sh` exit 0 in 734 s, kept in `store/gates/passes.tsv` as sprint 316 at e17e0cee0 with root=389bfeaa6 and "rust=crates python=0 samples=0", 188 tests, 654 ns an exit at load 4.22 (Sprint 315's 822 ns at load 5.85 not repeated), base=97cdecd4a, Sprint 315's close; multi-vCPU: not run (Python gates not run)

