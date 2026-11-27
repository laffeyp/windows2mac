# Sprint 361 — A process that starts another, on several vCPUs: the design

```yaml
---
id: 361
status: done
phase: 6
pass_kind: docs
---
```

## scope

The Decision "Step 6's order weighed again at Sprint 360, G2 unmet", with its correction "The six held runs' count
corrected", puts card 4f next. All six two-vCPU family runs still held wait on it:
- `process_parameters`, two runs;
- `time_and_error_mode`, two runs;
- `processes`, two runs.

Each stops at `MACHINE_ON_ONE` (`crates/w2m_vm/src/host_request.rs` 79 and 532-534): "a process that starts another
runs on one vCPU until the Machine has its own thread (the run-loop design's card 4f)". The Decision of 2026-11-24
("Step 6 restored to the user's fix (b)") requires card 4f to be "re-read against the design's Change 4 (each process
its own vCPUs) before its card, the Python host's one Machine thread not carried by default". This card is that reading.
It is written as a design, `docs/design-a-process-that-starts-another-2026-11-26.md`, that passes `design_check`.

- **The state as it is.** Map how a child process runs today, each part with its file and line:
  - CreateProcessW and the spawner, a second `w2m-vm` run in its turn (`host_request.rs` near 535);
  - the link between the two processes, and KERNEL 33's wait;
  - what "the Machine" is and which thread drives it;
  - how the two processes take turns, and what `run.rs` 810 says of a link running on one vCPU;
  - what crosses between them: a pipe, a handle on the other's process, the child's exit, named objects.
- **Against Change 4** (`docs/design-rust-host-2026-10-24.md` 212-240). Each process's objects and waits stay in its
  own VM, and the supervisor holds only what crosses processes. Map each crossing above to its home under Change 4,
  and state what of today's form is carried, changed or dropped. "The Python host's one Machine thread" is not carried
  by default, and if any part is carried, the design says why.
- **The form on several vCPUs.** How a parent on N vCPUs starts a child, waits on it, and exchanges a pipe and an exit
  code with it, with no `MACHINE_ON_ONE` refusal:
  - which thread or process runs the child, and on how many vCPUs;
  - which lock or request each crossing takes;
  - what one vCPU keeps unchanged.
  Look up and quote what a design rests on: Windows' process and pipe semantics on Microsoft Learn (CreateProcessW,
  CreatePipe, WaitForSingleObject on a process handle, GetExitCodeProcess), and the macOS primitives used.
- **Its judges, named for the build cards.**
  - The six held runs, held to Windows' rows on two vCPUs.
  - The `processes` runs' pipe.capacity row, which Sprint 360 carries in: it ran load-dependent on the wall clock
    (1,816 and 3,634 of Windows' 4,096 under load). State how it is held, by an allowed range from Windows' own rows
    or a quiet host, with grounds.
  - One vCPU's batch run, family runs and exit ledger, unchanged.
- **The cards that follow**, each one concept with its judge and plant, in order, within the budget (G2 by Sprint 366,
  the Decision above). If the reading finds that card 4f needs C2 or C3 first (the Decision's revisit clause), it says
  so.
- **Not here:** any code.

## prerequisites

- 360 (closed)

## context_files

- `docs/design-rust-host-2026-10-24.md`: Change 4 (212-240) and Change 1's order; `docs/design-run-loop-on-several-vcpus-2026-10-31.md` (card 4f, 210-220, and the Machine's refusal).
- `crates/w2m_vm/src/host_request.rs` (`MACHINE_ON_ONE` and the spawner, 520-620), `crates/w2m_vm/src/run.rs` 800-830,
  and the kernel's process and pipe services (`crates/w2m_vm/src/kernel.rs`).
- `probes/processes/processes.c`, `probes/process_parameters/`, `probes/time_and_error_mode/`, and their rows in
  `store/family_runs/known-2026-10-29.tsv`; `store/family_runs/two-vcpus-refused-2026-11-26.tsv`.
- `grep -n "Step 6 restored to the user's fix (b)\|Step 6's order weighed again at Sprint 360\|The instruction clock on several vCPUs" BLACKBOARD.md`: the three Decisions.
- `crates/design_check` (what a design must state to pass), and a design that passed it as the form:
  `docs/design-handles-and-sync-objects-2026-11-26.md`.

## signal contract

### Emits

- None: a design card with no code.

### Invariants

- No code changes. `design_check` passes over every design, this one among them.

## artifact contract

### Files created

- `docs/design-a-process-that-starts-another-2026-11-26.md`

### Command exit codes

- `target/release/design_check` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The design's map of today's state, each part with file and line.
  - Each crossing's home under Change 4.
  - The several-vCPU form, with its quotes.
  - The judges, the pipe.capacity holding with its grounds.
  - The ordered cards.
- **Planted faults:**
  - (1) The design's goal or systems section removed. `design_check` fails, naming it.

## done criteria

A design, passing `design_check`, that maps today's child-process form against Change 4, states the form on several
vCPUs and its judges, and orders the cards that take the six held runs off `MACHINE_ON_ONE`.

## self-review checklist

- [x] Scope names one verifiable property: the design, passing `design_check`, with its cards.
- [x] context_files cite both designs, the code, the probes, the held list and the Decisions.
- [x] Emits: none, a docs card.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the design's parts; one plant on its check.
- [x] One concept: card 4f's design.

## result

The gates, run by the Supervisor at the close on the finished tree, the design included (`docs/design-*` is gated):
exit 0 in 235 s	default	2026-11-27T00:10:51Z	sprint 361	ae82b92baa669d78fe3cd87d0b57a67658743f7e	afe675528428dad6ce85aa6b3678b112b7eb32e4acf4b4895bfbcb711067b331	root=1e705dafeaa788ddbba6c28e4f7eab11f100295f	rust=crates python=0 samples=0	base=a2f898a5d7c179835340150b6de17da2122f3f33

- **The design:** `docs/design-a-process-that-starts-another-2026-11-26.md`, 389 lines. Its finding: on several
  vCPUs no Machine runs. Processes run at once, each a `w2m-vm` on the run's vCPUs, and a request server thread in the
  first `w2m-vm` answers what crosses processes under one lock, `kernel` (Change 4's). Each process's link thread
  applies a wake from another process under its own runtime word. One vCPU keeps the Machine and its turns unchanged,
  for the instruction clock. The Python host's Machine thread is not carried: a Machine on its own thread still runs
  processes in turns, which Change 4 and Windows' scheduling do not.
- **Card 4f does not need C2 or C3 first.** KERNEL 33 was the one host request that waited, for the supervisor's turn,
  with the child's word held (run-loop design 49-57). The server runs no guest code and waits for none, so the wait is
  gone and the word is held across a bounded round trip, as for every host request today. The revisit clause of the
  Decision "Step 6's order weighed again at Sprint 360" is not met.
- **The pipe.capacity row** is held at Windows' 4,096 exactly, with no range. Measured 2026-11-26 on the Rust host,
  wall clock, one vCPU (two are refused): 50 of 50 runs read 4,096. These were 2 quiet runs whose every row equals the
  contract's after its derivation, 36 runs at once, and 12 runs at once beside 54 busy loops on 18 cores, at
  23:39:40-23:39:43 UTC. The 1,816 and 3,634 were the Python host's (commit 9a7515bb9).
- **The cards:**
  - 4f1, Sprint 362: the objects behind `kernel`.
  - 4f2, Sprint 363: the request server, `cross_process.rs`, judged by a two-process test.
  - 4f3, Sprint 364: processes at once, the child on one vCPU, and the server's deadlock verdict across processes;
    the six runs on two vCPUs, a two-process INFINITE-wait guest stopped as a deadlock, two plants.
  - 4f4, Sprint 365: the child on the run's vCPUs, 20 runs of each of the six, and a cross-process ping-pong guest
    with its plant.
  - G2's judge, Sprint 366.
- **Commands:**
  - `target/release/design_check` exit 0: "design_check: 9 plans and designs state the goal and the systems".
  - `target/release/signal_report --check` exit 0.
- **Plant 1** "docs/design-a-process-that-starts-another-2026-11-26.md: no "## Against the goal" section", `design_check` exit 1; live 23:58:49-23:59:01 UTC:
  - The design copied to the scratchpad, `cmp` equal.
  - Its "## Against the goal" section removed by Edit.
  - `design_check` exit 1: `docs/design-a-process-that-starts-another-2026-11-26.md: no "## Against the goal" section`.
  - Restored by `cp`, `cmp` equal, `design_check` exit 0 again.
