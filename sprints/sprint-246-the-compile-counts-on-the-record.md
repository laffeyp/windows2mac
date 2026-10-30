# Sprint 246 — The compile counts on the record

```yaml
---
id: 246
status: active
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence for step 5" (reviewer-3's Medium 1 of its review at 243): plan step 5's gate counts "compiles at
run time … on the record" and their fall by the share the cache covers, and the Rust batch record counts no compile,
so the cache would have no figure to fall from. Census row 15d (compile time per block) is carried to step 5 in Deferred.

- **Where it stands, by lookup (2026-10-30):**
  - The frozen host wrote 23 `vm_compile_blocks` measurements, one per `VMC_COUNT_NAMES` entry (`vm_backend.py`
    199-203, `vm_compile_counts` 2604-2607; `run_frontier.py` 350, 385-390), and host seconds by exit kind, "host hvc
    N", "host start", "host other" (164-169).
  - The in-VM compiler is the same C on both hosts: `tools/guest_runtime/block_compiler.c` 20-27 and 46 count into
    `VM_COMPILER + 0x100`.
  - The Rust host reads those counts at the stop (`run.rs` 977-979, into `Outcome::counts`, names `guest.rs` 51-57)
    and keeps each served exit's host time by kind (`Outcome::serve_by_kind`, `run.rs` 611-612), and writes neither:
    `record.rs` 229-260 writes instructions, the rate, `run_seconds` "vm" and one "host", the exits by hvc and the load.
  - Deferred searched for `record.rs`, `w2m-vm.rs` and `batch_record.rs` (KIT_DIARY #102): the entry on substrate's
    strict mode names `batch_record.rs` with a re-visit on the substrate clone, not here; row 15b's names `run.rs`, not
    touched here.
- **The readers:** `record.rs`' `Execution` and its unit test, which counts the record's rows; `w2m-vm.rs` 733-747,
  which builds it; `batch_record.rs`, which holds the measurements' metrics to the reference's and the rate to its
  seconds; the stored records stay as written.
- **The work:**
  - `Execution` takes the counts and the host times by kind; the record writes `vm_compile_blocks` per count, the
    frozen variant names, and `run_seconds` "host <kind>" per kind in place of the one "host" row, as the frozen host
    wrote them; the rate's seconds unchanged in sum.
  - `batch_record.rs` holds the record's `vm_compile_blocks` variants to the reference's, every one of the 23, and
    "compiled" above zero.
  - The batch run measured three times cold by `scripts/batch-run-record.sh` into `store/runs/`, as Sprint 240 did.
  - Census row 15d met as step 5's baseline: the in-VM compiles and their causes, and the host's seconds on hvc 1 per
    hvc 1 exit, against the reference's 275,303 compiled; Deferred's row 15d resolved.
- **Not here:** the cache (card 247 onward).

## prerequisites

- 245 (closed)

## context_files

- `crates/w2m_vm/src/record.rs`, `src/bin/w2m-vm.rs`, `src/run.rs` 276-284 and 970-980, `src/guest.rs` 50-57,
  `tests/batch_record.rs`; `tools/guest_runtime/block_compiler.c` 20-46; `run_frontier.py` 150-200, 350-390 and
  `vm_backend.py` 199-203, 2604-2607 at the tag; `docs/step-4-census-2026-10-28.md` row 15d

## signal contract

### Emits

- Three batch-run records, each with 23 `vm_compile_blocks` measurements, `run_seconds` by host exit kind, and the
  measurements of Sprint 240, all `cold`, with their conditions.

### Invariants

- Each record conforms; its rate times its seconds is its instructions.

## artifact contract

### Files modified

- `crates/w2m_vm/src/record.rs`, `src/bin/w2m-vm.rs`, `tests/batch_record.rs`, `docs/step-4-census-2026-10-28.md`
  (row 15d), `BLACKBOARD.md` (Deferred); three records under `store/runs/`

### Command exit codes

- `scripts/gates.sh` returns 0; each record's `check_records` 0 violations

## observation contract

- **Expected values:** 23 `vm_compile_blocks` variants equal to the reference's names; "compiled" in the order of the
  reference's 275,303; the three runs' counts against each other.
- **Planted faults:**
  1. One count left off the record: `batch_record.rs` names the variant missing.
  2. "compiled" written as 0: `batch_record.rs` names it.
  3. A host kind's seconds dropped: the rate check names the seconds that do not make the instructions.

## done criteria

The Rust batch record carries the run's compile counts and its host time by kind, on three cold records, step 5's
baseline.

## self-review checklist

- [x] Scope names one verifiable property: the compile counts on the record.
- [x] context_files cite the frozen writer, the C, the Rust reader and the census row.
- [x] Emits: three records.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the compile counts on the record.

## result
