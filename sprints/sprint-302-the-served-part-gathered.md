# Sprint 302 — The served part gathered

```yaml
---
id: 302
status: active
phase: 6
pass_kind: functional
---
```

## scope

The run-loop design's card 2 (`docs/design-run-loop-on-several-vcpus-2026-10-31.md`, "The cards" 2; "The guest's
parts"), first of two by rule 6: `Guest`'s 22 Served fields (`crates/w2m_vm/src/guest.rs` 631-716; the design's table)
are reached about 172 times in 12 files (`guest.rs`, `run.rs`, `host_request.rs`, `modules.rs`, `threads.rs`,
`compile.rs`, `bin/w2m-vm.rs`, and the tests `run_loop.rs`, `virtual_memory.rs`, `bridges.rs`, `process_area.rs`,
`first_decline.rs`). This card gathers them; the next makes their one home a lock whose guard checks the word.

- **The work:** a `Served` struct in `guest.rs` holding the 22 fields as the design's table names them, with their
  documentation moved; `Guest.served: Served`; every use `guest.<field>` read as `guest.served.<field>`. No field
  changes type, no behavior changes, one vCPU.
- **The design amended with this card's commit:** "The cards" 2 split in two by rule 6, 2a the Served part gathered
  (this card), 2b `served` a lock whose guard checks the word, the run loop's serving paths taking it, with the
  card's judge and plant as the design states them.
- **Not here:** the lock and its guard (the next card); the Load part shared by reference and the mapping's view per
  thread (card 4, the threads); the run loop's own state.

## prerequisites

- 301 (closed)

## context_files

- the design's "The guest's parts" and "The cards"; `crates/w2m_vm/src/guest.rs` (`pub struct Guest`)
- the 12 files above

## signal contract

### Emits

- None; a struct's shape, held by the compiler and the runs.

### Invariants

- Every run as held: the batch run, the guests, the families, the case runner, each by its judge in the gates.

## artifact contract

### Files modified

- the 12 files above; `docs/design-run-loop-on-several-vcpus-2026-10-31.md` ("The cards" 2, with the card's commit)

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `Guest` with 23 fields, 22 in `Served`; no `guest.<served field>` left outside `served`, by a
  search of the 22 names after the change; every run as held.
- **Planted faults:** (1) a test, `the_served_part_is_the_designs`, reading the design's table and `Served`'s fields,
  fails on a field the table calls Served that `Served` lacks; the plant moves `random` back to `Guest`, and it must
  name `random`.

## done criteria

The 22 fields the design calls Served live in one value, `Guest.served`, ready for the lock.

## self-review checklist

- [x] Scope names one verifiable property: the Served fields in one value.
- [x] context_files cite the design and the files.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and a plant changing what its judge reads.
- [x] One concept: one struct's shape; 12 files, each a mechanical rename of a field's path, the floor of a field move.

## result

