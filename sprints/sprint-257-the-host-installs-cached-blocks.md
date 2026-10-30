# Sprint 257 — The host installs cached blocks

```yaml
---
id: 257
status: active
phase: 6
pass_kind: functional
---
```

## scope

Card 4 of `docs/design-translation-cache-2026-10-30.md` as revised (G, H): the game run with `--cache` sends each
miss in UnityPlayer.dll's range to the host before the in-VM compiler, and the host installs the block from the cache
file, relocated into the code area with a host compile's bookkeeping, or compiles it as today when the cache does not
hold it. The design's card gates as amended (reviewer-3's Medium 1 at 256) are in this card's commit.

- **Where it stands, by lookup (2026-10-30):**
  - The miss stub binds `miss`, then makes its `host` label and tests `VMC_ENABLED` (`monitor_code.rs` 337-342); x16,
    x18 and x26 are free there (its comment at the restore); `hvc 1` reaches the run loop's lookup, `known_block`, then
    `HostCompiler::compile` (`run.rs` 959-975).
  - `compile` writes the words at the cursor, appends the marks to the reconstruction table, notes the tails, protects
    the pages and records the block (`compile.rs` 70-140); `cache_file::relocate` moves a cached block's words, all
    998,222 of UnityPlayer.dll's equal to their assembly in the code area (Sprint 256).
  - UnityPlayer.dll's range is known from the loader before the guest is built; the cache's key, which hashes the
    dispatcher, the slice stub and the runtime's calls, only after (`w2m-vm --make-cache`).
  - The batch run makes no write into UnityPlayer.dll's pages: one run from bcb4d42e5 with a temporary line in the
    `hvc 13` handler printed none, the line removed by `cp` and checked by `cmp`.
  - Searched for `monitor_code.rs`, `run.rs`, `compile.rs`, `cache_file.rs`, `w2m-vm.rs` in Deferred, Surfaced and
    `docs/surfaced-archive-2026-10-30.md`: Deferred (Sprint 251), card 4's half, whose IPA bound and fixed-base refusal
    concerned the regions the revision removed and whose second-page write plant becomes the write rule's test;
    Deferred (Sprint 247), the counters on several vCPUs, which an install adds to; census row 2 (the loader's tests)
    and row 15c (a full code area), re-visited in the result.
- **The readers:** the miss stub; the run loop's `hvc 1`; `HostCompiler`; `w2m-vm`; the record and its judges.
- **The work:**
  - `monitor_code`: with a cache range, the miss stub sends a target in `[base, base + size)` to the host before the
    in-VM compiler; with none, its words unchanged (the frozen-word test).
  - `w2m-vm --cache <dir>`: the range set from the loaded image, the file `<dir>/<key>.w2mcache` read once the guest is
    built, the run refused if no file has the key.
  - `HostCompiler::install`: the cached block relocated to the cursor, its marks rebased, its tails, pages and record as
    `compile` makes them; installs counted apart; a write to the image's code stops the cache's use for the rest of the
    run.
  - Three timed cold records with `--cache`, run alone; the counts and the time against Sprint 248's.
- **Not here:** the install inside the VM (card 5).

## prerequisites

- 256 (closed)

## context_files

- `docs/design-translation-cache-2026-10-30.md` (G, H, card 4); `crates/w2m_vm/src/monitor_code.rs` 330-380
- `crates/w2m_vm/src/compile.rs` 60-140, 240-270; `src/run.rs` 700-745, 950-980; `src/cache_file.rs`;
  `src/bin/w2m-vm.rs`; `scripts/batch-run-record.sh`

## signal contract

### Emits

- Three timed batch-run records with the cache: Sprint 248's measurements, the compiles by image, and the installs.

### Invariants

- Without `--cache`, the monitor code, the run and its record as at Sprint 256; each record with the cache conforms.

## artifact contract

### Files modified

- `crates/w2m_vm/src/` (`monitor_code.rs`, `compile.rs`, `run.rs`, `record.rs`, `bin/w2m-vm.rs`), tests,
  `scripts/batch-run-record.sh`, `docs/design-translation-cache-2026-10-30.md`, `BLACKBOARD.md`; three records

### Command exit codes

- `scripts/gates.sh` returns 0; each record's `check_records` 0 violations

## observation contract

- **Expected values:** installs; "compiled in UnityPlayer.dll" against 149,308; "compiled" against 274,837; the VM's
  compile time and the host's `hvc 1` seconds against Sprint 248's; the batch run conforming.
- **Planted faults:**
  - (1) An installed block's marks left at their cache offsets: the batch run's comparison or a give-back names it.
  - (2) The range test sending nothing to the host: the installs are 0 and the record's check names it.
  - (3) A write to the image's code not stopping the cache: the write rule's test names it.

## done criteria

The batch run with the cache conforms, its UnityPlayer.dll compiles fall by its installs, and its time against Sprint
248's is on the record.

## self-review checklist

- [x] Scope names one verifiable property: cached blocks installed by the host, the run conforming.
- [x] context_files cite the design, the stub, the host compile, the relocation and the maker.
- [x] Emits: three timed records.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the host's install.

## result
