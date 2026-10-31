# Step 6's census, 2026-10-30

Plan step 6 (`docs/rust-port-plan-2026-10-24.md` 161-168): "Several vCPUs. The redesigned host on two vCPUs and
more, with the runtime's locks split as the design sets … Gate: the litmus tests MP, SB, LB, 2+2W and IRIW; the 68
families on two vCPUs; the batch run on the wall clock; Sprint 122's guests (handshake, busychild, crosswake,
pingpong, statsize) ending on two vCPUs in many runs, the count in the card (crosswake's hang, BLACKBOARD Decision
2026-10-23, is fixed here); instructions a second on four vCPUs against the game's 4.6 billion over six threads, on a
guest that is not mostly waiting; the hot imports' rate on six threads at once against one." The design is
`docs/design-rust-host-2026-10-24.md`. Step 5 closed at Sprint 268 (`docs/step-5-close-2026-10-30.md`).

The Rust host today makes one vCPU in every run: `crates/w2m_vm/src/bin/w2m-vm.rs` 101 and 489, and
`crates/w2m_vm/src/case_inputs.rs` 233, each one `Vcpu::new`. Every row below starts from that.

## The gate

| # | What it asks | The frozen host | The Rust host | Status | What shows it met |
|---|---|---|---|---|---|
| G1 | the litmus tests MP, SB, LB, 2+2W and IRIW | Sprint 105, `tests/test_litmus.py` at the tag: 20 runs of 100,000 iterations, MP, LB, 2+2W and IRIW's forbidden outcomes 0 in 2,000,000 each, SB's relaxed outcome 1 to 36 a run; a plant with EnTSO clear gave MP's forbidden outcome 4 to 9 times a run; also `probes/mac_hv_litmus/mp_litmus.c`, `store/runs/litmus-1791441621` and `store/runs/litmus-1791496387` (Sprint 009) | none: one vCPU | absent | the five tests on two vCPUs of the Rust host, the same counts, and the EnTSO plant named |
| G2 | the 68 families on two vCPUs | two families on two vCPUs, threads and running_thread_context (Sprint 119, `docs/built-archive-2026-10-30.md` 125) | 68 of 68 on one vCPU, 273 of 273 runs (Sprint 234, `crates/w2m_vm/tests/family_runs.rs`) | absent on two | 273 of 273 runs on two vCPUs |
| G3 | the batch run on the wall clock | two scratch runs on the wall clock, two vCPUs and one, conforming (Sprint 122, `docs/sprint-tail-archive-2026-10-08.md`) | the batch run on the instruction clock, one vCPU (`w2m-vm.rs`, `start.instruction_clock`), conforming | absent on two | the batch run on the wall clock on two vCPUs conforming, in several runs |
| G4 | Sprint 122's guests ending on two vCPUs in many runs, the count in the card | ending on two vCPUs but crosswake, which reached its 20 s limit once in the gates of 2026-10-22 and was quarantined (Decision 2026-10-23) | 16 of 16 guests end on one vCPU (Sprint 234, `crates/w2m_vm/tests/batch_run.rs`), the six that start a second process in turns, a child `w2m-vm` a process (`crates/w2m_vm/src/turns.rs`, Sprint 197) | absent on two | handshake, busychild, crosswake, pingpong and statsize ending on two vCPUs in a counted number of runs, crosswake among them |
| G5 | instructions a second on four vCPUs against the game's 4.6 billion over six threads | not measured | not measured; one vCPU, 2.87 to 2.93 billion (Sprint 240) and 3.17 to 3.21 billion with the cache (Sprints 262-263) | absent | a guest that is not mostly waiting, on four vCPUs, with its rate on the record |
| G6 | the hot imports' rate on six threads at once against one | not measured; the plan's budget is "several hundred thousand a second over six threads, by the hot imports alone" (`docs/rust-port-plan-2026-10-24.md` 55-56) | not measured | absent | the rate on six threads at once and on one, on the record, with each lock's hold and wait |

## The design's changes

| # | Change | The Rust host | Status |
|---|---|---|---|
| C1 | a runtime call takes the lock of what it touches, or none | one runtime word for all of the runtime's state, `lock_runtime` (`crates/w2m_vm/src/monitor_code.rs` 110-129), as the frozen host's | absent |
| C2 | a host request holds no guest lock, on its vCPU's own transport page | one transport page a process, `PROCESS_VA + TRANSPORT_OFF` (`crates/guest_runtime/src/layout.rs` 99); the runtime word is held across `hvc #8`, so two vCPUs' requests do not meet while that holds | absent |
| C3 | translation in regions, looked up without a lock | one code area, one cursor (`VMC_CURSOR`), the in-VM compiler and the installer under the runtime word | absent |
| C4 | one process's waits stay in its VM; the supervisor sees what crosses processes | a child `w2m-vm` a guest process, run in turns (`turns.rs`, Sprint 197) | partial |
| C5 | every lock word has three states, and a waiter stops spinning | `lock_runtime` a two-state `CASA` spin that rereads the word until it is free | absent |

## Deferred and Surfaced items that wait on several vCPUs

| # | Item | Where it stands | What shows it met |
|---|---|---|---|
| D1 | a memory XCHG not atomic (the sweep of 2026-10-29: "step 6's first card") | `crates/translator/src/rules.rs` 629-644: a load and two plain stores; x86 locks an XCHG with memory implicitly; the locked forms are atomic (`rules.rs` 81-82) | XCHG with memory lowered as an atomic swap, judged on the class runs and by a litmus test of two vCPUs exchanging one word |
| D2 | a parked vCPU learns of a ready thread only at the next host serve (Sprint 118) | Deferred 2026-10-18 | the second vCPU busy when a thread is readied, by an SGI or the channel kick the entry names |
| D3 | the first taking of the runtime word waits 6.8 to 7.5 µs (Sprint 164) | Deferred 2026-10-28, cause not measured | the first taking's wait measured with the page touched first, on two vCPUs |
| D4 | crosswake with its kick planted away ended where it pins a hang, once (Sprint 184) | Deferred 2026-10-28 | the Rust host's planted no-kick test, counted runs |
| D5 | the narrow kick rule has no blocking guard and no Rust test (Sprint 222) | Deferred 2026-10-29 | a Rust test pinning the spinner to vCPU 0 and the child's start to vCPU 1 |
| D6 | the in-VM compiler's counters and the installer's shared words are plain adds and stores (Sprint 247, annotated at 248 and 261) | Deferred 2026-10-30 | per-vCPU slots the host sums, or atomic adds, with the frame bound in `batch_record.rs` kept or changed |

## What the counts say

All six gate items are absent on the Rust host, which has never run a second vCPU, and four of the five design
changes are absent; the fifth is partial. The frozen host ran two vCPUs under its one runtime word, held across every
service and host request, and its litmus results stand (G1). That word is the Rust host's too, so a second vCPU under
it is the frozen host's shape, and the litmus tests can judge the memory model before any lock is split. D1 is a
defect only a second vCPU exposes, and every lock built on an XCHG in the guest depends on it. The design's changes
are judged by G5 and G6, which measure the waits they exist to remove.
