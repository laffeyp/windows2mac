# Sprint 286 — The host's miss service under the runtime word

```yaml
---
id: 286
status: done
phase: 6
pass_kind: functional
---
```

## scope

Reviewer-3's Medium 1 at 285's close (BLACKBOARD Surfaced, this card's commit). The code cursor, the reconstruction
table and the dispatch table have writers inside the guest that hold the runtime word (the in-VM compiler,
`block_compiler.c`, and the installer, `monitor_code.rs`) and writers in the host that do not: `HostCompiler::compile`
and `place` (`compile.rs` 131, 164, 170), `install` (247) and `run::dispatch_insert` (395-406), which run while the host
serves `hvc 1`, after the miss stub released the word (`monitor_code.rs` 450, `release_runtime` before `cbz x25, host`).
On two vCPUs a host compile for one and an in-VM compile for the other can read the same cursor and write their code
over each other. The design's row says "every append and every host drain takes" the word; the host's appends do not.
On one vCPU nothing runs beside the host, and no run shows it.

- **The work:**
  - The run loop takes the runtime word (`host_take_runtime_word`, as the drain does since 275, its own vCPU's hold
    passed) before the miss service's install or compile and releases it after the dispatch insert, on every path,
    the five `break`s included: the service yields its result, the word is released, then the loop breaks or goes on.
  - `RunOut.host_word_takes` counts those takes, so a test sees the service take the word.
  - The way-0 pair read before a move, in the host's `dispatch_insert` and the C's, is one `ldp`
    (`ConcurrentGuestMemory::load_pair`, and inline assembly in `block_compiler.c`), so a pair moved to way 1 is whole
    whatever the dispatcher reads meanwhile; the installer's is already one `ldp`.
  - The design's table: the compiler-words and reconstruction rows name the host's compile, install and insert as
    writers under the runtime word; the dispatch row names the whole-pair read.
  - The step 6 Decision: the sixth precondition met; the second vCPU's race test adds a host compile on one vCPU
    against an in-VM compile on the other, the host compile without the word planted.
  - Reviewer-3's Low 1 at 285's close, answered by measurement: `w2m-vm` at 77d21ab2f, Sprint 278's close, against
    91802f99a, the game run with no ledger and no judge, alternated: 27.77 s and 25.21 s against 25.98 s and 25.56 s,
    loads 2.84 to 3.17, every stop "ReferenceEnd"; the design's goal paragraph records it.
- **Not here:** the remaining modules' slice calls; the race test (the second vCPU's card).

## prerequisites

- 285 (closed)

## context_files

- `crates/w2m_vm/src/run.rs` 395-406 (`dispatch_insert`), 455-515 (the drain's take), 1030-1080 (the `hvc 1` service)
- `crates/w2m_vm/src/compile.rs` 131, 164-170, 247; `crates/w2m_vm/src/monitor_code.rs` 440-451
- `tools/guest_runtime/block_compiler.c` (`dispatch_insert`); `crates/w2m_vm/src/concurrent_memory.rs`
- `docs/design-guest-memory-on-several-vcpus-2026-10-31.md` (the table); `crates/w2m_vm/tests/run_loop.rs` 195-260

## signal contract

### Emits

- None; a lock's coverage, held by tests and the one-vCPU runs.

### Invariants

- One-vCPU runs unchanged: the batch run's stop, stream and exit ledger; the image judge, `block_compiler.c` named.

## artifact contract

### Files modified

- `crates/w2m_vm/src/run.rs`, `crates/w2m_vm/src/concurrent_memory.rs`, `tools/guest_runtime/block_compiler.c`,
  `crates/guest_runtime/tests/python_image.rs` (its reason), `crates/w2m_vm/tests/run_loop.rs`,
  `docs/design-guest-memory-on-several-vcpus-2026-10-31.md`, `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `qpc`'s host compiles each under a take of the word, `host_word_takes` at least the host's
  compiles and installs; a pair read whole by `load_pair`; the run-loop tests and the batch run as before.
- **Planted faults**, each judge looked up to reach the line and to assert what the plant breaks:
  - (1) the miss service without the take: `run_loop.rs`'
    `qpc_runs_past_its_first_decline_compiling_as_the_python_host` runs `run_compiling` with a `HostCompiler`, whose
    misses reach the service, and gains an assertion that `host_word_takes` covers the host's compiles; with the take
    removed it counts 0.
  - (2) `load_pair` returning its two words swapped: `concurrent_memory.rs`' new test reads a stored pair back through
    it and names the words.

## done criteria

Every host writer of the code cursor, the reconstruction table and the dispatch table runs under the runtime word, a
moved pair is read whole, and the design's table names every writer and its guard.

## self-review checklist

- [x] Scope names one verifiable property: the host's miss service under the runtime word.
- [x] context_files cite the service, the drain's take, the host writers, the stub's release, the C and the table.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants, each judge's reach and assertion looked up.
- [x] One concept: the runtime word's coverage of the host's writers.

## result

- **The service** (98a25aa09): the run loop takes the runtime word by `host_take_runtime_word` before the install or
  compile and releases it after the dispatch insert; the service yields `Result<u64, Stop>` from a labelled block, so
  the five `break`s release the word before the loop stops.
- **The count, not the card's:** the card named `host_word_takes`, the host's takes, and an assertion that they cover
  the host's compiles. It failed at 6 takes for 8: at two misses the take returned at once, the vCPU holding the word
  already (`Err(holder) if holder == own`), and `host.translations` also counts the in-VM compiler's blocks, which
  `vm_compiled` pushes onto it. The count is `misses_outside_word`: the word read after the take, a miss counted when
  it holds neither `HOST_OWNER` nor the vCPU; the test asserts 0.
- **The reads:** `ConcurrentGuestMemory::load_pair`, one `ldp` and an acquire fence; the host's `dispatch_insert` and
  the C's read way 0 whole before a move; the image judge's reason for `block_compiler.c` names it.
- **The design's table:** the compiler-words row names every writer under the word, the host's miss service among
  them; the reconstruction and dispatch rows likewise; the goal paragraph records the game's timing (Low 1).
- **Expected values:** the 11 run-loop tests, `qpc` with 0 misses outside the word; the pair test.
- **The gates:** `scripts/gates.sh` exit 0 in 1059 s, kept in `store/gates/passes.tsv` as sprint 286 at 98a25aa09 with root=a49ad6bdb and "rust=all python=0 samples=0", every Rust test, 318 run, 318 passed, 1 skipped, base=91802f99a, Sprint 285's close; multi-vCPU: not run (Python gates not run)
- **Plant 1,** restored by `cp` and checked by `cmp`: the take replaced by `let took = false;`;
  `qpc_runs_past_its_first_decline_compiling_as_the_python_host` failed, "misses served outside the runtime word
  left: 6", the six misses the host took the word for.
- **Plant 2,** restored by `cp` and checked by `cmp`: `load_pair` returning `(high, low)`; the pair test failed, "the
  pair load_pair read left: (8738, 4369) right: (4369, 8738)".
