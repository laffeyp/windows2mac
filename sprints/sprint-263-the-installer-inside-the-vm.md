# Sprint 263 — The installer inside the VM

```yaml
---
id: 263
status: done
phase: 6
pass_kind: functional
---
```

## scope

Card 5 of `docs/design-translation-cache-2026-10-30.md`, its installer (choices III to VII; the file in the guest is
Sprint 261's): with a cache, a miss in UnityPlayer.dll's range is installed inside the VM from the mapped file, with
no exit, the host learning of it from the compile log. The reviews at 260 and 261 are disposed (Sprint 261's close).

- **Where it stands, by lookup (2026-10-30):**
  - The miss stub (`monitor_code.rs` 330-400) sends a target in the host range to `hvc 1` (Sprint 257); the compile
    path saves x0 to x15, x17 and x30 at `VMC_SAVE`, takes the runtime word, calls `w2m_vm_compile` through x16 with
    the target in x0, and branches to the code in x25 or to `hvc 1`.
  - The dispatcher's slot is 0x1800 to 0x1C00; `Guest::new` places the run's stubs in the code area through
    `cache.put` after the dispatcher is written, so the installer goes there and the stub reaches it through a monitor
    data word.
  - `dispatch_insert` (`block_compiler.c` 146-162, `run.rs` 378-393): the pair's first slot if empty or equal, else
    the second, else the first moved to the second.
  - The log entry (`block_compiler.c` 31-42): start, end, code, `recon_first`, `nmarks`, `count`, `cut`, `live_out`,
    `live_used`, `npages`, `pad`; the C moves its counts last. `vm_compiled` (`run.rs` 416-470) replays it at every exit:
    the block's start, its tails, its pages; it pushes the start to `translations` and `vm_compile_starts`.
  - `dc cvau, x0` is 0xd50b7b20 and `ic ivau, x1` 0xd50b7521 (clang, `--target=aarch64-linux-gnu`, 2026-10-30); the
    encoder has neither, nor `CTR_EL0`.
  - The file's layout (version 2): the header, then 40-byte blocks in start order, the words, the marks, the page
    index (variable), the fix-ups.
  - Searched for `monitor_code.rs`, `run.rs` (`vm_compiled`), `compile.rs`, `guest.rs` in Deferred, Surfaced and
    `docs/surfaced-archive-2026-10-30.md`: Deferred (Sprint 247), the shared words the installer adds to at step 6;
    Surfaced, the review at 261 (the both-ends test; choice I's figure).
- **The readers:** the miss stub; the run loop's replay; `HostCompiler`'s counts and write rule; the record; card 5's
  gate.
- **The work:**
  - `a64_encoder`: `dc_cvau`, `ic_ivau`, `CTR_EL0`, each held to clang's word.
  - The installer, monitor code placed by `Guest::new` when a run uses a cache: the stop word; the start by bisection
    over the file's blocks; the code area, the reconstruction table and the log checked for room; the words copied and
    the fix-ups patched by the move's distance (a branch past its reach declines); the marks rebased and appended; the
    words made visible by range; the log entry with `pad` 1; the counts moved last; the dispatch pair; the code
    returned, or 0 to send the target to `hvc 1`.
  - The section addresses and the image's base written by the host to monitor data words when the file is placed.
  - The miss stub, with the option on, calls the installer for a target in the range through the compile path; off,
    its words as before.
  - The replay: an entry with `pad` 1 counted as an install, its pages and start noted as a host install's, not a
    compile; `pad` cleared. A write sets the stop word.
  - Tests: the installer's words on small caches in a VM (`run_loop.rs`): a hit, a miss before the first block and
    after the last, the cached count exact against card 4's; plants named below.
  - Three timed records with the cache and three without, judge and ledger off, run alone; card 5's gate.
  - Choice I's figure moved to version 2's in a work commit (reviewer-3's Low 1 at 261).
- **Not here:** several vCPUs (Deferred, Sprint 247).

## prerequisites

- 262 (closed)

## context_files

- `docs/design-translation-cache-2026-10-30.md` (card 5, I to VII); `crates/w2m_vm/src/monitor_code.rs` 320-410
- `crates/w2m_vm/src/run.rs` 378-470; `src/compile.rs` (`install`, `stop_cache`); `src/guest.rs` 760-960;
  `src/cache_file.rs`; `tools/guest_runtime/block_compiler.c` 31-42, 146-176; `crates/a64_encoder/src/lib.rs`

## signal contract

### Emits

- Six timed batch-run records, three with the cache: the sequence of Sprint 262's, the installs now inside the VM.

### Invariants

- Without `--cache`, the monitor code's words and the run as at Sprint 262.

## artifact contract

### Files modified

- `crates/a64_encoder/src/lib.rs`, `crates/w2m_vm/src/` (`monitor_code.rs`, `guest.rs`, `run.rs`, `compile.rs`,
  `bin/w2m-vm.rs`), tests, `docs/design-translation-cache-2026-10-30.md`, `BLACKBOARD.md`; six records

### Command exit codes

- `scripts/gates.sh` returns 0; each record's `check_records` 0 violations

## observation contract

- **Expected values:** installs inside the VM against card 4's 128,456; `hvc 1` exits against card 4's 150,065;
  instructions retired against card 4's cached 80,307,872,496; the VM's compile time and the most in a 60 Hz frame
  against Sprint 248's 1,351 to 1,362 ms and 10.9 to 12.0 ms; the rate against Sprint 262's.
- **Planted faults:**
  - (1) A fix-up left unpatched: the hit test names the branch.
  - (2) The marks not rebased: the hit test names the mark.
  - (3) The stop word unread: the write test names an install after the write.

## done criteria

With a cache, UnityPlayer.dll's misses install inside the VM with no exit, the run conforming and retiring card 4's
cached count, and card 5's gate read on ledger-free timed records.

## self-review checklist

- [x] Scope names one verifiable property: installs inside the VM, the run conforming.
- [x] context_files cite the design, the stub, the replay, the host install, the log and the encoder.
- [x] Emits: six records.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and three plants.
- [x] One concept: the installer inside the VM.

## result

- **The installer** (229734920): `monitor_code::installer`, placed by `Guest::new` when `Start::install_in_vm` asks,
  reached by the miss stub through `VMC_INSTALLER` and `call_path`, the compile path now shared with
  `w2m_vm_compile` (its words without the option as before, the frozen-word tests passing). Its steps, the stop word,
  bisection, room, copy, fix-ups, marks, sync by range, the log entry with `pad` 1, the counts last, the dispatch pair,
  as choice III has them. `cache_file::sections` gives the file's tables, and `place_cache_file` writes them to the
  `VMC_CACHE_*` words; the replay counts a `pad` 1 entry as an install (`HostCompiler::installed_in_vm`) and clears
  `pad`; a write sets `VMC_INSTALL_STOP`. `a64_encoder`: `dc_cvau`, `ic_ivau`, `CTR_EL0`, held to clang's words.
- **Two faults found and fixed:**
  - Placed before the slice stub, the installer moved it, so the cache's key, which hashes the stub's address, named
    no file; it is placed last (72fc65bdd), held by `the_installer_moves_no_exit_a_cached_block_embeds` (c28554525).
  - The first batch run retired 79,815,503,799 instructions against card 4's 80,307,872,496; `ledger_parting` against
    Sprint 258's card 4 ledger named the 156th request, a `FILE_STAT` asked from a runtime stack 0x50 bytes shallower.
    The C declines the target at `VMC_HOST_ONLY`, the call trace's first block, which the host's compile opens the trace
    at; the installer had installed it. It declines it now (72fc65bdd), held by the installer test. KIT_DIARY #110.
- **The C's declines against the cached path** (reviewer-3's Low 1 at 263):

  | the C declines | the cached path |
  |---|---|
  | `VMC_HOST_ONLY`, the trace's first block | the installer declines it |
  | not a code page, a page needing a split | card 4 makes every executable page of the image read-only at load |
  | the log full | the installer declines |
  | a decode refusal | no cached block starts at an instruction that does not lift (6,339 left out): not a cached start |
  | a block starting at PAUSE, for its yield | the cache's blocks are cut as the host's compiles are, and the Rust lowering of `Op::Pause` yields (`lower.rs` 2058) |

- **The tests:** `the_installer_installs_a_cached_start_and_declines_the_rest` runs the installer alone in a VM over
  `qpc`'s cache: the words `relocate` gives at the code, the marks rebased, the log entry an install, the dispatch pair,
  and declines below the image, before the first block, after the last, and at `VMC_HOST_ONLY`;
  `qpc_runs_with_its_blocks_installed_inside_the_vm`: the run's stop, its instructions equal to the host install's, fewer
  `hvc 1`, and a write stopping the installer.
- **The records:** six from c28554525, judge and ledger off, alternating, run alone, load at start 3.31 to 4.21:
  without the cache `store/runs/w2m-vm-game-1793419013000000000`, `-1793419066000000000`, `-1793419118000000000`;
  with it `-1793419040000000000`, `-1793419092000000000`, `-1793419145000000000`; each conforms, `check_records` 0
  violations.

  | | without the cache | with the installer |
  |---|---|---|
  | rate, instructions a second | 3.082, 3.106, 3.107 billion | 3.200, 3.184, 3.210 billion |
  | "vm" run seconds | 23.72, 23.53, 23.52 | 22.72, 22.82, 22.63 |
  | host `hvc 1` | 0.041 s, 1,091 exits | 0.088 s, 4,498 exits |
  | compile time, installs included | 1,296, 1,295, 1,285 ms | 694, 685, 678 ms |
  | the most in a 60 Hz frame | 14.81, 14.79, 14.85 ms | 14.42, 14.61, 14.85 ms |
  | instructions retired | 80,306,813,574 | 80,307,872,496, card 4's |

  Installs 145,570 a run against card 4's 128,456; "compiled" 125,515 as at card 4.
- **Card 5's gate:** the compile time below Sprint 248's, met; the most in a 60 Hz frame below 248's 10.9 to 12.0 ms,
  not met: 14.4 to 14.9 ms, unmoved against the same batch's uncached 14.8 (and 248's at a load near 2). Step 5 is not
  met; Deferred (Sprint 263) names what fills the worst frame as the next card.
- **A void batch:** the first six records' fourth refused its cache, "No such file or directory": the placement test
  was added to `run_loop.rs` while they ran, and the key hashes the crates' tree. The four records made were removed and
  all six made again after the test's commit.
- **Files beyond the artifact contract:** `crates/w2m_vm/src/cache_file.rs` (`sections`, `BLOCK_BYTES`, the key's
  width asserted in `to_bytes`) and `src/process.rs` (`Start::install_in_vm`).
- **Choice I's figure:** version 2's in a work commit (da9348db9).
- **The gates:** `scripts/gates.sh` exit 0 in 693 s on 26a531615, copied from `store/gates/passes.tsv`; the timing stage 666 ns an exit at load 3.82.
- **Plants,** run on 26a531615, each restored by `cp` and checked by `cmp`.
- **Plant 1,** `the_installer_installs_a_cached_start_and_declines_the_rest` failed, "block 0x140001040's word 23":
  the fix-up's `bfi` made a `mov x11, x11`.
- **Plant 2,** the same test failed, "block 0x140001040's mark 0 … left: (288, 5368713280)": the marks' rebase made a
  `mov x11, x11`.
- **Plant 3,** `qpc_runs_with_its_blocks_installed_inside_the_vm` failed, "the installer installed after the write":
  the stop word's `cbnz` made a `mov x1, x1`.
