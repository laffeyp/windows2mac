# Sprint 142 — The run loop, to the first declined block

```yaml
---
id: 142
status: active
phase: 6
pass_kind: functional
---
```

## scope

The run loop's last piece and the first gate of `docs/design-host-compile-path-2026-10-24.md`'s order (link 1; split
from Sprints 140 and 141 by technique #17). `crates/w2m_vm/src/run.rs` runs one vCPU and one thread of a `Guest`
(Sprint 141), directly through `hypervisor`, as `VM.steps` does for that case (`src/w2m/vm_backend.py` 4916-5127):

- the step budget (`_set_budget`, 3284-3289);
- the first thread's run-table entry (`_add_thread`, 3328-3344; `GuestThread`, `scheduler.py` 60-71; `_vslot`,
  3346-3350; `set_current`, 3323-3326);
- the thread's start words (MXCSR 0x1F80, FCW 0x27F, the lock owner; 4973-4977);
- the vectors zero and FPCR from MXCSR (`fpcr_for`, 1138-1143);
- the system registers;
- the C compiler in the VM enabled (`VMC_ENABLED`, 4968-4969);
- the VM compiler's symbol table (2509-2510) and the COM vtables (2488, 3078-3085), which Sprint 141 left;
- the entry registers (4998-5004);
- the run words (`_before_run`, 5129-5141).

Then it serves exits. At each exit it takes in the C's compile log (`_vm_compiled`, 2559-2580) and keeps the count
from x23 (`serve_exit` 5197-5208). On `hvc #1` it inserts a thunk target's bridge (`_dispatch_insert`, 4827-4849) and
resumes at `miss_va + 4`, or stops, naming the target, when the target is neither a bridge nor a block. Every other
exit stops by its kind and number. The host's compiler is the translator's sprints' work; the stop is this link's
gate.

## prerequisites

- 141 (closed)

## context_files

- `src/w2m/vm_backend.py` 1138-1143, 2509-2510, 2559-2580, 3078-3093, 3284-3289, 3316-3350, 4827-4849, 4916-5141,
  5190-5245 (read in full); `src/w2m/scheduler.py` 40-71; `tools/guest_runtime/block_compiler.c` 26, 109, 231-233
- `crates/w2m_vm/src/guest.rs`, `monitor_code.rs`; `tests/data/timed_waits/qpc.c`

## signal contract

### Emits

None yet: the record and its golden check come at the skeleton's gate.

### Consumes

`Guest`; the runtime image.

### Invariants

- No Python file added or changed; no Python run or written.

## artifact contract

### Files created

- `crates/w2m_vm/src/run.rs`, `crates/w2m_vm/tests/first_decline.rs`

### Files modified

- `crates/w2m_vm/` (`guest.rs`: the symbol table and vtables), `WORKING_AGREEMENT.md`, `BLACKBOARD.md` (Sprint 141's
  CI run, in the tail entry)

### Content assertions

- Each function cites the `vm_backend.py` lines it carries.

### Command exit codes

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release` return 0
- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** `qpc` runs from its entry to an `hvc #1` whose x17 is an address inside `qpc.exe`'s `.text`
  that is neither a bridge nor a block, and the run names it. At that stop the C's counts (`VMC_COUNTS`) show
  "compiled" at least 1 and "no stencil" plus "cut by no stencil" plus "page needs a split" above 0. "Not a code page"
  equals the bridged thunk targets the run served. Every exit before the stop is `hvc #1`. The run reports each
  `hvc #1` round trip's time, from the vCPU's exit to its next run, against the C monitor's 813-852 ns an exit
  (Sprint 132), and the guest time between exits.
- **Planted fault:** the dispatch insert writes the bridge into the wrong pair: the thunk misses again, and the run
  stops naming the thunk's address as a repeated miss.

## done criteria

The Rust host runs a Windows program's code in the VM, compiled by the C compiler there, until the first block that
needs the host's compiler, and names that block.

## self-review checklist

- [x] Scope names one verifiable property: `qpc` runs to its first declined block, named.
- [x] context_files cite every Python line carried.
- [x] Emits: none yet, said so.
- [x] Artifact contract gradable by command.
- [x] Observation contract: expected values from the C's own counts, and a planted fault.
- [x] One concept: the run loop up to the host's compiler.

## result

- `crates/w2m_vm/src/run.rs`: `run_to_first_decline` (one vCPU, one thread, through `hypervisor`), `fpcr_for`,
  `dispatch_insert`, the first thread's run-table entry, the compile log taken in, the count kept from x23.
  `guest.rs` gains the COM vtables (their 54 methods' thunks and bridges after the loader's) and the VM compiler's
  symbol table, read from `stencil_table.h` as `_vm_compiler_symbols` reads it.
- Expected values (`tests/first_decline.rs`, one VM, 0.35 s): `qpc` ran from its entry; the C compiled 0x140001000
  and 0x14000100F; one miss, `QueryPerformanceCounter`'s thunk 0x7FFE00000020, was bridged; the run stopped at
  0x140001055, inside `qpc`'s `.text`, neither a bridge nor a block: `cmpq %r8, %rax; jne`, the hex loop's bottom,
  where the C cut the block from 0x14000100F. The C's counts: compiled 2, no stencil 1, cut by no stencil 1, not a code
  page 1 (equal to the one bridged thunk), pages protected 1. Both exits were `hvc #1`.
- Measured: one served exit, 3,667 ns from the vCPU's exit to its next run (the C monitor's exit alone was 813-852 ns,
  Sprint 132); two runs, a median of 140,750 ns, the first holding the vCPU's first entry. One sample is not a
  distribution; the skeleton's later links serve thousands of exits and measure it properly. A guest WFE was not
  reached.
- Not shown: that 0x140001055 is the block the Python host compiled first for `qpc`. That needs the frozen host run,
  which waits on the user's answer about inline Python; the stop's cause is shown instead by the C's own counts and
  the instruction there.
- Planted fault: the bridge inserted one pair off: "the run stopped otherwise: Repeated { target: 140728898420768 }"
  (0x7FFE00000020); restored, the test passes.
- Found on the way: the first `guest.rs` edit used a perl substitution with `|` as its delimiter and `\|` in the
  pattern, which spliced code into the licence header, the error class recorded before; the file was restored from
  Sprint 141's commit and the edits made with the Edit tool, and every other changed file's first line was checked.
