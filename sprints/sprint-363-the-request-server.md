# Sprint 363 — The request server: what crosses processes, answered one request at a time under `kernel`

```yaml
---
id: 363
status: active
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

GATES-LINE
