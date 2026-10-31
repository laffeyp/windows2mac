# Sprint 258 — Where the cached run parts from the uncached

```yaml
---
id: 258
status: done
phase: 6
pass_kind: observation
---
```

## scope

Deferred (Sprint 257), reviewer-3's Low 1 at 257, card 5's first item by name: the batch run with the translation
cache retires 80,307,872,496 instructions and without it 80,306,813,574, each deterministic, and its in-VM compiles in
mono-2.0-bdwgc.dll differ. This card finds the first exit where the two runs part and names, by measurement, what makes
them part there, before card 5 changes the same path.

- **Where it stands, by lookup (2026-10-30):**
  - The exit ledger (`exit_ledger.rs`, `--exit-ledger`) writes a row a served exit: its sequence, kind, x86 site, the
    instructions executed and the clock's warp at the exit, a request's words, and the sha256 of the process area.
  - With the cache, the miss stub sends UnityPlayer.dll's misses to the host, so the cached run has about 149,000 more
    `hvc 1` rows; the two ledgers line up only on the other kinds.
  - The C compiler and the cache cut blocks by different rules: the cache's blocks run to a branch or 64 instructions
    (`cache_file::lift_block`); the C's also end at a page it may not read (`cut by guarded`, `page needs a split`) and
    at a decode it refuses (`cut by decode`).
  - Searched for `exit_ledger.rs`, `run.rs`, "diverge", "path" in Deferred, Surfaced and
    `docs/surfaced-archive-2026-10-30.md`: Deferred (Sprint 257), this item; nothing else on two runs of one guest
    parting.
- **The readers:** card 5 (the install inside the VM), whose records are read against 257's; the design's card 5 gate.
- **The work:**
  - Two untimed batch runs from one tree, without and with the cache, each with `--exit-ledger`; their rows of every
    kind but `hvc 1` compared in order; the first row that differs in kind, site, instructions executed or the process
    area's hash named, with the thread it ran on.
  - From that row back, the cause measured, not inferred: what the two runs did differently before it, by whatever the
    ledger or a temporary line shows; each temporary line removed by `cp` and checked by `cmp`.
  - The finding on the record: in the card's result, and, when it is a defect, a test that holds the fix; when it is a
    conforming schedule a cached run may take, the design's card 5 gate says so.
- **Not here:** the install inside the VM (card 5).

## prerequisites

- 257 (closed)

## context_files

- `crates/w2m_vm/src/exit_ledger.rs`; `src/run.rs` 600-760; `src/compile.rs` (`install`, `place`); `src/cache_file.rs`
  (`lift_block`); `tools/guest_runtime/block_compiler.c` (`extent`)
- `docs/design-translation-cache-2026-10-30.md` (card 5); BLACKBOARD Deferred (Sprint 257)

## signal contract

### Emits

- None of its own: two exit ledgers, compared; the batch run's record unchanged.

### Invariants

- No run's behaviour changed unless the finding is a defect, and then only by its fix and its test.

## artifact contract

### Files modified

- `sprints/sprint-258-where-the-cached-run-parts.md` (the result); a fix and its test if the finding is a defect;
  `docs/design-translation-cache-2026-10-30.md` if the finding bears on card 5's gate; `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the first parting row's sequence in each ledger, its kind, site, thread and instructions
  executed; the cause, measured.
- **Planted faults:**
  - (1) The comparison given two ledgers that differ at one planted row: it names that row and no earlier one.

## done criteria

The first exit where the cached run parts from the uncached is named, and what makes it part is measured and on the
record, with a fix and its test if it is a defect.

## self-review checklist

- [x] Scope names one verifiable property: the first parting exit and its measured cause.
- [x] context_files cite the ledger, the run loop, the install and the two block-cutting rules.
- [x] Emits: none of its own.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and one plant.
- [x] One concept: where and why the two runs part.

## result

- **Two ledgered runs** from 2cada5aa4's tree, without and with the cache (`--exit-ledger`), each to the reference's
  end: 54,332 and 203,374 rows, 38,881 and 38,950 host requests.
- **A first reading, wrong:** compared row by row on every kind but `hvc 1`, the two first differed at a request at
  0x18143cb4c, 765 against 746, early in UnityPlayer.dll's start. A bisection of the host range (a temporary
  `W2M_RANGE` and `W2M_ONLY`) narrowed it to the block at 0x18143cb4c, and a temporary exit print then showed both runs
  entering the same services at the same counts, 517, 746 and 765, on thread 4. A row's monitor word is the count the
  next exit finds (`exit_ledger.rs`' columns), and the cached run's extra `hvc 1`s change which exit is next; the
  frozen host's ledger has the same exits as the Rust host's, so the comparison it was made for is unharmed.
- **The host's answers vary by run:** paired with the count its own bridge stored, the previous row's, the requests
  first differed at the 175th, a `FILE_STAT` (op 24) of `C:\w2m`, the run's own temporary directory, whose times are
  the host's clock at the run's start (0x01dd68d1676cd2c6 against 0x01dd68d167e7b802), and then at an inode number in
  op 7's answer. Three runs of each kind retire identical counts, so neither steers the guest; the comparison leaves
  the host's answers out.
- **The first parting:** what the guest asks, op, four arguments and entry count, is the same through 5,538 requests.
  The 5,539th, `FILE_READ` (op 9) of file 0xce, 0x800 bytes at 0x306000, comes at 20,683,462,044 instructions without
  the cache and 20,683,462,103 with it; the bytes read before it are the same (equal hashes), and the next read's count
  differs by −22. A temporary print at the two requests: both on thread 4, no exit between them, the VM's own switch
  count 10,076 to 10,090 in both, the slice's end 20,683,481,926 in both.
- **What makes it part, measured:** the same 14 in-VM switches give the threads other shares of instructions. A slice
  ends at a block's stop (`threads.c` 17-18), each block charged whole on entry (`assemble_block`'s `SUBS x23`), and
  the cache cuts blocks where the in-VM compiler does not: the request before the parting shows its site as 0x180595000,
  a page boundary the in-VM compiler cuts at, and 0x180594fd6, the cached block's start running through it. From there
  the threads interleave otherwise; the cached run retires 1,058,922 more instructions and reaches the reference's end.
  Not a defect: another schedule of the same guest, conforming. The design's card 5 gate says so and adds that card 5's
  runs retire card 4's cached count exactly (024db72c5).
- **The comparison kept:** `exit_ledger::asked` and `first_parting`, with `the_first_parting_is_the_first_request_asked_differently`,
  and `ledger_parting <a> <b>`, which on the two ledgers prints request 5538 (from 0), file 0xce at 0x306000, 20,683,462,044
  against 20,683,462,103 (27c5ab477).
- **Files beyond the artifact contract:** `crates/w2m_vm/src/exit_ledger.rs` (`asked`, `first_parting`, their test) and
  `src/bin/ledger_parting.rs`. The contract named code only for a defect's fix; the finding is no defect, but the
  observation contract's plant needs a judge to fail, and the by-hand `awk` comparison that found the parting had
  none, so the comparison became code with a test.
- **The temporary lines:** five, in `compile.rs`, `w2m-vm.rs`, `run.rs`, `exit_ledger.rs` and `file_bridge.rs`, each
  file backed up by `cp` first and restored and checked by `cmp` before any commit; each change of a crate's tree remade
  a cache under it for the cached probes.
- **The VM's time (reviewer-3's Medium 1 at 258):** not this card's; Deferred (Sprint 258), card 259 before card 5.
- **The gates:** `scripts/gates.sh` exit 0 in 633 s on 024db72c5, copied from `store/gates/passes.tsv`; the timing stage 652 ns an exit at load 3.71.
- **Plant,** run on 27c5ab477, restored by `cp` and checked by `cmp`.
- **Plant 1,** `the_first_parting_is_the_first_request_asked_differently` failed, "assertion `left == right` failed: the
  request at offset 0x1000, asked at another count … left: 1 right: 2": each request's count read from its own row, the
  comparison named an earlier request than the planted one.
