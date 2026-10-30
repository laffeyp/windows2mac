# Sprint 249 — Compiles by image

```yaml
---
id: 249
status: active
phase: 6
pass_kind: functional
---
```

## scope

Plan step 5's gate: "the batch run conforms with the cache, and compiles at run time, counted on the record, fall by
the share the cache covers" (`docs/rust-port-plan-2026-10-24.md` 158-160), the cache UnityPlayer.dll's first. The
record counts the in-VM compiles by cause (274,834 "compiled", Sprint 246) and not by the image they come from, so the
share UnityPlayer.dll's code holds, the share a cache of it covers, is on no record. A gate's quantity is on the record
before the change that moves it (KIT_DIARY #100).

- **Where it stands, by lookup (2026-10-30):**
  - `run.rs`' `vm_compiled` (395-440) drains the in-VM compiler's log at each exit: one `Translation { start, end,
    code }` a compile, into `Outcome::translations`; `Outcome::blocks` keys them by start.
  - `Guest::images` (`guest.rs` 656) holds every PE image loaded, the program and its DLLs at the start, a DLL loaded
    later (`modules.rs` 228, `threads.rs` 1248); each has its name, base and `pe.size_of_image` (`pe_image` 110).
    The system modules w2m serves (kernel32 and the rest) are the guest runtime's C, not compiled.
  - The vocabulary's `vm_compile_blocks` "counts blocks the compiler in the VM handled, variant naming the outcome
    (compiled, or a refusal or cut cause)" (`signals/0.12.json` 1335); "compiled in <image>" names the outcome
    "compiled" by where, so no new metric is needed. `batch_record.rs`' `compile_problems` refuses a variant the
    reference does not count (`vm_compile_blocks {extra:?}, which the reference does not count`).
  - The host's own compiles, 1,091 a run (`vm_exits` "hvc 1"), are 0.4% of the run's compiles and not counted here.
  - Searched for `run.rs`, `record.rs`, `w2m-vm.rs`, `batch_record.rs` in Deferred, Surfaced and
    `docs/surfaced-archive-2026-10-30.md` (as card 248, the day's search): census row 15b names `run.rs` for string
    errors, which this card does not change; census row 11, "the DLLs and imports the batch run loads and calls …
    re-visit with step 5's card, which chooses UnityPlayer.dll's code by what the run calls": this card counts the
    compiles by image, and leaves the imports' count to the card that chooses the code. `guest.rs` is read, not
    changed.
- **The readers:** `w2m-vm.rs`, which builds the record's `Execution`; `record.rs`; `batch_record.rs` on the gates'
  record and on every stored record.
- **The work:**
  - At the stop, the run's translations counted by the image whose range holds each start, and those in none.
  - The record writes `vm_compile_blocks` "compiled in <image>" for each image with a compile and "compiled outside
    any image", after the 23 counts.
  - `w2m-vm` refuses a run whose counts by image do not add up to "compiled".
  - `batch_record.rs` allows those variants beside the reference's, holds their sum to "compiled", and requires them on
    every record after Sprint 248's last, `w2m-vm-game-1793387174000000000`.
  - Three cold records by `scripts/batch-run-record.sh`, run alone; UnityPlayer.dll's share of the compiles from them.
- **Not here:** the cache; the imports' count (census row 11's rest); the host's compiles by image.

## prerequisites

- 248 (closed)

## context_files

- `crates/w2m_vm/src/run.rs` 270-285, 395-440; `src/guest.rs` 656; `crates/pe_image/src/lib.rs` 110
- `crates/w2m_vm/src/record.rs`, `src/bin/w2m-vm.rs`, `tests/batch_record.rs` 560-590
- `signals/0.12.json` 1335; `docs/rust-port-plan-2026-10-24.md` 158-160; census row 11

## signal contract

### Emits

- Three batch-run records, each with Sprint 246's measurements and `vm_compile_blocks` by image.

### Invariants

- Each record conforms; the 23 counts as Sprint 246's; the monitor code and the C runtime unchanged.

## artifact contract

### Files modified

- `crates/w2m_vm/src/run.rs` or `src/record.rs` (the count), `src/bin/w2m-vm.rs`, `tests/batch_record.rs`,
  `BLACKBOARD.md`; three records

### Command exit codes

- `scripts/gates.sh` returns 0; each record's `check_records` 0 violations

## observation contract

- **Expected values:** the compiles in each image and outside any, adding up to 274,834; UnityPlayer.dll's share.
- **Planted faults:**
  - (1) One image's count dropped from the record: `batch_record.rs` names the sum short of "compiled".
  - (2) A start at an image's end counted in it (the range's end taken as inside): a unit test names it.
  - (3) The counts by image left off a record after Sprint 248's: `batch_record.rs` names it.

## done criteria

Three records carry the compiles by image, adding up to "compiled", and UnityPlayer.dll's share of them is stated.

## self-review checklist

- [x] Scope names one verifiable property: the compiles by image on the record.
- [x] context_files cite the drain, the images, the vocabulary, the readers and the plan line.
- [x] Emits: three records.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: where the compiles come from.

## result
