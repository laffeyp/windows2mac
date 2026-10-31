# Sprint 257 — The host installs cached blocks

```yaml
---
id: 257
status: done
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

- **The install:** 67c09917b. `monitor_code::Options { timing, host_range }`: with a range, the miss stub compares the
  target with both ends and branches to the host path before the in-VM compiler; without one, its words are as before
  (`the_host_range_is_off_by_default_and_sent_before_the_compiler`). `HostCompiler::install` relocates the cached block
  to the cursor, rebases its marks and places it through `place`, the half of `compile` both now share; installs are
  counted apart and are not `translations`. `w2m-vm --cache <dir>` sets the range from UnityPlayer.dll before the guest
  is built and reads `<dir>/<key>.w2mcache` under the key `--make-cache` uses (`cached_image`, shared), refusing the
  run without it; the record carries `vm_compile_blocks` "installed from the translation cache".
- **The write rule, wider than the card:** the card's first count, taken in the `hvc 13` handler, saw only pages already
  behind code. `HostCompiler::use_cache` makes every executable page of the image read-only at load, outside the code
  page set; the first write to one stops the cache, drops every block installed from it (an installed block's live
  flags rest on its successors' bytes, which need not lie on its own pages) and unguards the pages no block is behind.
  The design's card 4 gate says so (b1135620d, a work commit).
- **Files beyond the artifact contract:** `guest.rs`, `monitor.rs` and `process.rs` (`Start::host_range`, passed to the
  monitor code), `cache_file.rs` (`CACHED_IMAGE`, 1b3ebdb48), `tests/run_loop.rs`.
- **The records:** three timed cold records from 67c09917b with `--cache build/translation-cache`, run alone, load at
  start 2.17 to 2.51, nothing busy: `store/runs/w2m-vm-game-1793401027000000000`, `-1793401307000000000`,
  `-1793401572000000000`; each conforms, `check_records` 0 violations. The cache remade under the tree, key `9dfb9d30…`,
  998,222 blocks, all relocating equal. Each run installs 128,456 blocks (the design's bound 128,523 distinct starts);
  "compiled" falls from 274,837 to 125,515; "compiled in UnityPlayer.dll" from 149,308 to none; mono-2.0-bdwgc.dll
  41,830 against 41,840 and outside any image 82,018 against 82,022 (Sprints 249 and 250); no write stopped the cache.
- **The run's path moves with the cache:** each way the run is deterministic, and the two differ. Instructions retired:
  80,306,813,574 on every record without the cache (Sprint 249's, the last gates record), 80,307,872,496 on every record
  with it, 1,058,922 more. Two untimed runs with a temporary line in `w2m-vm.rs` (removed by `cp`, checked by `cmp`)
  listed the in-VM compiles outside UnityPlayer.dll: all differences lie in mono-2.0-bdwgc.dll, 17 starts compiled only
  without the cache, 9 only with it, 14 compiled a different number of times. Both runs reach the reference's end. The
  cause is not traced here; Deferred (Sprint 257) holds it for card 5.
- **The first records' time, confounded:** the three records above ran the whole-block judge, whose host work
  `run.rs` charged to the next exit's kind, and Sprint 248's did not; their host seconds (`hvc 1` 0.580 to 0.585,
  `hvc 13` 0.694) and rate (4.1% to 4.4% under 248's) cannot be read as the install's (reviewer-3's Medium 1 at 257).
  The gates on 2cb9047f3 were stopped; a202b049a counts the judge's host time as its own kind, "judge", and a timed
  record leaves the judge out.
- **The time, like for like:** six timed records from a202b049a, judge off, run alone and alternating, load at start
  1.94 to 2.38: without the cache `store/runs/w2m-vm-game-1793403313000000000`, `-1793403692000000000`,
  `-1793404070000000000`; with it `-1793403413000000000`, `-1793403792000000000`, `-1793404169000000000`; each
  conforms, `check_records` 0 violations. The cache remade under the tree, key `21a89b29…`.

  | | without the cache | with it |
  |---|---|---|
  | rate, instructions a second | 2.946, 2.970, 2.969 billion | 2.929, 2.933, 2.922 billion |
  | "vm" run seconds | 24.33, 24.18, 24.21 | 24.30, 24.28, 24.36 |
  | host `hvc 1` seconds | 0.049, 0.047, 0.047 | 0.291, 0.290, 0.294 |
  | the VM's compile time | 1,385, 1,363, 1,369 ms | 674, 673, 667 ms |
  | the most in a 60 Hz frame | 11.8, 13.0, 11.7 ms | 11.5, 11.6, 10.7 ms |

  The host's install is slower overall: the rate falls 1.1% on the means (2.928 against 2.961 billion), about 0.3 s
  over the run. The VM's compile time halves and the frame's worst falls little, but the VM's run seconds do not fall
  with the 700 ms of compiling that leave it, and the host's `hvc 1` seconds rise by 0.24 over 149,000 more exits. So
  card 4 meets its gate's counts and states its time, a slower run named; card 5, the install inside the VM with no
  exit, carries the speed, as the Decision's annotation at 256 set (step 5 met at card 5's close).
- **The census rows:** row 2 (the loader's tests): the cache loads with the image under its key, and the host range
  comes from the loader's image; no loader change. Row 15c (a full code area): the cache's code, 112.6 MB, does not fit
  the 64 MiB area whole; the run installs what it reaches, and "code cache full" reads 0.
- **A departure in plant 2:** the card expected the range test planted to send nothing to give 0 installs. It gave 60,
  starts in the image the in-VM compiler declined, which still reach the host's `hvc 1` and the cache, and
  `batch_record.rs` passed. The judge now names a "compiled in UnityPlayer.dll" row beside installs (1b3ebdb48), since
  with the cache in use the in-VM compiler compiles nothing in that image.
- **The gates:** `scripts/gates.sh` exit 0 in 559 s on 2cada5aa4, copied from `store/gates/passes.tsv`; the timing stage 650 ns an exit at load 3.65.
- **Plants,** run on the committed tree, each restored by `cp` and checked by `cmp`, the planted record removed.
- **Plant 1,** `qpc_runs_with_its_blocks_installed_from_the_cache` failed, "no mark (0x7f00000625b8, 0x140001000): block
  0x140001000's marks are not rebased": the marks left at their cache offsets (`compile.rs`, on 67c09917b).
- **Plant 2,** `batch_record.rs` failed, "vm_compile_blocks \"compiled in UnityPlayer.dll\" is 149308 beside the
  installs: the miss stub kept the image from the host": the range test's branch to the host sent to `outside`, its
  cache and record made under the planted tree (on 1b3ebdb48; on 67c09917b the judge passed it, the departure above).
- **Plant 3,** `a_write_to_the_image_code_stops_the_cache` failed, "assertion `left == right` failed: the cache did not
  stop … left: None": `stop_cache`'s `stopped_at` left unset (on 67c09917b).
