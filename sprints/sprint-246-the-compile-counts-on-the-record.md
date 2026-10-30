# Sprint 246 — The compile counts on the record

```yaml
---
id: 246
status: done
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

- **The record:** `Execution` takes `host_by_kind` and `compile_counts`; `w2m-vm` fills them from `serve_by_kind` and
  `Outcome::counts` in `VMC_COUNT_NAMES` order; the record writes `run_seconds` "host <kind>" per kind, summing to the
  seconds the rate divides by, and `vm_compile_blocks` per count, the frozen variant names. The measurement ids follow
  the Rust record's one form, "<id>-vm_compile_blocks-<cause>", where the frozen host wrote "<id>-vm-compile-<cause>".
- **The judge:** `compile_problems` holds the record's variants to the reference's 23 and "compiled" above zero, on the
  gates' record and every stored record but Sprint 240's three, which `BEFORE_COMPILE_COUNTS` names;
  `a_missing_extra_or_empty_compile_count_is_named` holds it. `record.rs`' unit test counts the new rows.
- **The baseline:** three records from 92f4b4cb8, each conforming, `check_records` 0 violations:

  | record | load at start | busy | compiled | hvc 1 exits | host on hvc 1 | instructions a second |
  |---|---|---|---|---|---|---|
  | `store/runs/w2m-vm-game-1793369487000000000` | 4.31 2.88 3.03 | llama-server 227% | 274,834 | 1,091 | 80.7 ms | 1,778,638,492 |
  | `store/runs/w2m-vm-game-1793369623000000000` | 5.22 3.96 3.45 | llama-server 218%, chrome-headless-shell 109% | 274,834 | 1,091 | 72.5 ms | 2,291,156,288 |
  | `store/runs/w2m-vm-game-1793369737000000000` | 4.42 3.94 3.49 | none | 274,834 | 1,091 | 53.2 ms | 2,538,851,228 |

  The compile counts are the same in all three and every refusal and cut equals the reference's; the reference
  compiled 275,303 warm. The rates moved with a busy machine, 1.78 to 2.54 billion; row 10's figure stands from
  Sprint 240's three runs at lower load.
- **Departure, row 15d:** the card planned it met and its Deferred line resolved. The VM's own compile time is inside
  its run seconds and not separated, so the row reads partial and the Deferred line is annotated, `census.rs` holding
  it there.
- **A trial run:** one run into `build/batch_run/` before the commit showed the 23 counts and 0 violations, never
  committed.
- **The gates:** `scripts/gates.sh` exit 0 in 525 s on 41b83a5de, "rust=crates python=0 samples=0", 110 tests, the
  three records replayed and checked.
- **Plant 1** (the "decode" count's line deleted from `store/runs/w2m-vm-game-1793369737000000000`): "no
  vm_compile_blocks \"decode\", which the reference counts".
- **Plant 2** (that record's "compiled" 274,834 to 0): "vm_compile_blocks \"compiled\" is Some(0.0)".
- **Plant 3** (that record's "host hvc 8" seconds deleted): "instructions_per_second 2538851227.637233 over
  28.637697762000002 s is 72706854119.75774 instructions, where 80306813574 retired".
- Each plant's `git diff --stat` showed its one line first; the record was restored by `cp` each time, and `cmp`
  found it equal to its copy.
