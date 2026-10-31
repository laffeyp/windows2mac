# Sprint 295 — The exit table completed

```yaml
---
id: 295
status: active
phase: 6
pass_kind: design
---
```

## scope

Two rows of `docs/design-run-loop-on-several-vcpus-2026-10-31.md`'s exit table are wrong or open, each from reviewer-3's
review at 294, before any code divides `Guest` on them:

- **The model step** (`hvc #10`, `#11`) reads "no word". A model step of a form x86 locks (a `LOCK`-prefixed form, or a
  memory XCHG the exclusive pair cannot take) needs every other vCPU stopped: an aligned locked access runs in the VM
  as an exclusive pair and takes no word (Deferred, Sprint 272 at 274). The eighth precondition, in the step 6
  Decision since 294's close, with its two conditions (the stop complete before the step's load; an interrupted
  exclusive pair failing on resume), the second looked up at 294's close as far as Arm's "ARMv8-A Synchronization
  primitives" reaches.
- **A host request** (`hvc #8`) runs under the guest's hold of the word; which of its services can wait was not
  listed. Looked up for this card: a pipe read on an empty pipe answers at once, "would block", the guest waiting in
  its own code (`kernel.rs` 270-272); the file bridge's one loop names a drive at its creation (`file_bridge.rs` 161);
  a child process's KERNEL 33 request waits for the supervisor's answer in its turn (`host_request.rs` `child_request`,
  `turns.rs` 264 `hand_back(Message::Request, …)`), the guest holding the child's word, which the host cannot let go
  without breaking the guest's own critical section.

- **The work:** the design amended: the model-step row's two forms and the eighth precondition in its list; the
  host-request row with its one waiting service and the design's answer, a child process keeping one vCPU until
  KERNEL 33 stops waiting under the word (the host design's change 2), the supervisor's process free to run on two;
  card 4's judges and the revisit clause to match.
- **Not here:** code; the owner's read and one `LOCK_OWNER` (the card that divides `Guest`).

## prerequisites

- 294 (closed)

## context_files

- `docs/design-run-loop-on-several-vcpus-2026-10-31.md` ("The exits", "The second vCPU's preconditions", "The cards",
  "Revisit if"); `docs/design-rust-host-2026-10-24.md` (change 2)
- `crates/w2m_vm/src/host_request.rs` 150-175; `crates/w2m_vm/src/turns.rs` 255-280; `crates/w2m_vm/src/kernel.rs`
  250-275; BLACKBOARD Surfaced, the review at 294, and Deferred (Sprint 272)

## signal contract

### Emits

- None; a design.

### Invariants

- No code changes; `design_check` passes.

## artifact contract

### Files modified

- `docs/design-run-loop-on-several-vcpus-2026-10-31.md`

### Command exit codes

- `target/release/design_check` returns 0; `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the model-step row naming the stop for locked forms; the host-request row naming KERNEL 33's
  wait and its answer; eight preconditions in the list; `design_check` passing.
- **Planted faults:** (1) the design's systems section stripped of its sources (each `http`); `design_check` must
  name the systems unsourced. (2) the preconditions' list cut to seven items; a check written into the card's run
  counts the list's numbered items and must report seven.

## done criteria

Every exit in the design's table states, from the code, whether it holds the word, what its service touches, whether
it can wait, and its rule on two vCPUs.

## self-review checklist

- [x] Scope names one verifiable property: the exit table complete against the code.
- [x] context_files cite the design, the host design's change 2, the code read and the review.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each changing what its judge reads.
- [x] One concept: the exit table's two open rows.

## result

- **The gates:** GATES-LINE
