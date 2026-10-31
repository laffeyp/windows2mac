# Sprint 293 — The run loop on several vCPUs, designed

```yaml
---
id: 293
status: active
phase: 6
pass_kind: design
---
```

## scope

Step 6's next move after census row D7: the run loop on a second vCPU, under today's runtime word (the census's "What
the counts say": "a second vCPU under it is the frozen host's shape"), judged by gate items G2 to G4. The host design
(`docs/design-rust-host-2026-10-24.md`) fixes the target: a thread per vCPU that runs it and serves its exits, its
host locks (`modules`, `vm_map`, `code_page`, `files`, `file`, `output`, `trace`, `sched`, `vcpu_handle`) and no lock
across `hv_vcpu_run`; the guest-memory design's choice 6 fixes the order: the runtime word outermost, no lock over
`Guest` whole. Neither says how today's `Guest`, 45 fields the run loop holds as `&mut Guest`, divides.

- **The work:** `docs/design-run-loop-on-several-vcpus-2026-10-31.md`, through `design_check`:
  - each of `Guest`'s 45 fields placed by its writers after load, each writer found in the code: set once at load
    and read after; a vCPU's own; behind one of the host design's locks, named; or a one-vCPU diagnostic the run
    refuses on a second vCPU; the run loop's own state (`HostCompiler`, the exits' counts) the same way;
  - the threads: who creates each vCPU, how an exit is served on its own thread, how the run stops and every thread
    ends, how a wake reaches a parked vCPU (census D2);
  - where the second vCPU's preconditions sit: the FEAT_LSE2 refusal, break-before-make for a block split, the lock
    order, the race tests and their plants (the step 6 Decision's seven);
  - the cards that build it, each with its gate, the last G4's;
  - against the goal's figures and against the systems that solved it, as `design_check` asks.
- **Not here:** code; the guest's lock split (the host design's change 1), which step 6 builds after G4.

## prerequisites

- 292 (closed)

## context_files

- `docs/design-rust-host-2026-10-24.md` (whole); `docs/design-guest-memory-on-several-vcpus-2026-10-31.md`
- `docs/step-6-census-2026-10-30.md` (G1-G6, D1-D7); BLACKBOARD, the step 6 sequence Decision and its annotations
- `crates/w2m_vm/src/guest.rs` (`Guest`), `crates/w2m_vm/src/run.rs` (the loop, `Outcome`), `crates/w2m_vm/tests/litmus.rs`
  (several vCPUs today, under one `Mutex`)

## signal contract

### Emits

- None; a design.

### Invariants

- No code changes; `design_check` passes with the new document among the plans and designs.

## artifact contract

### Files modified

- `docs/design-run-loop-on-several-vcpus-2026-10-31.md` new

### Command exit codes

- `target/release/design_check` returns 0; `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** every one of `Guest`'s 45 fields in the document's table, each with the writer that places it
  (file and function) or "none after load"; `design_check` counting 6 plans and designs.
- **Planted faults:** (1) the document's goal section removed; `design_check` must fail naming it. (2) one field left
  out of the table; a check written into the card's run counts the table's rows against `Guest`'s fields by name and
  must name the field.

## done criteria

The run loop's second vCPU has a design every field of `Guest` is placed in, with its cards and gates, passing
`design_check`.

## self-review checklist

- [x] Scope names one verifiable property: every field placed, the threads and the cards designed.
- [x] context_files cite both designs, the census, the Decision and the code.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: the run loop's division on several vCPUs.

## result

- **The gates:** GATES-LINE
