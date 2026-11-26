# Sprint 353 — Every per-vCPU word the host writes, checked against run-wide state, and slicing past vCPU 0 judged

```yaml
---
id: 353
status: done
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-4's twenty-first review, Medium 1, recorded as the Decision "The served-vCPU words, enumerated in card 353".
Sprint 351 found that a thread taken by vCPU 1 ran on the run's whole step budget and was never put down. `set_budget`
(`crates/w2m_vm/src/threads.rs` 1328-1341) computes `shared` from run-wide state,
`guest.served.threads.len() > 1 || guest.served.link.is_some()`, and writes the limit only at
`guest.served.vcpu.monitor + 8`. Its callers are `threads.rs` 1322, `host_request.rs` 545 and `run.rs` 1358 and 1877.
Sprint 351 added a call in `take_a_thread`. That defect names a class: a per-vCPU word whose right value depends on
state another vCPU's service can change, written only for the vCPU served when that state changed.

- **The enumeration.** List every word of a vCPU's monitor data page or state block that the host writes, one line a
  word: its writer and line, which vCPU it writes, and whether its right value depends on run-wide state. The words
  include at least:
  - at the run's start, for every vCPU: the limit `md + 8`, `MXCSR_OFF`, `LOCK_OWNER`, `FCW_OFF` (`run.rs` 983-998);
  - at each turn's start, for the turning vCPU: `SLICE_UNTIL`, `RUN_END` (`run.rs` near 1464);
  - for the served vCPU: `CURRENT`, `CURRENT_ENTRY`, `SLICE_COUNT`, `SLICE_PAUSES`, the limit (`threads.rs` 175-176,
    1033-1034, 1067-1068, 1075-1076, 1341);
  - any other found by grep of the monitor-data offsets in `crates/w2m_vm/src`.
- **Each word whose value can go stale on another vCPU is a defect.** It can leave a thread past its slice, so fix it
  in this card under the user's Decision of 2026-10-18, and report each one to the Supervisor at once with its
  evidence. Two cases to check first:
  - `SLICE_UNTIL`, computed from a run-wide count of ready threads that another vCPU's service changes between this
    vCPU's turns;
  - `set_budget` when `shared` turns true while another vCPU is mid-slice on the whole budget, for instance a second
    thread made on vCPU 0 while vCPU 1 runs the first.
- **The judge.** A guest built as `tests/data/timed_waits`' guests are: three CPU-bound threads on two vCPUs, each
  counting in a loop, the second made while the first already runs on vCPU 1. It is held to every thread's count
  being above 0 by the run's end, and the run ending inside its limit. Run it 20 times on two vCPUs and quote the
  counts. Give it its held stop line in `store/batch_run/first-stop-2026-10-28.tsv` if the batch run reads it (Sprint
  350's `shared_close`).
- **Not here:** the woken mark; any lock.

## prerequisites

- 352 (closed)

## context_files

- `crates/w2m_vm/src/threads.rs` (`set_budget`, `take_a_thread`, `slice_until`, every write to a monitor-data offset)
  and `crates/w2m_vm/src/run.rs` (the run's start near 983-998, the turn's start near 1464, 1358, 1877).
- `crates/w2m_vm/src/host_request.rs` 540-550 and the monitor-data offsets' definitions
  (`grep -n 'SLICE_UNTIL\|RUN_END\|CURRENT_ENTRY\|SLICE_COUNT\|SLICE_PAUSES\|LOCK_OWNER\|MXCSR_OFF\|FCW_OFF' crates/w2m_vm/src/*.rs crates/guest_runtime/src/*.rs`).
- Sprint 351's result (`sprints/sprint-351-the-family-runs-on-two-vcpus.md`), its fix and its diagnosis.
- `tests/data/timed_waits/shared_close.c` and its test in `crates/w2m_vm/tests/run_loop.rs`, for a w2m-only guest's
  form and its held stop.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None; which per-vCPU words the host writes, and when.

### Invariants

- On one vCPU every run, the batch run, the exit ledger and the family runs pass unchanged.
- The 273 family runs on two vCPUs pass as at Sprint 351.

## artifact contract

### Files modified

- `crates/w2m_vm/src/threads.rs` and `run.rs` as the fixes need, the new guest and its test, and any held line the
  gates name.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The enumeration, one line a word.
  - Each stale word found, its evidence and its fix.
  - The guest's counts over 20 runs on two vCPUs, before any fix and after.
- **Planted faults:**
  - (1) `take_a_thread`'s `set_budget` call removed. The guest's test fails, a thread's count 0 or the run's limit.
  - (2) Each further fix this card makes taken out in turn. The guest's test, or a test the fix adds, fails.

  Before any fix, read the judge's reach: plant 1 on today's code.

## done criteria

Every per-vCPU word the host writes is listed with whether it depends on run-wide state; every one that could go
stale on another vCPU is fixed; and a guest of three busy threads on two vCPUs shows every thread sliced and the run
ending inside its limit.

## self-review checklist

- [x] Scope names one verifiable property: no per-vCPU word stale on another vCPU, slicing past vCPU 0 judged.
- [x] context_files cite the writers, Sprint 351's fix and the guest's form.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the enumeration, the fixes, the counts; numbered plants, their reach read.
- [x] Sweet spot: `threads.rs` and `run.rs` with one guest; one concept, the per-vCPU words.

## amendment (2026-11-26)

Builder-353's guest, `tests/data/timed_waits/three_counters`, found two stale `SLICE_UNTIL` cases. Both are fixed in
this card under the user's Decision of 2026-10-18:
- **(A) The host.** When vCPU 0's service readies threads, vCPU 1, running T1, keeps `SLICE_UNTIL` at
  0xffffffffffffffff. T1 ran unsliced from count 0 to 32,342,801 while T2 and T3 were ready.
- **(B) The runtime C.** `threads.c`' `readied()` and `w2m_switch` write only their own vCPU's `SLICE_UNTIL`. T2 ran
  alone on vCPU 0 for the whole run, while T1, put down ready by vCPU 1, waited; this happened in 20 of 20 runs.
  - The fix may move the runtime image. Since Sprint 352 the exit ledger names runtime pointers by symbol, as
    `batch_run.rs` does.
  - vCPU 0's monitor page is not at an address other vCPUs reach. Choose by experiment how a readying on one vCPU
    ends another vCPU's slice: a run-wide word every vCPU's slice check reads, a host request, or another form. State
    the choice with its grounds, and keep one vCPU's path unchanged.
- **(C)** vCPU 1's first take reads `RUN_END` 0, so its first slice ends early, never late. This is recorded, not
  fixed.

**The judge is too weak.** "Every count above 0" passed 20 of 20 before either fix. Strengthen it to a property today's
code fails, for instance that no thread runs more than a stated number of slices while another ready thread waits, or
a bound on the ratio of the threads' counts. It must fail on today's code over 20 runs and pass after the fixes over 20.
Plant (A)'s fix and (B)'s fix each out in turn.

## result

- **The enumeration.** Every host write to a vCPU's monitor data page or state block in `crates/w2m_vm/src`, found by
  grep of the offsets and of `md +`, `monitor +`, `.state`. One line a word: writer and line, the vCPU it writes,
  whether its right value depends on run-wide state.
  - limit `md + 8`: `run.rs` 993, every vCPU at the run's start, the run's budget (a child's a slice); run-wide (the
    link). `threads.rs` 1391 `set_budget`, the served vCPU; run-wide (`shared`: the threads, the link). Callers:
    `create_thread` (1372), `host_request.rs` 545, `run.rs` 1358 (`take_a_thread`, Sprint 351) and 1877 (`hvc #4`).
  - `MXCSR_OFF`, `FCW_OFF`: `run.rs` 994, 998, every vCPU at the start; `threads.rs` 1121 (`load`, the per-thread
    ranges 0x20-0x40, 0x50-0x58, 0x70-0x72, 0x400-0x500), the served vCPU; `model_step.rs` 149, 180-184, 187 (MXCSR,
    lazy AF/PF, DF, FCW, YMM upper halves), the served vCPU. The loaded thread's own state; not run-wide.
  - `LOCK_OWNER`: `run.rs` 997, every vCPU at the start, its index + 1; not run-wide.
  - `SLICE_UNTIL`: `run.rs` 1464, the turning vCPU at its turn's start, from `threads::slice_until`; run-wide (other
    threads' status, suspension, holder, deadlines, `RT_WAITING`, the clock's warp). Since this card also
    `threads.rs` 235 (`readied`), every other vCPU holding a thread. Stale: (A) and (B) below.
  - `RUN_END`: `run.rs` 1465, the turning vCPU at its turn's start, `max_steps`; a run-wide constant. (C) below.
  - `CURRENT`, `CURRENT_ENTRY`: `threads.rs` 177-178 (`add_thread`, the first thread, the served vCPU, vCPU 0 before
    any run) and 1117-1118 (`load`), the served vCPU; the vCPU's own thread; not run-wide.
  - `SLICE_COUNT`, `SLICE_PAUSES`: `threads.rs` 1082-1083 (`save`) and 1125-1126 (`load`), the served vCPU; its own
    count and pauses; not run-wide.
  - The count, `md + 0`: `run.rs` 114 (`exit_count`) and 596 (`give_back`), the exiting vCPU; its own.
  - The state block: `threads.rs` 1124 (`load`, the whole block), `run.rs` 1738 (`STATE_SCHEDULE` 0 at `hvc #12`),
    `model_step.rs` 151-156 (x87 status, tags, stack), each the served vCPU's; the thread's own.
  - `CLOCK_MODE`, `CLOCK_RATE`, `CLOCK_EPOCH`, `START_MODE`, `FOREGROUND_MODE`: `process.rs` 287-294, vCPU 0's page
    at the load, copied to vCPU k's by `Guest::private_chain` before any run; run-wide constants, set once.
  - The case runner's (`cases.rs` 386-390, 519-537): its own one-vCPU harness, no guest run.
  - The C runtime's own writes, each to its own vCPU's page: `threads.c` `readied()` (`SLICE_UNTIL` 0), `w2m_switch`
    (limit, count, `SLICE_COUNT`, `SLICE_PAUSES`, `CURRENT`, `CURRENT_ENTRY`, the per-thread ranges and state,
    `SLICE_UNTIL`), and the slice stub (`guest.rs` 455-481: the limit, at each TIME_SLICE while the count is below
    `SLICE_UNTIL`).
- **The two cases checked first.**
  - `SLICE_UNTIL`: stale, (A) and (B).
  - `set_budget` when `shared` turns true while another vCPU is mid-slice on the whole budget: not reached since
    Sprint 351. `shared` turns true at a second thread's `add_thread` or at a link's making (refused on two vCPUs,
    `host_request.rs` 530-531), and never turns back (`threads` keeps ended threads). When it turns true the
    creating thread is the run's only one, on the serving vCPU, which `create_thread` then budgets. Every other vCPU
    holds no thread until `take_a_thread`, which budgets it. Measured, with a temporary print in `take_a_thread`:
    vCPU 1, parked, held the whole budget, "take k=1 taken=Ok(false) ... limit=1099511627776", then took its thread
    with a slice. Plant 1 below reads that call's reach.
- **(A) stale, the host.** A thread made by vCPU 0's service left vCPU 1's `SLICE_UNTIL` as vCPU 1's last turn
  computed it. Temporary prints at each turn's start, removed after, showed both vCPUs' words.
  - After the second and third threads' creation: "turn k=0 cur=4 su=0x0 ... waiting=0 counts=[12413, 0, 0]
    other_su=0xffffffffffffffff".
  - vCPU 1's first thread then ran unsliced: "turn k=1 cur=8 su=0xffffffffffffffff limit=32360000 count=32342801".
  - Fix: `threads.rs` `readied`, called by `add_thread`, `set_status` to READY, `save` of a ready thread and a
    resume to 0, writes 0 into the `SLICE_UNTIL` of each other vCPU that holds a thread, when more threads are ready,
    unsuspended and held by none than vCPUs wait (`slice_until`'s rule). It reaches that vCPU's page by `monitor_of`,
    held to `private_chain` by the unit test.
  - The served vCPU's own word stays its turn's start's, so one vCPU writes nothing new.
- **(B) stale, the C runtime.** `readied()` and `w2m_switch` write only their own vCPU's `SLICE_UNTIL`.
  - vCPU 1's switches in the VM put the first thread down ready while vCPU 0 kept ~0: "turn k=1 cur=16 su=0x0 ...
    other_su=0xffffffffffffffff".
  - The second thread then ran alone on vCPU 0 for the whole run: before any fix, counts [7.54M, 20.3M to 28.0M,
    4.00M] in 20 of 20 runs of the first form of the guest.
  - The form chosen: `slice_until` answers 0 while another vCPU holds a thread, in `threads.c` and alike in
    `threads.rs`. A vCPU running while another holds a thread so ends its slice at each TIME_SLICE, and its switch
    in the VM looks again; with nothing else free it takes its own thread back, with no host exit.
  - Grounds: a run-wide word every slice check reads needs the slice stub changed. The stub's address is in every
    cached block and the cache's key (`guest.rs` 1079-1081), and a new define would change `python_image.rs`' frozen
    key. A host request costs a host exit for each readying and each put-down in the VM. vCPU 0's page is at no
    address another vCPU reaches. On one vCPU no thread is held elsewhere, so its path is unchanged.
  - The asymmetry with (A), reviewer-4's twenty-third review, Low 1: the host ends another vCPU's slice only for a
    readying no waiting vCPU will take, since it sees each readying it makes. The C rule ends it whenever another
    vCPU holds a thread, ready threads or not, since a readying in the VM writes no other vCPU's word. Both comments
    name the other.
  - Sprint 321's unit test `a_slice_ends_only_for_a_ready_thread_no_vcpu_will_take` now expects 0 for a thread held
    by another vCPU.
- **(C) recorded, not fixed (the amendment).** vCPU k's page is copied from vCPU 0's before any turn's start, so
  `take_a_thread`'s `set_budget` read `RUN_END` 0: "take k=1 taken=Ok(true) run_end=0 limit=0 count=0". The stub
  raised the limit at the first block ("limit=20000 count=7"), so the first slice ends early, never late.
- **The cost of (B).** `two_counters`, a scratch guest not committed: two threads on two vCPUs each count to
  300,000,000, the main thread waiting with no timeout, five runs each, one at a time.
  - Today's `threads.rs` and `threads.c`: 149,638,326 to 153,964,835 ticks of 100 ns (median 150,672,742).
  - The fixes: 155,665,372 to 165,960,372 (median 162,116,541), 7.6% more at the median.
- **The switches.** `three_counters` on one vCPU: 44,802 switches in the VM (`RT_SWITCHES`, `threads.c`'s
  `switches`), 3 `hvc #12`. On two vCPUs, 20 runs: 41,988 to 42,028 in the VM, 3 or 4 `hvc #12`.
- **The judge.** `tests/data/timed_waits/three_counters` (built with the neighbours' flags, GCC 16.2.0).
  - The main thread makes the first thread and spins until it counts, so the first runs on vCPU 1. It makes the
    second and third while the first runs there, then waits with no timeout.
  - The third names the first's count 4,000,000 ahead and waits on an event, which the first sets from its own vCPU.
  - The window runs from that set to the third's count of 40,000,000, all three ready throughout.
  - `run_loop::three_busy_threads_on_two_vcpus_each_count_and_the_run_ends` runs it once on one vCPU, then 20 times
    on two. Each two-vCPU run is held to ExitProcess inside 60 s, "done" and every count above 0. Then every window
    must keep its largest count within `SHARE_RATIO`, 1.5, of its smallest.
  - Its held line, `guest three_counters ExitProcess { code: 0 } ExitProcess done\n`, is in
    `store/batch_run/first-stop-2026-10-28.tsv`.
- **The counts, 20 runs on two vCPUs each.**
  - Before any fix (`threads.rs`, `threads.c` as at HEAD): every run ended, windows such as [40000539, 80607543,
    40000000], ratios 2.011 to 2.018. The judge failed: "three_counters on 2 vCPUs, run 1: window [40000539,
    80607543, 40000000], a thread ran 2.015 times another's, over 1.5".
  - After the fixes: ratios 1.000 to 1.018, windows such as [40009720, 39989177, 40000000]; pass. The final run:
    1.000 to 1.001.
- **Plant 1** "three_counters on 2 vCPUs, run 1: window [80479564, 40002579, 40000000], a thread ran 2.012 times
  another's, over 1.5": `take_a_thread`'s `crate::threads::set_budget(held.guest);` (`run.rs` 1358) made `let _ =
  &held; // plant 1`, one line for one, `sed`, on the fixed tree. The judge ran its 20 runs, all ending, ratios
  2.012 to 2.017, exit 100. `run.rs` restored by `cp`, `cmp` 0.
  - Its reach was first read on today's code with the guest's first form: "assertion `left == right` failed:
    three_counters on 2 vCPUs, run 1: counts [747298337, 1290952445, 0], output \"\"", stop TimeLimit.
- **Plant 2** "assertion `left == right` failed: a thread made on vCPU 0: vCPU 1's slice and vCPU 0's", left
  (18446744073709551615, 18446744073709551615), right (0, 18446744073709551615). (A)'s fix out: `readied`'s
  `write_u64(mem, md + SLICE_UNTIL, 0);` (`threads.rs` 229 then) made `let _ = md; // plant A`, `sed`.
  `threads::tests::a_thread_the_host_readies_ends_the_slice_of_another_vcpu_that_holds_one` failed, exit 100.
  Restored by `cp`, `cmp` 0. The guest's judge does not reach (A) alone, since (B) ends the same slice within one
  TIME_SLICE.
- **Plant 3** "three_counters on 2 vCPUs, run 1: window [40002662, 80622492, 40000000], a thread ran 2.016 times
  another's, over 1.5": (B)'s fix out.
  - `threads.c`' `if (elsewhere) return 0;` made `(void)elsewhere; /* plant B */`.
  - `threads.rs`' `if guest.served.threads.values().any(...)` made `if false && ...`.
  - Each one line, `sed`. The judge's 20 runs ended, ratios 2.013 to 2.021, exit 100. Both restored by `cp`,
    `cmp` 0.
- **The gates:**
  - The first run failed at 14:17Z on `guest_runtime::python_image::the_working_trees_c_departs_from_the_frozen_c_only_where_named`: `threads.c` was not in `DEPARTURES`. It is named there now with this sprint's reason, and that test passed alone.
  - The second run passed ("gates: steps 395 s of the run's 400 s"): `exit 0 in 400 s	default	2026-11-26T14:24:46Z	sprint 353	97bb2578afb87169f562cd443f33aef419ec8233	f5872d6fbe7eb238837a075c5a38f9279e7acfe114ec18b9df4ff8f86643a6da	root=209a55c1f99321b742b06abdd9757ae5fd9eb9e9	rust=all python=0 samples=0	base=fee00ad5504e13903b0784bc926461b28c090a23`
