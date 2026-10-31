# Sprint 270 — Step 6's census

```yaml
---
id: 270
status: done
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

- **The census** (975a98925): `docs/step-6-census-2026-10-30.md`, six gate items, five design changes and six waiting
  Deferred items, each with the frozen host's state, the Rust host's, its status and what shows it met. All six gate
  items absent on the Rust host, which makes one vCPU in every run; changes 1, 2, 3 and 5 absent, 4 partial (a child
  `w2m-vm` a process, in turns).
- **What the lookups found:** the memory XCHG lowers to a load and two plain stores (`rules.rs` 629-644), not atomic
  on two vCPUs; `lock_runtime` is the frozen host's two-state `CASA` spin (`monitor_code.rs` 110-129); one transport
  page a process (`layout.rs` 99); the frozen host ran two families on two vCPUs (Sprint 119), not 68, and two scratch
  batch runs on the wall clock (Sprint 122); no host measured instructions a second on four vCPUs or the hot imports'
  rate on six threads.
- **The Decision** "Sequence for step 6": 271 the XCHG; 272 two vCPUs under the runtime word, judged by the five
  litmus tests and the EnTSO plant; 273 the shared counters; 274 Sprint 122's guests with the kick tests; 275 the 68
  families; 276 the batch run on the wall clock; then the rates and the design's changes in the order their waits name.
- **A ground checked:** the Decision first said QEMU "ran several vCPUs under its big lock and split the translation
  lock after"; QEMU's documentation states only its current design, and commit 0ac20318c ("tcg: remove tb_lock")
  shows a translation lock removed, not vCPUs run under one first; the ground now holds what the commit says
  (4668e5ead, 19e5ee826).
- **The check:** `census.rs` holds the new document's paths.
- **The gates:** `scripts/gates.sh` exit 0 in 249 s on 65951bbc9, copied from `store/gates/passes.tsv`; the timing stage 681 ns an exit at load 7.26.
- **Plant 1,** restored by `cp` and checked by `cmp`: `crates/w2m_vm/src/turns.rs` misspelled `turn.rs`; the test
  failed, "docs/step-6-census-2026-10-30.md names 1 path(s) never in the tree: [\"crates/w2m_vm/src/turn.rs\"]".
