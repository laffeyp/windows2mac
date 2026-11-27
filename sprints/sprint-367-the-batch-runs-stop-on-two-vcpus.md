# Sprint 367 — The batch run's stop on two vCPUs, on the wall clock

```yaml
---
id: 367
status: done
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Step 6's order after G2, weighed again at Sprint 366" puts the batch run's stop on two vCPUs first, so
that G3 (Sprint 368: the game's batch run on two vCPUs, several runs, conforming as on one) can judge C2, C5 and C3
before they are built. Today the batch run cannot run on two vCPUs: `w2m-vm --game` takes the instruction clock
(`crates/w2m_vm/src/bin/w2m-vm.rs` 345-348, 439-441: "The game's start keeps 1"), which the run loop refuses above one
vCPU since Sprint 365; and the batch run's stop (`crates/w2m_vm/src/batch_stop.rs` 1-17) is "points on the
instruction clock": it is asked after each host request, and its fallback `REACH_AFTER_LOG` counts instructions since
the game's log reached the reference's 168 lines.

- **The game on two vCPUs.** `w2m-vm --game ... --vcpus 2` runs the batch run on the wall clock, as a family's start
  does above one vCPU (Sprint 360), naming the fallback on stderr; on one vCPU it keeps the instruction clock and its
  stop exactly as today.
- **The stop by the reference's own rule.** On several vCPUs the stop keeps `run_frontier.py`'s `stop_when` (the log
  at the reference's line count and the main thread's calls), asked after each host request whichever vCPU serves it,
  its state read and written under the run's lock. State what `since_log` and `REACH_AFTER_LOG` count on several vCPUs
  (the run's instructions summed over its vCPUs, the main thread's alone, or another measure), with grounds from the
  frozen host's `stop_when` (`run_frontier.py` 284-310) and `batch_stop.rs`' own comment, and how a main thread that
  spins after the log fills is still bounded (reviewer-3's Low 3 of its review of Sprint 219, as `batch_stop.rs` 11-13
  quotes it).
- **The class first.** List every reader of `batch_stop`, of `start.instruction_clock` on the game's path, and of the
  game's single-vCPU assumptions (`w2m-vm.rs`' `game`, `batch_run.rs`, `batch_record.rs`, the exit ledger, the
  translation cache's setup, `call_compare`), with file:line and what each does on two vCPUs after this card. Report
  each defect found to main at once with its evidence; fix any that can stop a run or leave a thread past its deadline
  here.
- **Not here:** G3's several runs and their conformance (Sprint 368). One two-vCPU run reaching the stop is this
  card's; whether its call stream conforms is the next card's, read under the user's Decision of 2026-10-13.

## prerequisites

- 366 (closed)

## context_files

- `crates/w2m_vm/src/batch_stop.rs`: in full.
- `crates/w2m_vm/src/bin/w2m-vm.rs` 280-480 (the options, the family start's fallback, `game`), `crates/w2m_vm/src/run.rs`
  (`instruction_clock_allowed`, `run`, the stop's call at 2019-2022, the run's instruction count),
  `crates/w2m_vm/src/host_request.rs` (where a request's serve asks the stop).
- `crates/w2m_vm/tests/batch_run.rs`, `crates/w2m_vm/tests/batch_record.rs`, `store/batch_run/first-stop-2026-10-28.tsv`,
  `store/batch_run/main-thread-calls-2026-10-29.tsv`, and `scripts/gates.sh` 155-200 (the batch run's steps).
- The frozen host's `scripts/run_frontier.py` 60-80 and 284-310 (`REACH_AFTER_LOG`, `stop_when`), read as source from git: `git show 72e0bab6a^:scripts/run_frontier.py` (Sprint 226 took the Python out of the tree).
- `grep -n "Step 6's order after G2, weighed again at Sprint 366\|The instruction clock on several vCPUs\|The child on the run's vCPUs" BLACKBOARD.md`:
  the Decisions.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None: where the batch run stops on two vCPUs; no emit site changes. The one-vCPU batch run's records are unchanged.

### Invariants

- One vCPU unchanged: the batch run stopping where `store/batch_run/first-stop-2026-10-28.tsv` holds it today, on the
  instruction clock; the 273 family runs on one vCPU and two as held; the two-process guests; the exit ledger.

## artifact contract

### Files modified

- `crates/w2m_vm/src/batch_stop.rs`, `crates/w2m_vm/src/bin/w2m-vm.rs`, with others only where the class list
  requires, reported to main when found. One concept: the batch run's stop on several vCPUs.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The class list.
  - The batch run on one vCPU stopping at its held row, quoted.
  - The batch run on two vCPUs on the wall clock (`w2m-vm --game ... --vcpus 2`) reaching `Stop::ReferenceEnd` by the
    reference's rule in at least one run, its stop line and its time quoted, every process on two vCPUs.
  - The stop's decision unit-tested on several vCPUs' counts (the `decide` form), quoted.
- **Planted faults:**
  - (1) Today's form: the game's start refused above one vCPU, or kept on the instruction clock, so the two-vCPU run is
    refused by name, and the two-vCPU stop test fails.
  - (2) The stop asked on one vCPU's requests alone, or its fallback counting one vCPU's instructions, as the class
    list shows the fault the two-vCPU run would meet; its judge names it. If a judge catches a plant only sometimes,
    report that to main before going on.

## done criteria

The game's batch run runs on two vCPUs on the wall clock and stops by the reference's rule; on one vCPU it stops where
it stops today.

## self-review checklist

- [x] Scope names one verifiable property: the batch run's stop on two vCPUs, one vCPU unchanged.
- [x] context_files cite the stop, the game's start, the run loop, the batch tests and the frozen host's rule.
- [x] Emits: none, with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the one-vCPU stop, a two-vCPU stop, the decision's test; numbered plants.
- [x] One concept: the batch run's stop on several vCPUs.

## result

**The class**, each reader and what it does on two vCPUs after this card:

- `batch_stop` (`crates/w2m_vm/src/batch_stop.rs`): read at `run.rs` 2020-2030 inside the request's service, under
  the run's lock (`lock.serve`) and the requester's runtime word, at every request any vCPU answers. Two defects, both
  fixed here because either can stop a run. (a) The fallback took the asking vCPU's own count (`run.rs` 2021,
  `read_u64(md)`), and `executed - self.log_end` (old 126) wrapped when another vCPU set `log_end`; with release
  `overflow-checks = true` (`Cargo.toml` 26) that panics the vCPU thread. It now takes the run's count, every vCPU's
  summed (`vcpu_counts` 90, `run_instructions` 75), saturating (`since_log` 70). (b) A pending trace index whose slot
  `call_trace.c` 131-134 gives back, read again at or past the count (`guest.rs` 1622-1625), stopped the run with
  `Stop::Exit` "the batch run's end: trace record k: the trace holds k". Such an index is now dropped (186).
- `start.instruction_clock` on the game's path: `w2m-vm.rs` `game_clock` 729 gives the instruction clock on one vCPU
  and none above one, refused as any run is (`vcpus_allowed`); `game` 1091-1095 takes it and names the fallback on
  stderr; `serve` 346 asks it for `--game`. `--log-lines` was refused above one vCPU as a diagnostic (Sprint 308, old
  340). It no longer is (339): it reads at an answered request, not at every exit in order. `run.rs` 1203 still
  refuses the clock above one for every caller. `process.rs` 282-292 sets clock mode 0 (the counter) without it, and
  `guest.rs` 1062 seeds no generator: HOST_RANDOM is the host's CSPRNG on two vCPUs.
- The game's record (`record.rs`): `Execution.vcpus` 196 is written on EXECUTION_STARTED 311 and on each
  MEASUREMENT_RECORDED 390 above one vCPU, and is absent on one, as before. Until this card it was `vcpus: None`
  always, a false record on two. The stop file gets a `vcpus` row above one (`w2m-vm.rs` 1163), and `batch_record.rs`
  holds every event's `vcpus` to it (227, 408), leaving `vcpus` out of the comparison with the one-vCPU reference
  (183).
- `batch_run.rs` and `batch_record.rs`'s own run: both read the gates' run, which takes no `--vcpus` (`harness/w2m-vm-game.toml`), so on one vCPU nothing changes.
- The exit ledger and the block judge: still refused above one vCPU by name (`--exit-ledger`, `--judge-blocks`, 339).
- The translation cache's setup (`--make-cache`, `--cache`, `compile.rs` `use_cache` 192, `guest.rs` 1141): placed
  before any vCPU is made. This card does not run it on two vCPUs.
- `call_compare` (`main-thread-calls`): derived from the Windows capture alone, so the vCPU count does not touch it.
  Whether the two-vCPU call stream conforms is card 368's.
- `w2m-vm.rs` 1247 `current_tid` and 1262 `called_service`: both read `guest.served.vcpu` after the run, which is the
  last vCPU served and not always the stopper. On two vCPUs an Unserved stop's SERVICE_UNIMPLEMENTED may name another
  thread. ReferenceEnd does not read them (`record.rs` 232). This defect goes to card 368.
- `signal_report` (`crates/signal_report/src/main.rs`, reviewer-4's thirty-eighth review's follow-up, Low 1): the walk's
  amendment branch never read `BETWEEN_SPRINTS`, so b358c5588 (BLACKBOARD.md, a design and store/gates/ beside card
  367) would have been named at this close. It now passes a named commit (`named_between_sprints` 1516,
  `amendment_faults` 1522, the walk 1608), and b358c5588 stands there (1509) with its reason.

**The stop on several vCPUs.** `since_log` and `REACH_AFTER_LOG` count the run's instructions summed over its vCPUs.
That is the frozen host's measure: `stop_when` reads `vm.instructions_executed()`, which is "its vCPUs' counts" summed
(`vm_backend.py` 4125-4127 at 72e0bab6a^), and every thread's instructions on one vCPU. It is also the record's
`instructions_retired` (`run.rs` 1355). A main thread that spins after the log fills is bounded by the other threads:
their requests ask the stop, and the sum counts their vCPUs' instructions. Only a run in which no thread asks anything
goes to the step budget or the 10,800 s time limit, as on one vCPU.

**Observed.** The one-vCPU batch run stops at its held row, `stop	ReferenceEnd: the log holds the reference's 168
lines and the main thread its calls` (`store/batch_run/first-stop-2026-10-28.tsv` line 2), held by `batch_run.rs` in
the gates' run below. The two-vCPU run, `echo '{}' | scripts/sign_and_run.sh target/release/w2m-vm --game
~/w2m-depots/hail-to-the-rainbow "Hail to the Rainbow.exe" --output … --stop … --vcpus 2 --log-lines 168
--main-thread-calls store/batch_run/main-thread-calls-2026-10-29.tsv` on the final binary, ran 05:11:08-05:14:03 UTC.
Its stop file reads `stop	ReferenceEnd: the log holds the reference's 168 lines and the main thread its calls`,
`seconds	175.1`, `vcpus	2`, `calls traced	10480`, and its log holds 168 lines. Its stderr reads "w2m-vm: the batch
run's instruction clock runs on one vCPU; on 2 vCPUs, the wall clock" and "w2m-vm: process 4096 on 2 vCPUs, runs by
vCPU [211220, 111038]". All 55 of its record's EXECUTION_STARTED and MEASUREMENT_RECORDED events carry `"vcpus": 2`,
and `instructions_retired` is 77759395693. An earlier run on the binary before the record's change ran 05:05:38-05:08:33
UTC and stopped on the same row ("seconds 175.0", 10474 calls). In each run the child, started `--child 3 4 0` with
its parent's `vcpus` 2 in its start line (`host_request.rs` 242), was ended by the request server at the run's end
before it wrote its own vCPU line. `pgrep -fl w2m-vm` found nothing left after either run.

**The decision tested.** `the_bound_counts_every_vcpus_instructions`: counts [11,000,000,000, 11,000,000,000] with
`log_end` 2,000,000,000 give `since_log` 20,000,000,000. `decide` then returns "the log holds the reference's 168
lines; the main thread short of 1 functions' calls 20000000000 instructions later", and the first vCPU's count alone
gives None. `the_stop_asked_on_one_vcpu_counts_every_vcpus_instructions` runs `BatchStop::check` asked on vCPU 1 of
two and ends at the bound's sum. `a_record_given_back_is_not_read_again`, run with the filter taken out, failed with
`left: Err("trace record 1: the trace holds 1")`. The game's clock is tested by
`the_batch_run_on_two_vcpus_takes_the_wall_clock`, the record by `a_run_on_two_vcpus_says_so_on_its_record` and
`a_two_vcpu_record_carries_its_count`, and the amendment rule by
`a_named_card_amendment_passes_and_an_unnamed_one_is_named`.

**Plants**, backups under
`/private/tmp/claude-501/-Users-peterlaffey-port/18efb1a6-9978-416c-904b-05e124030429/scratchpad/plants/367/`:

- **Plant 1** "w2m-vm: 2 vCPUs refused: the instruction clock reads the running vCPU's own count", w2m-vm exit 1; and
  "assertion `left == right` failed: two vCPUs: the wall clock / left: Err(\"2 vCPUs refused: the instruction clock
  reads the running vCPU's own count\") / right: Ok(None)", `cargo nextest run -p w2m_vm -E
  'test(the_batch_run_on_two_vcpus)'` exit 100; live 05:14:25-05:14:50 UTC:
  - `w2m-vm.rs` backed up as `w2m-vm.rs.plant1`. `game_clock`'s clock made `Some((CLOCK_RATE, REFERENCE_START))` by
    the Edit tool, which keeps the game on the instruction clock; grep found it at line 730.
  - Release build, then the two-vCPU run refused by name before any VM. The bin test failed.
  - Restored by `cp`, checked by `cmp`, rebuilt; the test passed.
- **Plant 2** "assertion `left == right` failed: every vCPU's count, asked on vCPU 1 / left: [11000000000] / right:
  [11000000000, 11000000000]", `cargo nextest run -p w2m_vm -E 'test(batch_stop::)'` exit 100; live 05:15:32-05:15:51
  UTC:
  - `batch_stop.rs` backed up as `batch_stop.rs.plant2`. `vcpu_counts` made `vec![mem.read_u64(guest.served.vcpu.monitor)]`
    by the Edit tool: the fallback counting the asking vCPU's instructions alone, the form the class list shows the
    two-vCPU run met; grep found it at line 92.
  - `the_stop_asked_on_one_vcpu_counts_every_vcpus_instructions` failed, and the other five passed.
  - Restored by `cp`, checked by `cmp`; 6 passed. A first run of this plant (05:15:05-05:15:22) failed at the same
    assertion before it had a message, and the message was added.
- **Plant 3** "assertion `left == right` failed / left: [\"sprint 367: commit b358c5588 \\\"Sprint 367 card amendment:
  every C2 card built\\\" amends its card and changes BLACKBOARD.md, outside card 367 and probes/\", …] / right: []",
  `cargo nextest run -p signal_report -E 'test(a_named_card_amendment)'` exit 100; live 05:16:02-05:16:23 UTC:
  - `main.rs` backed up as `signal_report-main.rs.plant3`. The b358c5588 entry removed from `BETWEEN_SPRINTS` by the
    Edit tool; grep counted 0 entries.
  - The unit test failed. `signal_report 367 --check` exited 0 under the plant: the walk runs only from a closed tail
    entry (`main.rs` 931-935), so it names the amendment at the close.
  - Restored by `cp`, checked by `cmp`, rebuilt.

**Gates:** `exit 0 in 424 s	default	2026-11-27T05:25:14Z	sprint 367	b358c5588f91e2681c88ebd0ec9d0fcbca08c1fd	52cee7996cc4065eb70c00ccf81a68c3af53fa186769ee033731e79b35c408d5	root=c0d4479d202095338f7fe86de0f65955dee06a8f	rust=crates python=0 samples=0	base=65c95296383acf3e998501e3e73be58ed62fcb80`; nextest 317 tests run, 317 passed; the gates' one-vCPU `build/batch_run/first-stop.tsv` line 1 `stop	ReferenceEnd: the log holds the reference's 168 lines and the main thread its calls`.
