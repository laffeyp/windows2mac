# Sprint 360 — The instruction clock on two vCPUs

```yaml
---
id: 360
status: done
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Step 6's order weighed again at Sprint 360, G2 unmet" puts this card first. G2, the 68 families on two
vCPUs, stands at 267 of 273 runs conforming. Two of the six held runs, `processes` 38189162864 and 38189164581
(`store/family_runs/two-vcpus-refused-2026-11-26.tsv`), take the instruction clock. `vcpus_allowed` (`run.rs` 653-689)
refuses them: "the instruction clock reads the running vCPU's own count". `process.c` 41-46 computes the clock in clock
mode 2 as the instructions this process ran, at a set rate, plus the idle warp, from `monitor_word(0)`, the running
vCPU's own count. The per-vCPU pages design left it open: the runs that take it "stay on one vCPU until a card decides
how the counts combine" (`docs/design-each-vcpus-own-pages-2026-10-31.md` 106-109). This card decides that and builds
it.

- **The decision, first, written into the card's result before any code:**
  - **What the process's instruction count is on N vCPUs.** For instance, the sum of every vCPU's retired
    instructions: each vCPU publishes its count, and a reading adds the others' last published counts to its own.
  - **How a reading stays a clock.** Windows' QueryPerformanceCounter is monotonic and consistent across processors.
    Look it up: Microsoft Learn, "Acquiring high-resolution time stamps", and QueryPerformanceCounter's page; quote
    what they say. A reading on any vCPU must therefore never be below one any thread saw before. Say how the design
    ensures it, for instance a process-wide floor that each reading raises by an atomic max.
  - **The idle warp** (`W2M_CLOCK_WARP_VA`), and the deadline wakes that read the clock (`wake_expired`, the host's
    `warp_to_deadline` and `timed_waiter`): on two vCPUs, what each reads and writes.
  - **What one vCPU keeps:** the clock on one vCPU is bit for bit today's, so the batch run, the reference runs and every
    one-vCPU run are unchanged. The sum of one vCPU's count is its count.
  - Name each choice's grounds, and what Windows defines against what w2m chooses.
- **The build.**
  - The clock's C and the host's readers of the count as the decision needs.
  - `vcpus_allowed`'s instruction-clock refusal removed.
  - The two `processes` runs removed from `two-vcpus-refused-2026-11-26.tsv` and held to Windows' rows on two vCPUs in
    the gates' two-vCPU family runs (`scripts/gates.sh` `family_runs 2`), as Sprint 351 held the 267.
- **The class.** List every reader and writer of the instruction count and the warp, in the C and the host, one line
  each, with what it does on two vCPUs. That includes the slice and deadline arithmetic that turns a deadline into a
  count (`threads.c` `slice_until`, `threads.rs`' `slice_until`). Report each defect found to main at once, with its
  evidence.
- **Not here:** the batch run's stop on two vCPUs (its own card, later); a child process (card 4f).

## prerequisites

- 359 (closed)

## context_files

- `tools/guest_runtime/process.c` in full; `threads.c` (`slice_until`, `wake_expired`, the switch's warp); `runtime.h`
  (the clock's words).
- `crates/w2m_vm/src/run.rs` 640-690 (`vcpus_allowed`) and every host reader of the count and the warp (`grep -n
  'CLOCK_WARP\|CLOCK_RATE\|clock_mode\|instruction_clock\|warp_to_deadline' crates/w2m_vm/src/*.rs`).
- `docs/design-each-vcpus-own-pages-2026-10-31.md` 100-125.
- `crates/w2m_vm/tests/family_runs.rs` and `store/family_runs/two-vcpus-refused-2026-11-26.tsv`; `scripts/gates.sh`
  (`family_runs`).
- `grep -n "Step 6's order weighed again at Sprint 360" BLACKBOARD.md`: the Decision.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None: the clock's readings are the guest's own results, judged in the family runs' rows; no emit site changes.

### Invariants

- On one vCPU the clock is bit for bit today's: the batch run, the reference runs, the exit ledger and every family
  run pass unchanged.
- The 267 two-vCPU family runs that conform today still conform.

## artifact contract

### Files modified

- `process.c`, `threads.c` and the host's clock readers as the decision needs, `run.rs` (`vcpus_allowed`), the refused
  list, the tests, and `DEPARTURES`. One concept: the instruction clock on several vCPUs.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The decision, with Microsoft Learn quoted.
  - The class list.
  - The two `processes` runs' rows on two vCPUs equal to Windows', in 20 runs each.
  - A two-vCPU judge of the clock's monotonicity: two threads, one on each vCPU, each reading QueryPerformanceCounter
    in a loop and recording the other's last reading. No reading is ever below one already seen, across 20 runs.
  - One vCPU's clock unchanged, by the batch run's held stops and the reference runs.
- **Planted faults:**
  - (1) A vCPU's reading from its own count alone, today's form. The monotonicity judge fails, a reading below one the
    other thread saw; or the `processes` rows differ.
  - (2) The floor (or whatever keeps readings monotone) removed. The monotonicity judge fails.
  - (3) The refusal restored. The two `processes` runs are refused again, and the gates' two-vCPU step names them.

## done criteria

The instruction clock on N vCPUs is one process-wide clock, monotonic across vCPUs as QueryPerformanceCounter is. The
two `processes` runs conform on two vCPUs, so 269 of 273 do, and one vCPU's clock is unchanged bit for bit.

## self-review checklist

- [x] Scope names one verifiable property: one monotonic process clock on N vCPUs; two held runs conforming.
- [x] context_files cite the clock's C, the host's readers, the design's open question, the family runs and the
      Decision.
- [x] Emits: none, with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the decision, the class list, the rows, a monotonicity judge; numbered plants.
- [x] One concept: the instruction clock on several vCPUs.

## amendment (2026-11-26)

Reviewer-4's thirty-second review, Medium 1, verified by the Supervisor. The scope's example model, "the sum of every
vCPU's retired instructions", is wrong, and it is ruled out. The instruction clock models a guest second as a fixed count
of instructions retired by one core: 1.4 billion, after QEMU's `-icount` (the Decision of 2026-10-16). Two vCPUs retire
in parallel, so under the sum the clock runs twice as fast whenever both are busy. A Sleep(100) would end after 50 ms of
each vCPU's progress, and the batch run's main thread, which paces itself by QueryPerformanceCounter, would see time run
fast whenever its workers run.

Before any code, the decision weighs three models, and the result says which is chosen and why:
- **(a) the maximum with a floor.** A reading is the larger of its own vCPU's count and a process-wide floor, and each
  reading raises the floor by an atomic max, so it never goes backward across vCPUs. A parked vCPU's count stalls while
  the running one advances, and the idle warp still covers every vCPU parked. On one vCPU the maximum of one count is
  that count.
- **(b) the instruction clock kept to one vCPU, as today's refusal does,** with the two `processes` runs held to
  Windows' rows on two vCPUs on the wall clock instead, the form G3 takes for the batch run.
- **(c) any other model** the lookups below support.

Two lookups settle it, each quoted in the result:
1. **QEMU's own answer.** What QEMU's documentation or source says about `-icount` with multi-threaded TCG: whether it
   runs icount across parallel vCPUs, or falls back to one thread taking the vCPUs in turn. The project already cites
   `docs/devel/multi-thread-tcg.rst`.
2. **Why the `processes` family takes the instruction clock.** Its rows in `store/family_runs/known-2026-10-29.tsv`
   (184-185) carry `{"instruction_clock":true}`. Find the card or Decision that set it (`grep` the family runs' history,
   the Decision of 2026-10-28 on the machine clock), and what the family needs from it: determinism across its
   processes, or a guest time that does not run with the host.

Under model (b), the two runs leave the refused list by running on the wall clock, and `vcpus_allowed` keeps its clock
refusal; the monotonicity judge then runs on the wall clock's QueryPerformanceCounter. Under model (a), add a judge: a
guest whose Sleep(100) with both vCPUs busy reads 100 ms of guest time within the clock's resolution, with the sum as
its plant.

## result

- **The decision: model (b), the instruction clock kept to one vCPU; a family's start on it runs on the wall clock
  above one.** Written before the code of (b). The first form built, before the amendment arrived (the card's first
  example: the sum of the vCPUs' counts, published counts and an atomic-max floor), was taken out whole; the amendment
  rules the sum out, and its diff is kept outside the tree.
  - *What Windows defines.* QueryPerformanceCounter "Retrieves the current value of the performance counter, which is a
    high resolution (<1µs) time stamp" (Microsoft Learn, QueryPerformanceCounter function, updated 2026-05-15).
    "Acquiring high-resolution time stamps" (updated 2025-12-13): "In general, the performance counter results are
    consistent across all processors in multi-core and multi-processor systems, even when measured on different threads
    or processes"; "Is the performance counter monotonic (non-decreasing)? Yes. QPC does not go backward."; "when
    comparing performance counter results that are acquired from different threads, values that differ by ± 1 tick have
    an ambiguous ordering"; and on Arm, "the Generic Timer System Counter is a fixed-frequency platform-wide time
    source". Windows defines one monotonic counter across processors, in wall time. The instruction clock, time as
    instructions retired, is w2m's choice for deterministic tests, not Windows'.
  - *Lookup 1, QEMU's answer.* `docs/devel/multi-thread-tcg.rst` (qemu master, 9751c6235): "System emulation will fall
    back to the original round robin approach if: * forced by --accel tcg,thread=single * enabling --icount mode".
    `docs/devel/tcg-icount.rst`: icount "is only available for system emulation and is incompatible with multi-threaded
    TCG ... It can also provides for a degree of deterministic execution". QEMU never runs an instruction count over
    parallel vCPUs; it takes them in turn on one thread, which is what w2m's one vCPU does with its threads.
  - *Lookup 2, why `processes` takes the instruction clock.* Commit 9a7515bb9 (2026-10-13), "A family may run on the
    instruction clock; processes does: its pipe.capacity row counted a writer's progress during a wall-clock Sleep(500)
    and fell to 1,816 and 3,634 bytes of Windows' 4,096 under load"; `scripts/family_run.py` at frozen-python-2026-10-29:
    "pipe.capacity counts a writer's one-byte writes during the main thread's Sleep(500): on the wall clock 1,816 and
    3,634 of Windows' 4,096 when the host was loaded (2026-10-13 sweep); on this clock the sleep ends only once the full
    pipe blocks the writer". The family needs a guest time that does not run with the host's load, which one vCPU's turn
    taking gives deterministically; on parallel vCPUs the interleaving is the host's under any count. Its parent and
    child share that clock by turns: the Decision of 2026-10-28 (the second process's shape) runs a machine's processes
    "one process running at a time" on "one clock for the machine (`_machine_now`, `_catch_up`, `warp_to_deadline`)",
    the same round robin.
  - *Weighed.* (a), the larger of a vCPU's own count and an atomic-max floor: a vCPU that starts or resumes behind the
    floor (vCPU 1 starts at count 0 while vCPU 0 has run the start) reads the floor and moves no clock until it has run
    as many instructions as the leader, so a Sleep on the parked vCPU waits on the other's catch-up; (a) needs a
    per-vCPU catch-up on top, and still gives no determinism. (b) has the project's grounds: the user's "the instruction
    clock stays the deterministic clock for tests" (2026-10-16), census G3 (`docs/step-6-census-2026-10-30.md` 20,
    "the batch run on the wall clock on two vCPUs conforming"), and QEMU's round robin. And the `processes` runs start
    a child process (below): under (b), on the wall clock above one vCPU, they need no clock card for G2, only card 4f;
    under (a) they would need both. Chosen: (b).
  - *Its cost, carried to card 4f.* Once 4f lets the two `processes` runs start on two vCPUs, they run on the wall
    clock, and their `pipe.capacity` row is the one the 2026-10-13 sweep found load-dependent (1,816 and 3,634 of
    Windows' 4,096). Card 4f, or G2's judge card, holds that row on two vCPUs by an allowed range or a quiet host, or
    the runs flake.
  - *What it is.* `vcpus_allowed` keeps its refusal of the instruction clock above one vCPU (`run.rs` 671-679, its
    comment now giving Sprint 360's grounds); `--instruction-clock` and `--game` asked above one vCPU are refused by name
    as before. A family's start that asks the clock (`--start {"instruction_clock":true}`) on more than one vCPU runs on
    the wall clock, `w2m-vm` saying so on stderr ("the family's instruction clock runs on one vCPU; on 2 vCPUs, the wall
    clock"; `w2m-vm.rs` 423-428). The warp and every deadline's arithmetic run only in clock mode 2, which no run of
    more than one vCPU reaches, so each is as it was. One vCPU: no C changed, so the runtime image is HEAD's byte for
    byte, and every one-vCPU run, the batch run and the reference runs among them, reads the clock as before.
- **The `processes` runs on two vCPUs.** Reported to main at once: they start a child process (`probes/processes/
  processes.c` 279 and 323, CreateProcessW), which `host_request.rs` 532-534 refuses on more than one vCPU
  (`MACHINE_ON_ONE`). Run on this tree, both, 38189162864 and 38189164581, stop on two vCPUs at "HostRequest op 33: a
  process that starts another runs on one vCPU until the Machine has its own thread (the run-loop design's card 4f)",
  and on one at ExitProcess, 38189162864's rows byte-equal to the gates' last. They are held to that refusal in
  `two-vcpus-refused-2026-11-26.tsv`; G2 stays at 267 of 273, all six held runs on card 4f.
- **The class: every reader and writer of the count (monitor word 0, each vCPU's own) and the warp
  (`RUN_TABLE + CLOCK_WARP`, shared), and what each does above one vCPU, where no run is in clock mode 2:**
  - C `process.c` 44-45 `counter_now`, mode 2: own count at the rate plus the warp. Unreached (mode 0, CNTVCT_EL0).
  - C `threads.c` 217-224 `wake_expired`: deadlines against `w2m_counter_now`, the wall clock's.
  - C `threads.c` 238-262 `slice_until`: 0 while another vCPU holds a thread; the deadline-to-count arithmetic
    (257-261) only in mode 2, so unreached; on another clock 0.
  - C `threads.c` 314-327, 350-352, 367 `w2m_switch`: the count's slice limit, give-back (`*count -=`), the thread's
    instructions and x23, each on the vCPU's own words, as on one vCPU.
  - C `threads.c` 335-342 `w2m_switch`'s warp: only in mode 2; unreached.
  - C `threads.c` 534-537 `slept`, `slim_locks.c` 142-144, `sync_objects.c` 779-1131, `system_info.c` 96, `time.c`
    24-36, `file.c` 502: readings of `w2m_counter_now` for deadlines and times, the wall clock's.
  - Monitor code `count_from_budget` (`guest.rs` 429, 617; `monitor_code.rs` 959) writes the count at each bridge,
    switch stub and exception; `budget_from_count` reads it back: per vCPU, unchanged.
  - Host `process.rs` 282-291: mode 2 and the warp's 0 written at the start, only with the instruction clock, which
    no run of more than one vCPU now has.
  - Host `threads.rs` 788-802 `counter_now`, 807-819 `catch_up`, 822-838 `wake_expired`, 842-864 `turn_state`,
    869-893 `warp_to_deadline`, 987-1008 `timed_waiter`, 748-760 `park_deadline`: on the wall clock `mach_absolute_time`
    in 100 ns units; the warp is written only in mode 2 (`warp_to_deadline` 871, `timed_waiter` 990, `catch_up` 808).
  - Host `threads.rs` 1405-1419 `set_budget`, 1085-1126 `save`, `run.rs` 636-651 `give_back`, 1517, 1748, 1756, 1911:
    the count per vCPU for the budget, slices and stops, unchanged.
  - Host `threads.rs` 1425-1488 `slice_until`: the deadline-to-count arithmetic (1474-1487) only in mode 2.
  - Host `run.rs` 1133, 1579: the exit ledger's rows read the count and the warp, a one-vCPU diagnostic
    (`vcpus_allowed` refuses `--exit-ledger` above one).
  - Host `guest.rs` 563 `clock_mode_at_load`, 969, 1398: a service's lock form by the clock mode at load.
  - The run loop itself (`run::run`) does not refuse the instruction clock above one vCPU; `w2m-vm`'s refusal and
    fallback are the two gates, and no in-process caller in the tree starts it on two.
  Defects found: one, the `processes` runs' child process (above). None in the class.
- **The monotonicity judge**, `tests/data/timed_waits/monotonic_clock` (new; built `x86_64-w64-mingw32-gcc -O1
  -nostdlib -Wl,-e,start -o monotonic_clock.exe monotonic_clock.c -lkernel32`, GCC 16.2.0), two threads each reading
  QueryPerformanceCounter 200,000 times, recording the other's last reading and counting a reading below the larger of
  it and its own; its figures in `.clock`, its output "done", or "below". `run_loop.rs`
  `the_wall_clock_never_reads_below_a_reading_taken_on_two_vcpus`, in-process: one run on one vCPU (below [0, 0],
  6,420,418 instructions) and 20 on two, each below [0, 0], runs by vCPU [14, 20] or [20, 12], each thread seeing the
  other's reading move 196,030 to 198,888 times (floor 1,000). `a_family_start_on_the_instruction_clock_reads_the_wall_
  clock_on_two_vcpus`, through `w2m-vm` with `--start {"instruction_clock":true} --vcpus 2`: 20 of 20 at ExitProcess,
  "done", stderr naming the wall clock. Its held stop on one vCPU: `store/batch_run/first-stop-2026-10-28.tsv`, guest
  `monotonic_clock`, "done".
- **One vCPU unchanged**: no file under `tools/` changed; the gates' batch run, guests, exit ledger and family runs
  judge it (the gates line below).
- **Plant 1** (the instruction clock let onto two vCPUs in today's form: `vcpus_allowed`'s `if clock {` made `if clock
  && false {` and `w2m-vm`'s fallback made `if false && ...`; 2026-11-26T23:14:13Z to 23:14:25Z):
  `a_family_start_on_the_instruction_clock_reads_the_wall_clock_on_two_vcpus` failed, "monotonic_clock through w2m-vm
  on 2 vCPUs: 20 of 20 runs failed", each "stop \"ExitProcess { code: 0 }\", output \"below\\n\"": a reading below one
  the other thread took, every run.
- **Plant 2** (`vcpus_allowed`'s refusal removed, `if clock && false {`; 23:14:31Z to 23:14:33Z):
  `run::tests::each_reason_against_a_second_vcpu_is_refused_by_name` failed, "panicked at crates/w2m_vm/src/run.rs:2124:49:
  allowed: ()", the instruction clock's case, the only call the plant lets through.
- **Plant 3** (`w2m-vm`'s wall-clock fallback removed, `if false && ...`; the family runs on one vCPU and two rerun from
  the planted build; 23:14:39Z to 23:15:49Z): `family_runs::every_family_run_conforms_or_is_held_by_what_it_waits_on`
  failed, "processes run 38189162864 on two vCPUs stops at \"2 vCPUs refused: the instruction clock reads the running
  vCPU's own count\", where store/family_runs/two-vcpus-refused-2026-11-26.tsv holds it to \"HostRequest op 33: a
  process that starts another runs on one vCPU until the Machine has its own thread (the run-loop design's card 4f)\"",
  and the same for 38189164581.
  Each plant was backed up by `cp`, its edit found by `grep` before its judge, restored by `cp` and checked by `cmp`.
- **Files changed:** `crates/w2m_vm/src/run.rs` (the refusal's grounds), `crates/w2m_vm/src/bin/w2m-vm.rs` (the
  family start's wall clock), `crates/w2m_vm/tests/run_loop.rs` (the two judges), `crates/w2m_vm/tests/family_runs.rs`
  (one refusal), `store/family_runs/two-vcpus-refused-2026-11-26.tsv`, `store/batch_run/first-stop-2026-10-28.tsv`,
  `tests/data/timed_waits/monotonic_clock.c` and `.exe` (new), `scripts/gates.sh` (a comment). No `DEPARTURES` entry:
  no C changed.

- **Gates:** `exit 0 in 442 s	all	2026-11-26T23:25:18Z	sprint 360	c08297a0a65dce684afca122c4206bb682158fdb	3e3ccd3edbde03de6653331dc2e124294619b9cf09790e3bdd97d94734506556	root=efbf0e3ca5b0f84444b0327a53f7e36767ff2bf6	rust=all python=0 samples=1	base=45043d2b0beb426e109faf1c84ac72ecdc7c5adf`
