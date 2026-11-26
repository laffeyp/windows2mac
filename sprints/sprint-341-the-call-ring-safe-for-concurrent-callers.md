# Sprint 341 — The call ring safe for concurrent callers

```yaml
---
id: 341
status: active
phase: 6
pass_kind: functional
---
```

## scope

Sprint 339's proof stopped Change 1's None class. All nine of its services, and every other logged service, return
through `w2m_logged` (`tools/guest_runtime/runtime.c` 60-70). That function writes the process-wide call ring
(`struct w2m_ring`, `runtime.h` 84-95, at `W2M_RING_VA`, `RING_OFF` 0x5000, size 0x40000) by unlocked load, add and
store. The built `_w2m_logged` reloads `count` before each field's store. On two vCPUs this has two faults: counts are
lost, and a record can be written past the ring, into the exception area at `EXCEPTION_OFF`.

The Decision "Card 341's form" (2026-11-25, reviewer-4's twelfth review) sets this card. It must be the atomic form,
because the exit ledger hashes the process area below `TRACE_OFF`, the ring included (`exit_ledger.rs` 16-17;
`process_below_trace`, `run.rs` 492-495). So the ring's bytes on one vCPU stay as they are.

- **The work:**
  - **`per_api[api]`** is incremented by `__atomic_fetch_add`.
  - **`count`.** A single `__atomic_fetch_add(&r->count, 1, ...)` reserves the record's index `i` and keeps
    `count`'s meaning, "records written, including any dropped past capacity". The capacity check is `i < capacity`,
    made on `i`, and all three fields are stored at `rec[i]`. No field's store reads `count` again.
  - **The memory order.** Pick it on the grounds: the GCC/Clang atomic builtins documentation and the Arm ARM's
    `LDADD` family. Record it.
  - **The build.** Confirm in the built image that `_w2m_logged` uses `ldadd` (or `ldaddal`) and calls no helper.
    Quote the disassembly before and after. Builder-339 probed this: `clang --target=arm64-apple-macos14 -O2
    -ffreestanding -fno-builtin -mgeneral-regs-only` gives `ldadd x8, x0, [x0]`, no call. Find the runtime's own build
    flags where `crates/guest_runtime` builds the C, and check against those.
  - **One vCPU unchanged.** On one vCPU every ring byte is written as before. So the exit ledger's fixture
    (`build/exit_ledger/python.tsv`, pinned by `store/exit_ledger/python.sha256`) and `batch_record.rs` pass unchanged.
    If the runtime image's own bytes lie inside the ledger's hashed range and its new code moves them, the result says
    so. It also says how the ledger's judge still holds, with the lines read.
- **The judge.** A run on two vCPUs makes more logged calls than the ring holds: `(0x40000 - 0x1010) / 24` = 10,750
  records, from two threads on two vCPUs at once. Use a guest or probe that does this; read the existing ones first
  (`tests/data/timed_waits`, `probes/`). If a new one is needed, it goes under the project's rules for a new binary,
  its walk fixture entry among them (Sprint 333). After the run the test reads the ring and the exception area from
  guest memory and asserts three things:
  - `count` equals the logged calls made;
  - `per_api`'s total equals `count`;
  - no byte from `EXCEPTION_OFF` to `EXCEPTION_OFF + 0x1000` changed from its value before the run.
- **Not here:**
  - The None class itself (card 342).
  - FlsSetValue's first set and QueryPerformanceCounter's clock mode 1, left on the word with card 342.
  - Any change to the ledger's hashed range.

## prerequisites

- 340 (closed)

## context_files

- `tools/guest_runtime/runtime.c` 55-75 (`w2m_logged`) and `tools/guest_runtime/runtime.h` 80-95 (`struct w2m_ring`,
  `W2M_RING_APIS`).
- `crates/guest_runtime/src/lib.rs` (how the C is built, its flags) and `crates/guest_layout`, `RING_OFF` to
  `TRACE_OFF` (100-117).
- `crates/w2m_vm/src/process.rs` 390-400 (the ring's count and capacity set up), `crates/w2m_vm/src/exit_ledger.rs`
  1-24 and 79-89, and `run.rs`' `process_below_trace` (492-495).
- Card 339's result (`sprints/sprint-339-the-thread-local-calls-take-no-runtime-word.md`), the proof and the built
  `_w2m_logged`'s disassembly.
- The Decision: `grep -n "Card 341's form" BLACKBOARD.md`.
- `crates/w2m_vm/tests/run_loop.rs` 1065-1250, the two-vCPU helpers and tests, the form a new test takes.
- `WORKING_AGREEMENT.md` lines 137-266, "Build and verification commands", with "A sprint closes in its work commit",
  "no gate row is dropped", and the rule on a new binary under `store/binaries/`.

## signal contract

### Emits

- None; how the runtime writes its call ring, no new event.

### Invariants

- On one vCPU the ring's bytes are as before, and the exit ledger and the batch run pass unchanged.
- Every test passes as before.

## artifact contract

### Files modified

- `tools/guest_runtime/runtime.c`, and a test in `crates/w2m_vm/tests/` (with a guest, if one is needed).

### Command exit codes

- `scripts/gates.sh` returns 0
- `target/release/signal_report --check` returns 0 on the tree

## observation contract

- **Expected values:**
  - The built `_w2m_logged`'s disassembly before and after, quoted.
  - The two-vCPU run's `count`, `per_api` total and logged calls made, quoted.
  - The exception area's bytes unchanged.
  - The exit ledger tests passing.
- **Planted faults:**
  - (1) The capacity check made on `count` reloaded (`if (r->count < r->capacity)`, with the fields stored at
    `rec[r->count]`) while the add stays atomic. The two-vCPU test fails on the exception area or on the count.
  - (2) `count` incremented by a plain `r->count++` again. The two-vCPU test fails on `count` against the calls made.

  Before the work, read each judge's reach. Name the guest's calls that reach `w2m_logged`, and check that a run
  passes 10,750 records.

## done criteria

`w2m_logged` writes the call ring safely from several vCPUs: each record at the index its atomic add reserved, the
capacity checked on that index, and its counts exact. On one vCPU the ring is written as before, and a two-vCPU run
past capacity leaves the exception area untouched.

## self-review checklist

- [x] Scope names one verifiable property: the ring exact and bounded under two vCPUs, unchanged on one.
- [x] context_files cite the C, the build, the layout, the ledger, card 339's proof, the Decision and the test form.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: the disassembly, the counts, the area, the ledger; two numbered plants, their reach read.
- [x] Sweet spot: `runtime.c` and a test; one concept, the ring's writes.

## result

GATES-LINE
