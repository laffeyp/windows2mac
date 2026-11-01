# Sprint 325 — A machine is refused on several vCPUs

```yaml
---
id: 325
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Architecture: a vCPU with no thread" (the run-loop design's "The Machine", Sprint 318): until the Machine
has its own thread (card 4f, the Python host's Sprint 122), a process with a machine link is refused on more than one
vCPU, by name, when its link is made: `host_request.rs` 524-530, where a `HOST_KERNEL` spawn makes the supervisor's
link mid-run. `w2m-vm` reads `--vcpus` and hands the count to `vcpus_allowed` alone (`w2m-vm.rs` 300-320, 389); no
part of the run knows it. A child process is refused at the start already (`vcpus_allowed`'s `child`, Sprint 308);
a parent that spawns is not, and card 4c3b2b, the threads, lifts "a second vCPU is not built". This card comes
before it.

- **The work:** the run's vCPU count on the guest's start (`Start`), 1 unless `--vcpus` says more, set where
  `vcpus_allowed` reads it; the spawn refused, before any link or child is made, when the count is above 1, as
  `HostError::NotServed` with the reason "a process that starts another runs on one vCPU until the Machine has its own
  thread (the run-loop design's card 4f)", so the run stops as `Stop::HostRequest` naming it.
- **Not here:** the Machine's own thread (card 4f); the threads (4c3b2b).

## prerequisites

- 324 (closed)

## context_files

- `docs/design-run-loop-on-several-vcpus-2026-10-31.md` "The Machine" (199-204); `crates/w2m_vm/src/host_request.rs`
  509-540; `crates/w2m_vm/src/bin/w2m-vm.rs` 295-395; `crates/w2m_vm/src/run.rs` `vcpus_allowed`;
  `crates/w2m_vm/src/process.rs` `Start` (58)

## signal contract

### Emits

- None; a refusal by name in the run's stop.

### Invariants

- Every run as held: on one vCPU, `busychild` and `twoproc-ic` spawn as before.

## artifact contract

### Files modified

- `crates/w2m_vm/src/host_request.rs`, `crates/w2m_vm/src/process.rs`, `crates/w2m_vm/src/bin/w2m-vm.rs`,
  `crates/w2m_vm/tests/` (the refusal's test)

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** a test serving a `HOST_KERNEL` spawn on a guest whose start says 2 vCPUs, the request refused
  with the reason above and no link made (`served.link` still `None`); the same request on 1 vCPU making the
  supervisor's link; the batch run and the guests as held.
- **Planted faults:** (1) the count's test taken out of the spawn: the 2-vCPU case makes a link, and the test fails
  naming it; (2) the count left at its default where `w2m-vm` reads `--vcpus`: a test of the start `w2m-vm` builds for
  `--vcpus 2` reads 1 and fails.

## done criteria

A process that starts another stops by name on a run of more than one vCPU, before its link is made.

## self-review checklist

- [x] Scope names one verifiable property: the spawn refused above one vCPU.
- [x] context_files cite the Decision, the spawn and the count's one reader.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each changing what its judge reads.
- [x] One concept: the Machine's refusal.

## result

- **The count** (`crates/w2m_vm/src/process.rs`): `Start.vcpus`, 1 in `Start::console`; `w2m-vm` sets it on the guest
  path after `vcpus_allowed` (`w2m-vm.rs` 389-392). The game's start keeps 1: `--game` takes the instruction clock,
  which `vcpus_allowed` holds to one vCPU (`w2m-vm.rs` 319-320), so the count is not carried through `GameOptions`.
- **The refusal** (`crates/w2m_vm/src/host_request.rs`): at the head of `HOST_KERNEL`'s spawn, before `child_start`,
  the link and `Supervisor::spawn`, `guest.start.vcpus > 1` returns `HostError::NotServed(MACHINE_ON_ONE)`, "a process
  that starts another runs on one vCPU until the Machine has its own thread (the run-loop design's card 4f)", which the
  run loop stops as `Stop::HostRequest`.
- **The test** `run_loop::a_spawn_on_two_vcpus_is_refused_before_its_link`: `busychild` on a start of 2 vCPUs (the run
  itself one vCPU; the refusal reads the count alone) stops as `Stop::HostRequest` with that reason, `served.link`
  still `None`; passing. The spawn on one vCPU is held by the gates' `busychild` and `twoproc-ic` rows in
  `store/batch_run/first-stop-2026-10-28.tsv`, read by `batch_run.rs`.
- **Plant 1,** `host_request.rs` copied aside and restored by `cp`, checked by `cmp`: the test `if guest.start.vcpus >
  1` made `if false`; the test failed, "busychild on 2 vCPUs stopped as Handover { …, why: \"child process 4100: its
  w2m-vm ended without a word to the supervisor (Some(ExitStatus(unix_wait_status(25856))))\" }". Its first two runs
  ran no test: `timeout`, which macOS does not have, ended the first before nextest started, the output filtered to
  nothing, and the second printed "command not found: timeout".
- **Plant 2 not run, and why:** it needs `w2m-vm --vcpus 2` to reach a spawn, and `vcpus_allowed` refuses any count
  above 1 with "a second vCPU is not built" before the start is built, so `w2m-vm`'s `start.vcpus = vcpus` is reached
  by no run until card 4c3b2b lifts that refusal; 4c3b2b's judge holds it then, `busychild` on two vCPUs stopping with
  `MACHINE_ON_ONE`.
- **The gates:** GATES-LINE
