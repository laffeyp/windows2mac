# Sprint 275 — The host's drain under the runtime word

```yaml
---
id: 275
status: done
phase: 6
pass_kind: functional
---
```

## scope

Census row D6 (`docs/step-6-census-2026-10-30.md`; Deferred, Sprint 247, annotated at 248 and 261) names the in-VM
compiler's 23 counts, the miss stub's compile-time slots and the installer's shared words (the code cursor, the
reconstruction table's count, the compile log's count) as plain reads and writes that race on several vCPUs. This
card's lookup finds every one of them written inside the VM under the runtime word: `call_path` takes it before the
compiler or the installer and releases it after the probe (`crates/w2m_vm/src/monitor_code.rs` 356-381), so on two
vCPUs they do not race while the runtime word is held across a compile. What the word does not cover is the host:
`run::vm_compiled` reads the compile log and writes its count back to 0 at each exit (`run.rs` 426-475, called at
718) with no lock, while another vCPU may be appending under the word (`block_compiler.c` 237-314: read `n_log`, write
the entry, write `n_log + 1`). On two vCPUs an append between the drain's read and its reset is lost. This card has
the host take the runtime word around its drain.

- **Where it stands, by lookup (2026-10-31):**
  - The runtime word: a 32-bit word at `RUN_TABLE + RUNTIME_LOCK` (0x80), taken by `CASA` with the vCPU's owner
    (`MONITOR_DATA + LOCK_OWNER`) and released by `STLR WZR` (`lock_runtime`, `unlock_runtime`, `monitor_code.rs`
    110-134); the guest's memory is the host's mapping, so a host atomic on the same word is the same word.
  - `vm_compiled`'s callers: one, the run loop's exit service.
  - The counters, the probe's slots and the installer's words: each written between `take_runtime` and
    `release_runtime` in `call_path`.
- **The work:**
  - A host-side take and release of the runtime word, an `AtomicU32` compare-and-swap from 0 to a host owner and a
    release store of 0, on the word in the guest's memory.
  - `vm_compiled` holds it while it reads and resets the log.
  - Tests: the host's take waits while a guest owner holds the word and proceeds when it is released; the drain
    takes and releases it; the one-vCPU runs unchanged.
  - Census row D6 and Deferred (Sprint 247) annotated: the counters and words serialized by the runtime word, the
    drain now with them, and the re-visit moved to the card that takes the compile out from under the word (design
    change 3).
- **Not here:** splitting the runtime word (design changes 1 and 3).

## prerequisites

- 274 (closed)

## context_files

- `crates/w2m_vm/src/monitor_code.rs` (`lock_runtime`, `unlock_runtime`, `call_path`); `src/run.rs` (`vm_compiled`)
- `tools/guest_runtime/block_compiler.c` 237-314; `docs/step-6-census-2026-10-30.md` (D6)

## signal contract

### Emits

- None; a lock taken by the host, held by tests.

### Invariants

- One-vCPU runs unchanged: the batch run's stop and stream as held.

## artifact contract

### Files modified

- `crates/w2m_vm/src/run.rs` (and a module for the host's take), `docs/step-6-census-2026-10-30.md`, `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the host's take blocked while a guest owner holds the word, free after; the batch run
  unchanged.
- **Planted faults:**
  - (1) the host's take made a plain store: the waiting test names a take that did not wait.
  - (2) the drain without the take: its test names a drain that did not hold the word.

## done criteria

The host reads and resets the compile log only while it holds the runtime word, and the census says which shared
words the word serializes and when that changes.

## self-review checklist

- [x] Scope names one verifiable property: the host's drain under the runtime word.
- [x] context_files cite the lock, the call path, the drain and the C's append.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: the drain's lock.

## result

- **The finding by lookup:** the 23 counts, the compile-time slots and the installer's three shared words are written
  only between `take_runtime` and `release_runtime` in `call_path`, so census row D6's races do not happen while the
  runtime word is held across a compile; the host's drain was the one access outside it.
- **The take** (c6602b6b2): `monitor_code::host_take_runtime_word(mem, own)`, a compare-and-swap of the guest's word
  from 0 to `HOST_OWNER` (`u32::MAX`), spinning while another vCPU holds it and returning at once when the holder is
  `own`, the vCPU whose exit the host serves, which holds the word across a host request; `host_release_runtime_word`
  stores 0. Each makes its atomic for one access, so no reference into guest memory outlives the call.
- **The drain:** `run::vm_compiled` takes the word before it reads `VMC_LOG_COUNT` and releases it after the reset.
- **The tests:** the take waits while another owner holds the word and goes on when it is freed, does not wait for
  its own vCPU's hold, and takes a free word; the drain waits likewise. The `w2m_vm` library tests (76), the litmus
  tests and the run-loop tests pass.
- **Census row D6** (13674dd49) and Deferred (Sprint 247) annotated.
- **The gates:** `scripts/gates.sh` exit 0 in 685 s, kept in `store/gates/passes.tsv` as sprint 275 at 13674dd49 with root=f761c1d05 and "rust=crates python=0 samples=0", rdeps(signal_report) and rdeps(w2m_vm), 185 tests, base=6c9a419e1, Sprint 274's close; multi-vCPU: not run (Python gates not run)
- **Plant 1,** restored by `cp` and checked by `cmp`: the take made a plain store; the test failed, "assertion
  failed: waits_for_the_word(&mut held, |mem| { assert!(host_take_runtime_word(mem, 2)); })".
- **Plant 2,** restored by `cp` and checked by `cmp`: the drain without the take; the test failed, "assertion
  failed: waits_for_the_word(&mut held, |mem|".
