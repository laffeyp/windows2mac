# Sprint 363 — The request server: what crosses processes, answered one request at a time under `kernel`

```yaml
---
id: 363
status: done
phase: 6
pass_kind: functional
---
```

## scope

Card 4f2 of `docs/design-a-process-that-starts-another-2026-11-26.md` ("The cards that follow", item 2; "The request
server", "Which lock or request each crossing takes"). The design is the spec. On several vCPUs no Machine runs, and a
thread in the first `w2m-vm` answers what crosses processes. This card builds that server and its client, judged by a
test of two processes, before any run reaches it (card 4f3 connects it).

- **A new file, `crates/w2m_vm/src/cross_process.rs`.**
  - The page's second layout, on the same anonymous 64 KB file `turns.rs` maps: a doorbell word the server waits on,
    then a slot per process. Each slot holds a request state (idle, posted, answered), four arguments, the result and
    second word, a message (request, ended, stopped), the exit code, the stop text, the maps posted once at the
    child's start, and a wake count.
  - `Server`: its thread, the doorbell wait (`os_sync_wait_on_address`, SHARED, `shared.rs`) with a 100 ms timeout,
    and at each timeout a look at each child's `try_wait`, as `shared.rs` 132-150 does. A request is served under
    `kernel` (`run::KernelLock`, Sprint 362) against a `ConcurrentGuestMemory` view of the child's mapping. The server
    writes the result and second word, then the state, answered, by a release store after the answer's words, and wakes
    the slot.
  - `Client`: post the arguments, mark the slot posted, ring the doorbell (`os_sync_wake_by_address_any`, SHARED), and
    wait on the slot's state.
- **`kernel`'s guard and the server.** Sprint 362's `KernelLock::take` asserts a runtime word. The server holds none:
  add its own take, named for the server, and keep the assertion for every other taker.
- **`kernel.rs`' `Memory`** gains the view, so a request reads and writes a child's buffers through
  `ConcurrentGuestMemory`, not `as_mut_slice` (the census row D7 rule the design cites).
- **The design's goal section repriced.** `docs/design-a-process-that-starts-another-2026-11-26.md` 359 prices a
  crossing at Change 4's "median of 0.58 to 0.71 µs". This Mac measured a round trip over SHARED words between two
  threads at a median of 6,292 ns, p99 12,750 ns (`shared.rs` 173-176, 2026-10-28). The section is restated at this
  card's measured median, with that figure beside it. Line 99 quotes Change 4 and stays as quoted.
- **Not here:** any run reaching the server; `MACHINE_ON_ONE` stays; the link thread and wake count's reader (4f3).

## prerequisites

- 362 (closed)

## context_files

- `docs/design-a-process-that-starts-another-2026-11-26.md`: in full.
- `crates/w2m_vm/src/shared.rs`: in full (the page, `post`, `wait_until`, `wait_until_alive`, and the tests at 177-280,
  the form for this card's two-process test).
- `crates/w2m_vm/src/turns.rs` 1-60 and 380-420 (the page's first layout, `ChildMemory`), `crates/w2m_vm/src/kernel.rs`
  (`Memory`, `Objects::request`), and `crates/w2m_vm/src/run.rs` 925-995 (`KernelLock`).
- `crates/w2m_vm/src/concurrent_memory.rs` (the view).
- `grep -n "The form of card 4f on several vCPUs\|The .kernel. lock, card 4f1\|Sequence after Sprint 196" BLACKBOARD.md`: the
  Decisions; and `grep -n "thirty-third review" BLACKBOARD.md`: the review whose Low 2 this card's gates take.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands".

## signal contract

### Emits

- None: a server no run reaches yet; no emit site changes. Every run's records are unchanged.

### Invariants

- The batch run conforming, the 273 family runs on one vCPU and two as held (the six still refused by name), the
  `twoproc` guests and the exit ledger, all unchanged.

## artifact contract

### Files created

- `crates/w2m_vm/src/cross_process.rs`

### Files modified

- `crates/w2m_vm/src/kernel.rs` (the view in `Memory`), `crates/w2m_vm/src/run.rs` (the server's take),
  `crates/w2m_vm/src/lib.rs` (the module), and the design's goal section.

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/design_check` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The two-process test, as `shared.rs` 211-250's form: the test runs its own binary as a second process. The
    second process makes 10,000 PIPE_WRITE requests of one to 100 bytes; a thread of the first makes the PIPE_READs
    under `kernel`. The bytes read equal the bytes written, in order.
  - The round trip's median and p99 over the 10,000 requests, printed and quoted. The gates hold the median under 1 ms,
    a gross failure only, since they run the test beside others (`shared.rs` 174-176). A run of the test alone, quoted
    in the result with its command, holds the median under 10 µs, the revisit line of the Decision "Sequence after
    Sprint 196".
- **Planted faults:**
  - (1) The state stored before the answer's words. The order check fails.
  - (2) The client's doorbell wake removed, so the server finds a request only at its 100 ms timeout. The gates' 1 ms
    median check fails.

## done criteria

A request server and its client, in `cross_process.rs`, answer a second process's requests one at a time under
`kernel`, through a view of that process's memory; 10,000 requests arrive whole and in order, at a median quoted and
held under 10 µs alone and under 1 ms in the gates; every run unchanged.

## self-review checklist

- [x] Scope names one verifiable property: the server answering a second process under `kernel`.
- [x] context_files cite the design, the transport, the lock and the measured figure.
- [x] Emits: none, with its reason.
- [x] Artifact contract gradable by command.
- [x] Observation contract: order, the measured median and p99, two numbered plants.
- [x] One concept: the request server.

## result

- **The page's second layout** (`crates/w2m_vm/src/cross_process.rs` 67-101): the turn form's 64 KB file
  (`turns::PAGE_SIZE`, `shared::anonymous_file`), word DOORBELL, then 7 slots of 1,024 words: STATE (IDLE, POSTED,
  ANSWERED), MESSAGE (REQUEST, ENDED, STOPPED), four ARGS, RESULT, SECOND, EXIT_CODE, WAKES, the stop text (its length,
  128 words) and the maps (their count, 256 triples of VA, IPA, size). Its words are reached as atomics through a view
  of the mapping (`Words`, 107-150), waited on with `os_sync_wait_on_address` and SHARED, the declarations quoted from
  this Mac's SDK header (31-47).
- **`Server`** (222-348): its thread, `w2m-request-server` (`start`, 231). It waits on the doorbell from the count read
  before it looked at the slots, so a request posted meanwhile ends the wait at once. The wait times out at 100 ms, and
  every 100 ms it looks at each child's `try_wait` (`serve_until_ended` 351, `look_at_children` 485). A child gone
  without its end or stop stops the run by name. `serve_posted` (371) takes `kernel` by the server's own take (378),
  then the children's table, the design's "the children's table is under it too", so a spawn under `kernel` can
  `adopt` (265). It serves a REQUEST through `kernel::MappedMemoryView` over a view of the child's mapping, an ENDED by
  `Objects::end`, a STOPPED into the run's stop. It writes RESULT and SECOND, then STATE ANSWERED by a release store
  (420), and wakes the slot. Before the answer's words sits a test seam (`AnswerPause`, 448, `#[cfg(test)]`, set by `Server::pause_before_answers`, 291): when a
  test sets it, every Nth answer waits 1 ms there; unset it does nothing. After a waking answer it raises every live child's WAKES and the first process's own
  `WakeCount` (155), an in-process count and condition. A refused request stops the run by name, its slot left
  unanswered.
- **`Client`** (502-600): `post_maps` once at the start; `request`, `end`, `stop`. Each is one `round_trip` (575):
  MESSAGE, STATE POSTED (release), the doorbell's count raised and `os_sync_wake_by_address_any`. It then looks at its
  state for ANSWER_SPIN, 20 µs (65), before it blocks on it.
- **`kernel`'s server take** (`run.rs` 980): `take_for_request_server` asks for a `cross_process::RequestServerThread`,
  a value only the server's thread makes (183, private field), and takes the lock with no word to assert. `take` (968)
  keeps its assertion for every other taker, unchanged.
- **The view in `kernel.rs`**: `MappedMemoryView` (80) implements `Memory` through `ConcurrentGuestMemory` by maps of
  (VA, offset, size). Bytes past a map's end or at no map are refused by name. Judge
  `a_request_reads_and_writes_through_a_view_by_its_maps` (563).
- **The two-process judge**, `cross_process::tests::ten_thousand_pipe_writes_from_another_process_arrive_whole_and_in_order`
  (654). It runs its own binary again as the second process (`W2M_CROSS_PROCESS_PEER`), which makes 10,000 PIPE_WRITE
  requests of 1 to 100 bytes through slot 0 (`write_the_stream`, 783), REQUEST_GAP, 50 µs (628), after each answer. The
  first process's test thread makes the PIPE_READs under a runtime word it takes and `kernel`'s `take`. The checks:
  bytes read equal bytes written in count and in order; p99 under P99_CEILING, 50 ms, half the server's 100 ms
  doorbell timeout (632, 770); median under 1 ms (773); median under 10 µs with `W2M_ROUND_TRIP_ALONE=1` (775), the
  performance check. The peer stops once more than 1% of the 10,000 reach the ceiling, which decides the p99. The
  same judge runs a second time with the pause seam set to 100 in process, as its own gated test,
  `cross_process::tests::an_answer_paused_before_its_words_is_read_only_once_its_words_are_written` (663, through
  `the_judge`, 669), so the gates hold the order of an answer's stores; the unpaused test's figures hold no pause. A 1 ms
  ceiling, tried first, failed the second gates run with no plant live: "median 35416 ns, p99 4479000 ns" over 1,636
  requests. That is what a cross-process request cost on this Mac under nextest's full load. The 50 ms ceiling sits
  eleven times over that p99 and half a lost wake's 102 ms. Run alone, five times,
  `W2M_ROUND_TRIP_ALONE=1 target/debug/deps/w2m_vm-f3571950fd24e700 --exact
  cross_process::tests::ten_thousand_pipe_writes_from_another_process_arrive_whole_and_in_order --nocapture`:
  "wrote 505000 bytes in 10000 PIPE_WRITE requests; round trip median 8458 ns, p99 13292 ns", then 8,625/12,625,
  8,708/12,959, 8,750/12,834, 8,584/12,416; and at 17:18 PST, load average 1.19, "median 8333 ns, p99 12667 ns"; on
  the file before the seam, 8,708/13,209, 8,791/13,000, 7,666/11,792; with the seam, unset, 8,708/13,167 and
  8,833/12,625; on the final file, 8,750/12,958. Each exit 0. With the seam set and the stores in order (first by an
  environment variable, since replaced), three runs passed: median 8,625, 8,625, 8,708 ns, p99 1,056,500, 1,046,208,
  1,292,125 ns; the gated paused test alone, median 8,625 ns, p99 1,093,000 ns. The pause shows in the p99 alone. In the gates: the passing run's row below; nextest keeps a passing test's output, so that run prints
  no figure. The one figure from inside the gates is the second run's, "median 35416 ns, p99 4479000 ns" over 1,636
  requests, which the 1 ms ceiling failed (above). `pgrep -fl w2m_vm-` found nothing left after every run.
- **The spin, measured.** The same command with ANSWER_SPIN set to 0 (the file copied, edited, measured, restored by
  `cp`, `cmp` silent): medians 10,250, 10,125, 10,417, 10,167, 9,875 ns, p99 17,084 to 18,875 ns. Four of five failed
  the 10 µs alone check. Back to back (no gap, before REQUEST_GAP was added): 9,125 to 9,458 ns without the spin,
  3,250 to 3,333 ns with it. The spin's grounds and cost are in its doc (65): parking_lot's `SpinWait::spin`
  (`core/src/spinwait.rs` 32-52 at 85827ffa2, fetched to the scratchpad) and std's futex mutex (`futex.rs` 35). The
  cost is up to 20 µs of the requesting core a request, 7.5 ms a second at Change 4's 376 a second.
- **The design repriced**: `docs/design-a-process-that-starts-another-2026-11-26.md` "Against the goal" prices a
  crossing at the measured 8,333 ns median, p99 12,667 ns, beside `shared.rs`' 6,292 ns and the no-spin and
  back-to-back figures. Line 99 is unchanged, as quoted. `target/release/design_check` exit 0.
- **Plant 1** "assertion `left == right` failed: the bytes read against the bytes written / left: 505000 / right:
  503216", the judge in the gates' form, `cargo nextest run -p w2m_vm -E
  'test(=cross_process::tests::an_answer_paused_before_its_words_is_read_only_once_its_words_are_written)'`, exit 100
  in 5 of 5 runs (right 503216, 503255, 503135, 503067, 503229); live 01:59:36-01:59:47 UTC:
  - `cross_process.rs` copied to the scratchpad; by the Edit tool, the STATE line, `store(ANSWERED,
    Ordering::Release)`, moved from after RESULT and SECOND to before them, the seam between: STATE, the pause, RESULT,
    SECOND. `sed -n 416,421p` showed that order.
  - Restored by `cp`; `cmp` silent.
  - Before the gated test, the same swap against the judge seeded by an environment variable, live 01:48:52-01:49:02
    UTC, failed 5 of 5 (right 503072, 503110, 503072, 503148, 503091), and with the seam unset passed 3 of 3: the
    unpaused test alone does not catch this plant.
  - Observations before the seam. The swap of the two adjacent lines alone, live 01:05:40-01:06:16 UTC, failed 1 of 3
    runs ("the bytes read against the bytes written"), a window of two stores. ANSWERED moved to before the request
    is served, a larger fault, failed 5 of 5 (live 01:18:14-01:18:28 UTC, right 504971, 504890, 505190, 505095,
    505128; and 01:31:19-01:31:25 UTC, right 504952, 504811, 505043, 505165, 505719).
- **Plant 2** "p99 round trip 102031209 ns, median 101829708 ns", the judge in the gates' form, `cargo nextest run -p
  w2m_vm -E 'test(=cross_process::tests::ten_thousand_pipe_writes_from_another_process_arrive_whole_and_in_order)'`,
  exit 100 in 10.28 s; live 02:00:59-02:01:14 UTC, on the final file:
  - `cross_process.rs` copied to the scratchpad; by the Edit tool, the client's `ring(&self.page)` in `round_trip`
    replaced by the doorbell's `fetch_add` alone, its wake removed; `grep -n` found it at 579.
  - Live also 01:49:15-01:49:27 UTC before the gated paused test: "p99 round trip 102027917 ns, median 101964917 ns",
    exit 101 (the peer: "wrote 5064 bytes in 101 PIPE_WRITE requests").
  - Live also 01:30:45-01:31:05 UTC, before the seam: "p99 round trip 102022666 ns, median 101439750 ns", exit 101.
  - Live also 01:17:20-01:18:06 UTC on the 1 ms ceiling: "p99 round trip 102023500 ns, median 101961959 ns", exit
    101; with REQUEST_GAP also set to 0 for one run, "p99 round trip 102050750 ns, median 4417 ns", exit 101 (3,099
    requests). Unpaced, the median alone would have passed.
  - A first run, 01:06:46-01:08:47 UTC, before REQUEST_GAP and the p99 check, passed: "round trip median 5208 ns, p99
    102094250 ns", exit 0, 116.16 s. The median hid a wake lost on about 1% of requests, so the p99 ceiling and the
    gap were added.
  - Restored by `cp`; `cmp` silent.
- **The layout check.** The first gates run failed on `layout_constants::every_fact_has_one_definition_or_a_named_reason`:
  "ENDED in ... cross_process.rs, ... threads.rs; SLOTS in ... cross_process.rs, ... turns.rs". The class was every
  `pub const` of the file, nine; nothing outside it reads them, and all nine are now private.
- **The census D7 check.** The third gates run failed on `slice_calls::no_file_builds_more_views_from_a_slice_than_the_census_counts`:
  "kernel.rs: 1 views built from a slice, census D7 allows 0", the kernel test's `ConcurrentGuestMemory::new(&mut
  buffer)`, now `from_raw_parts` as the view's own doc test builds one. Its sibling, `slice_calls::no_file_holds_more_slice_calls_than_the_census_counts`, matched
  the peer's `v.as_slice()` on a `Vec` in `cross_process.rs`, which the census allows none (the match is by text, on
  purpose); now `&v[..]`. Both files hold no more slice calls or slice-built views than the census counts.
- **A second test**, `a_child_gone_without_a_word_stops_the_run_by_name` (826): `/usr/bin/true` adopted; the stop
  "child process 4100: gone without a word (exit status: 0)" within 2 s.
- **Every run as held**: the gates' row below. No plant was live while it ran. No run reaches the server;
  `MACHINE_ON_ONE` stays.
- **Files:** `crates/w2m_vm/src/cross_process.rs` (new), `kernel.rs`, `run.rs`, `lib.rs`, the design. The card's list.

exit 0 in 418 s	default	2026-11-27T02:08:41Z	sprint 363	278b42a76ad0923a488791adc7631acdbb216f54	7163307622e29849f1271d666ce7755a37fd6310b47ac26afb142cd4d29d4b7b	root=9fdda7d8c4c63aafe48ec30d55410dd083385faf	rust=crates python=0 samples=0	base=6e3b67ba1ad016ec69bbbf92e6467f567b55b707
