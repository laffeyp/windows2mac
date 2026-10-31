# Sprint 270 — Step 6's census

```yaml
---
id: 270
status: active
phase: 6
pass_kind: functional
---
```

## scope

Plan step 6 (`docs/rust-port-plan-2026-10-24.md` 161-168), several vCPUs, follows step 5's close (Sprint 268). Step 4
began with a census of its gate's items against the Rust host's state (`docs/step-4-census-2026-10-28.md`), and the
order of its cards came from that table; step 6 has none. This card writes it: each gate item, each change the host
design sets for several vCPUs, and each Deferred item that waits on them, with its state on the Rust host by lookup and
what would show it met, then a Decision "Sequence for step 6" ordering the cards from the table.

- **Where it stands, by lookup (2026-10-30):**
  - The Rust host makes one vCPU in every run (`crates/w2m_vm/src/bin/w2m-vm.rs` 101 and 489,
    `src/case_inputs.rs` 233). The litmus work is the frozen host's and a C probe's (Sprints 009 and 105,
    `probes/mac_hv_litmus/mp_litmus.c`, `store/runs/litmus-1791441621`, `-1791496387`).
  - The plan's gate: the litmus tests MP, SB, LB, 2+2W and IRIW; the 68 families on two vCPUs; the batch run on the
    wall clock; Sprint 122's guests (handshake, busychild, crosswake, pingpong, statsize) ending on two vCPUs in many
    runs, crosswake's hang (Decision 2026-10-23) fixed; instructions a second on four vCPUs against the game's 4.6
    billion over six threads; the hot imports' rate on six threads at once against one.
  - The design (`docs/design-rust-host-2026-10-24.md`): change 1, a runtime call takes the lock of what it touches;
    change 2, a host request holds no guest lock, on its vCPU's own page; change 3, translation in regions looked up
    without a lock; change 4, one process's waits in its VM, the supervisor seeing what crosses processes; change 5,
    three-state lock words.
  - Deferred and Surfaced items that name step 6 or several vCPUs: a memory XCHG not atomic ("step 6's first card",
    the sweep of 2026-10-29); a parked vCPU's wake (Sprint 118); the first taking's wait (Sprint 164); crosswake's
    planted no-kick (Sprint 184); the kick rule with no Rust test (Sprint 222); the in-VM compiler's counters and the
    installer's shared writes (Sprint 247, annotated at 248 and 261).
- **The work:**
  - `docs/step-6-census-2026-10-30.md`: one row per item above, in step 4's census's columns (what the item asks, where
    it is measured, the frozen host's state, the Rust host's, the status, what would show it met), each cell's facts
    with their file and line or record.
  - `crates/signal_report/tests/census.rs` holds the new document's paths, as it holds the other two.
  - The Decision "Sequence for step 6", agent-resolved: the cards in order from the table, with grounds and a revisit
    clause; the XCHG first if the table bears out the sweep's ruling.
- **Not here:** any change to the host; the step's first card.

## prerequisites

- 269 (closed)

## context_files

- `docs/rust-port-plan-2026-10-24.md` 161-168; `docs/design-rust-host-2026-10-24.md`; `docs/step-4-census-2026-10-28.md`
- `BLACKBOARD.md`: Deferred, Surfaced (the sweep of 2026-10-29), Decisions 2026-10-17, 2026-10-21, 2026-10-23
- `crates/w2m_vm/src/` (`run.rs`, `threads.rs`, `turns.rs`, `monitor_code.rs`); `tools/guest_runtime/`

## signal contract

### Emits

- None; a document of the code and records as they stand.

### Invariants

- No code or record changes; one test reads one more document.

## artifact contract

### Files modified

- `docs/step-6-census-2026-10-30.md` new, `crates/signal_report/tests/census.rs`, `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** every cell's file and line read at this tree; the census's path check passing on the new
  document.
- **Planted faults:**
  - (1) a path in the new document misspelled: `census.rs` names it.

## done criteria

Step 6's census stands in one document, every gate item, design change and waiting Deferred item with its state and
what would show it met, and the Decision orders the step's cards from it.

## self-review checklist

- [x] Scope names one verifiable property: step 6's items against the Rust host, each placed.
- [x] context_files cite the plan, the design, the model census and the record.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and one plant.
- [x] One concept: step 6's census.

## result
