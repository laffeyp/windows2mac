# Sprint 366 — Host requests off the runtime word (C2): the design

```yaml
---
id: 366
status: active
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

GATES-LINE
