# Sprint 341 — The call ring safe for concurrent callers

```yaml
---
id: 341
status: done
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

Done. `w2m_logged` (`tools/guest_runtime/runtime.c` 60-75) increments `per_api[api]` and reserves the record's index by
`__atomic_fetch_add(..., __ATOMIC_RELAXED)`, checks `i < capacity` on that index, and stores all three fields at
`rec[i]`. The judge is `run_loop.rs`' `the_call_ring_stays_exact_and_within_itself_on_two_vcpus`.

- **The reach, read before the work.** Every bridge takes the runtime word around its service (`guest::bridge`,
  `guest.rs` 577-581), and every logged service returns through `w2m_logged`. So no guest's calls reach `w2m_logged`
  from two vCPUs at once, and no guest or probe in `tests/data/timed_waits` or `probes/` can be the judge. The test
  drives the image's own `w2m_logged` (its symbol, `Guest::symbols`) from a loop on each of two vCPUs, without the word,
  as card 342's none-class bridge will call it: 17 words at `MONITOR_VA + 0x2400`, which meet at an arrival word, spin,
  then call `w2m_logged(api, calls left, tag)`. No new binary, so no walk fixture entry. The capacity is
  `(0x40000 - 0x1010) / 24` = 10,751 records (the card's 10,750 is one short; `process.rs` 396).
- **The memory order: relaxed.** The adds need only their atomicity: a read-modify-write on one location loses no
  update in any order. The host reads the ring with every vCPU stopped. GCC's `__atomic` builtins: `__ATOMIC_RELAXED`
  "implies no inter-thread ordering constraints". The Arm ARM: LDADD has no ordering; LDADDA and LDADDAL acquire;
  LDADDL and LDADDAL release.
- **The build.** The runtime's flags are `CFLAGS` (`crates/guest_runtime/src/lib.rs` 20-33), which hold
  `-ffreestanding -fno-builtin -mgeneral-regs-only` at `-O2`. Before (`build/guest_runtime_rust/22a9eaf0...bin`,
  `_w2m_logged` at 0x7f0004182050, 0x6c bytes): `ldr x10, [x9, #0x10]; add x10, x10, #0x1; str x10, [x9, #0x10]` for
  `per_api`, then `ldp x9, x10, [x8]` for the check and `ldr x9, [x8]` reloading `count` at ...090, ...09c and ...0a8
  before each field's store and the plain increment. After (`f46baecc...bin`, same address, 0x5c bytes):
  `ldadd x10, x9, [x9]` at ...06c (`per_api`), `ldadd x9, x9, [x10]` at ...07c (`count`, the old value in x9), then
  `ldr x8, [x8]` (capacity), `cmp x9, x8`, `madd x8, x9, x8, x10`, `stp x0, x1, [x8]` and `str x2, [x8, #0x10]`. No
  call, no stack (the image has no undefined symbol). Every runtime symbol after `w2m_logged` moves 0x10 lower. The
  image lies at `RUNTIME_IMAGE_VA` in the monitor area, outside the ledger's hashed range (`PROCESS_VA` to `TRACE_OFF`).
- **The two-vCPU run, before the change**: "round 0: the ring's count against the 16000 logged calls made, left:
  12278, right: 16000".
- **After**: "the call ring on two vCPUs: round 0's 16000 logged calls from an empty ring, count and per_api total
  16000; 19999 rounds of 8 from 10750 calls before, each count and total 10758; capacity 10751, the exception area
  unchanged". Each round compares the 0x1000 bytes from `EXCEPTION_OFF` with their value before the rounds; 13.4 s.
- **The judge's rounds.** Round 0 is the card's run: 8,000 calls a loop from an empty ring. Plant 1 under 2,000 such
  rounds wrote nothing past the ring; it failed only on the records. So each later round starts the ring one record
  short of capacity, with `count` 10,750 and those calls under `per_api[0x100]`, and makes 4 calls a loop. Each loop
  spins `(round * 37 or 61) % 1024` times after the meeting, since the last to arrive leaves first, ahead by more than
  a call's check and stores. Sixteen runs of plant 1 reached the exception area at rounds 41 to 13,096 of 20,000.
- **One vCPU.** The ring's bytes after each call are the plain update's: `per_api[api]` + 1, `rec[count]` filled below
  capacity, `count` + 1. The exit ledger and the batch record pass in the gates below.
- **Plant 1** "round 13096: the exception area changed, first at EXCEPTION_OFF + 0x0: 0x00 to 0x04". The check made
  on a reloaded `count`, the fields stored at `rec[r->count]`, the add atomic; run on the tree as gated, then restored
  and checked by `cmp`. In 64 rounds of the judge's first form it failed only as "records not one call's whole:
  [\"round 0, record 0: (0x101, 8000, 0xa000000000000002)\", ...]".
- **Plant 2** "round 0: the ring's count 12309 against the 0 calls before and 16000 logged calls made". `count` by a
  plain `r->count++`; run on the tree as gated, then restored and checked by `cmp`.
- **A third file.** The first gates run failed at `guest_runtime::python_image`'s
  `the_working_trees_c_departs_from_the_frozen_c_only_where_named` (`python_image.rs` 104): `runtime.c` was changed
  from `frozen-python-2026-10-29`'s and not named. `DEPARTURES` now names it with Sprint 341 and its reason. The
  frozen judge, `the_image_rust_builds_is_the_python_builds_byte_for_byte`, builds the C at the tag and is unmoved. The
  batch run's call stream names runtime addresses by symbol (`batch_run.rs` 214-219), so the 0x10 move compares equal.
  The first run's step rows stay in `store/gates/steps.tsv`; it wrote no pass row.
- **The gates** (copied from `tail -1 store/gates/passes.tsv`): `exit 0 in 369 s	default	2026-11-26T03:29:31Z	sprint 341	c62a25911799acead66312f1fe9858b5332c4e62	f77dff558b346ac24d298ccae1cc40f0ff7786aa138c58610ecad7073a2225a2	root=104052c95d212efd012615a5645bd62bd3936858	rust=all python=0 samples=0	base=5ba5b714f7011fbac219ee70d6cb711acb7fe5fc`
