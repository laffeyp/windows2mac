# Sprint 366 — Host requests off the runtime word (C2): the design

```yaml
---
id: 366
status: done
phase: 6
pass_kind: docs
---
```

## scope

The Decision "Step 6's order weighed again at Sprint 360" puts C2 first after G2, and G2 is met (the Decision "G2 met
at Sprint 364"). C2 is the host design's Change 2 (`docs/design-rust-host-2026-10-24.md` 155-170): "a host request
from the guest holds no guest lock, on its own vCPU's page". Today `w2m_host` (`tools/guest_runtime/runtime.c` 25-35)
writes one transport page for the whole process and executes `hvc #8`, and "the transport page is every vCPU's"
(`runtime.c` 60), so every host request is made with the runtime word held: 67 call sites in 14 files at 7ca67b25a
(file.c 22, sync_objects.c 8, processes.c 7, module.c 7, exceptions.c 5, virtual_memory.c 4, process.c 3, and seven
files with one or two). This card is the reading Change 2 asks for: "Each of the 65 call sites is read for state it
holds across the request". It is written as a design, `docs/design-host-requests-off-the-runtime-word-2026-11-26.md`,
that passes `design_check`.

- **The state, as it is.** The transport page today (its address, its layout, who serves it: `host_request.rs`), the
  word's take around a request (`claim_runtime_word`, the bridges' takes), and the host side's assumptions that the
  word is held while it serves (`GuestLock::serve`'s assertion, `kernel`'s guard, Sprint 362).
- **The per-vCPU page.** Where each vCPU's transport page lives (its private page beside its state block), how the
  host finds the exiting vCPU's page, and what of the layout moves; and Sprint 363's client slot, which today is "one
  request per process in flight, since the child's runtime word covers its `hvc #8`" (the design of card 4f,
  "The request server"), becoming a slot per vCPU thread.
- **Every call site, read.** A table of all 67 sites, each with file:line, the request it makes, the guest state it
  holds across the `hvc #8` (a handle slot, a heap in the middle of a change, a module table during a load, a thread's
  scratch, none), and its form under C2: the host's answer first and the slot after, or the slot taken, marked
  pending, the lock let go, the host asked, and the slot filled or freed under the lock again (Change 2's two forms).
  The table is the class; the design says how a later card proves every row was changed.
- **What the word still covers.** After C2, which services still take the runtime word and why, against Change 1's
  lock classes (Sprints 342-359).
- **The woken mark's window.** The Decision "The woken mark in `threads::wake_from_another_process`, a backstop for
  C2": today a wait across processes checks and blocks inside one holding of the word (`processes.c` 71-89), so the
  mark is unreached. State where C2 opens that window and name the build card's plant of the mark with a judge that
  reaches it.
- **Its judges, named for the build cards**: every run as held on one vCPU and two (the batch run, the 273 family runs,
  the `twoproc` and two-process guests, the exit ledger); a guest that makes host requests from two vCPUs at once and
  checks every answer is its own; a count of host requests made with the word held, zero after the last card, from
  the per-lock timing of Sprint 358 or a new count; and the woken mark's plant.
- **The cards that follow**, each one concept with its judge and plant, in order.
- **Not here:** any code.

## prerequisites

- 365 (closed)

## context_files

- `docs/design-rust-host-2026-10-24.md`: Change 1 and Change 2 (to 170), and the locks table.
- `docs/design-handles-and-sync-objects-2026-11-26.md` (the form of a design whose cards split a lock) and
  `docs/design-a-process-that-starts-another-2026-11-26.md` ("The request server", "Which lock or request each crossing
  takes").
- `tools/guest_runtime/runtime.c`, `tools/guest_runtime/runtime.h`, and every file under `tools/guest_runtime/` with a
  `w2m_host(` call (`grep -c 'w2m_host(' tools/guest_runtime/*.c`).
- `crates/w2m_vm/src/host_request.rs` (the transport's reader), `crates/w2m_vm/src/monitor_code.rs` (the word),
  `crates/w2m_vm/src/run.rs` (`GuestLock::serve`, `KernelLock`), `crates/guest_layout/src/lib.rs` (the pages, `guest_runtime::layout`).
- `grep -n "Step 6's order weighed again at Sprint 360\|G2 met at Sprint 364\|The woken mark in .threads::wake_from_another_process." BLACKBOARD.md`:
  the Decisions.
- `crates/design_check` (what a design must state to pass).

## signal contract

### Emits

- None: a design card with no code.

### Invariants

- No code changes. `design_check` passes over every design, this one among them.

## artifact contract

### Files created

- `docs/design-host-requests-off-the-runtime-word-2026-11-26.md`

### Command exit codes

- `scripts/gates.sh` returns 0 (a design is in the gated tree)
- `target/release/design_check` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The design's map of the transport and the word today, each part with file and line.
  - The table of all 67 call sites, each with its held state and its form under C2, and a count that matches
    `grep -o 'w2m_host(' tools/guest_runtime/*.c | wc -l`.
  - The per-vCPU page's place, with its layout.
  - The judges and the ordered cards, the woken mark's plant among them.
- **Planted faults:**
  - (1) The design's goal or systems section removed. `design_check` fails, naming it.

## done criteria

A design, passing `design_check`, that reads every host request's call site for the state it holds across the
request, places each vCPU's transport page, and orders the cards that take host requests off the runtime word.

## self-review checklist

- [x] Scope names one verifiable property: the design, passing `design_check`, with its 67-row table and cards.
- [x] context_files cite Change 2, the two prior designs, the runtime C, the host's reader and the Decisions.
- [x] Emits: none, a docs card.
- [x] Artifact contract gradable by command; the gates run, since a design is gated.
- [x] Observation contract: the design's parts; one plant on its check.
- [x] One concept: C2's design.

## result

exit 0 in 241 s	default	2026-11-27T04:43:52Z	sprint 366	5fd4d381dccd94825ffcd89b60c6d98b388ed074	6764edb17e92010fd519a93bbd2b814874a3b0fc3fa2934b3a6cb00480bb2e11	root=0af6d2d0ce3f7e1f608a5d1af269a11e75c7f816	rust=crates python=0 samples=0	base=7ca67b25aa773bb11a9b42e723704de1ae2989f2

- **The design:** `docs/design-host-requests-off-the-runtime-word-2026-11-26.md`, 536 lines. The grep's 67 lines in 14
  files are the request's definition (`runtime.c` 26), two wrappers' bodies (`runtime.c` 22, `threads.c` 83) and 64
  direct calls; the wrappers' 13 callers and `runtime.h` 341's two make the class 79 request sites. 39 hold guest state
  across the `hvc #8`, 40 hold none. By kind: a shared static buffer 14, a table slot 15 (13 of `files[]`, 2 of
  `searches[]`; 4 of them a file's position too), an object's id with its reference dropped 5, the wait's check 6, the
  process's one exception area 3, a set-once static 1. No site holds a heap mid-change or the module table during a
  load (that table is the host's).
- **The per-vCPU page:** the 19th page of each vCPU's slot, `slot + 18 * PAGE`, mapped at `PROCESS_VA + TRANSPORT_OFF`
  beside the state block; the tables move one page, 27 of 32; no table added, the page sharing the state block's
  2 MB block. `struct w2m_transport` and `w2m_host`'s text unchanged; `VcpuPages` gains `transport`, and the host's
  readers read the served vCPU's. The request server's child slot gains a record per vCPU at words 910-973.
- **What the word still covers after C2:** the run table; the services still of class `RuntimeWord` between their
  requests (`files[]`, the module statics, `info[]`); the in-VM compiler; the exception entry; and on the host the
  requests whose serve writes the run table or PEB.Ldr. The run's lock stays one host lock; the host design's locks per
  part are its card H.
- **The woken mark's window:** opened by card 10 at the six wait sites, between the check's answer and `w2m_block`'s
  exchange; the wait arms the mark before its check. Plant: the mark out of `wake_from_another_process`
  (`threads.rs` 691), judged by `two_process_pipe_pingpong` 20 runs on two vCPUs with a census count that shows the
  window reached.
- **The judges:** every run as held on one vCPU and two; `own_answers.c`; `Outcome::requests_under_word` by op, zero
  after card 12; the mark's plant.
- **The cards:** after Sprint 367 (the batch run's stop on two vCPUs on the wall clock) and 368 (G3's runnable part),
  by the Decision made at this close on reviewer-4's thirty-seventh review: card M, Sprint 369, measures the game's
  batch run on two vCPUs with lock timing (`W2M_TIME_LOCKS`), the runtime word's takings, holds and waits split by
  host request class, the count of requests under the word by op, and `GuestLock`'s hold and wait. The prerequisites,
  Sprints 370-372: the per-vCPU page, the request server's record per vCPU thread, the host's serve by class. Then
  the release cards (the nothing-held sites with the release helper first; two of buffers; the file slots; the file
  position the host's; objects referenced across; the waits; the exception area and the clock's base) and card H,
  the host's locks per part (design-rust-host 262-272), in the order of the measured wait each removes, each built
  only above Change 1's bound, 5% busy or a p99 wait above 10 µs (design-rust-host 136-139), weighed against the
  goal's 37,600 calls a second over four to six threads; `w2m_host` itself last. Every card's judge 1 names G3's runs.
- **Commands:**
  - `target/release/design_check` exit 0: "design_check: 10 plans and designs state the goal and the systems".
  - `target/release/signal_report --check` exit 0.
- **Plant 1** "docs/design-host-requests-off-the-runtime-word-2026-11-26.md: no "## Against the goal" section", design_check exit 1; live 04:25:28-04:25:43 UTC:
  - The design copied to the scratchpad.
  - Its "## Against the goal" heading removed by Edit.
  - `design_check` exit 1: `docs/design-host-requests-off-the-runtime-word-2026-11-26.md: no "## Against the goal" section`.
  - Restored by `cp`, `cmp` equal, `design_check` exit 0 again.
