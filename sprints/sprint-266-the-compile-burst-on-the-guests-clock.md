# Sprint 266 — The compile burst counted on the guest's own clock

```yaml
---
id: 266
status: active
phase: 6
pass_kind: functional
---
```

## scope

Card 5's gate (`docs/design-translation-cache-2026-10-30.md` 170-173) reads the most compiling in a 60 Hz frame's span,
in `CNTVCT_EL0` ticks (`monitor_code.rs` 313-316), and the virtual counter runs while the host holds the vCPU off-core,
so the figure moves with the machine's load (reviewer-3's Medium at 264: 14.79 to 14.85 ms at load 3.3 to 4.2, 11.86 ms
at load near 2, the same uncached guest). This card gives the gate a figure the guest's instructions fix: the most
compiles inside any window of W instructions retired, installs counted apart, written in the design before any record
is read.

- **Where it stands, by lookup (2026-10-30):**
  - The instructions retired live in x23 as a budget left (`REG_BUDGET`), each block charged whole on entry; the
    monitor writes `limit - left` to `MONITOR_DATA` only at an exit (`run::exit_count`), so a compile inside the VM,
    which makes no exit, sees no current count there.
  - `struct log_entry` (`block_compiler.c` 31-42) is 120 bytes of the 0x80 slot (`VMC_LOG_ENTRY`): bytes 120-127 are
    free. The installer writes the same entry (`monitor_code.rs` 576-597).
  - `run::vm_compiled` reads every entry at each exit, so the host can keep one count a compile at no exit of its own.
  - The run's count is deterministic: 80,307,872,496 instructions with the cache, card 4's and Sprint 263's alike.
- **The readers:** card 5's gate; the Decision on whether step 5 needs mono-2.0-bdwgc.dll's cache.
- **The design first:** card 5's gate amended in a work commit before any record: W = 53,500,000 instructions, a
  60 Hz frame at 3.21 billion instructions a second, the fastest rate recorded (Sprint 263, the installer); the gate
  the most compiles in a window of W with the cache below the same figure without it, each from one untimed run.
- **The work:**
  - `call_path` (`monitor_code.rs` 356-381), after `blr x16` and when x25 is not 0, writes `[x28 + 8] - x23`, the
    instructions retired at the call, into bytes 120-127 of entry `[VMC_LOG_COUNT] - 1`: every nonzero return logged
    one entry, the C's only one after `v[VMC_LOG_COUNT] = n_log + 1` (`block_compiler.c` 314-317), the installer's
    after its count moves. One place for a compile and an install, no change to the C or the installer's words; an
    option off by default, as `Timing` is, so the frozen-word tests hold the default miss stub (design line 189).
    `struct log_entry` names the word, and a `_Static_assert` holds the entry at 128 bytes.
  - `vm_compiled` keeps the counts of compiles and of installs apart.
  - `record.rs`: `most_in_window(counts, w)`, the most counts inside any `[c, c + w)`, by two indices over the sorted
    counts; tested on a fixture whose answer is worked by hand.
  - Two rows on the record, `vm_compile_blocks` "most compiled in a 60 Hz frame of instructions" and "most installed
    in …".
- **Not here:** the Decision on mono-2.0-bdwgc.dll's cache; the timed figure, kept beside it.

## prerequisites

- 265 (closed)

## context_files

- `docs/design-translation-cache-2026-10-30.md` (card 5's gate)
- `tools/guest_runtime/block_compiler.c` (`struct log_entry`); `crates/w2m_vm/src/monitor_code.rs` (call path,
  installer); `src/run.rs` (`vm_compiled`, `exit_count`); `src/record.rs`

## signal contract

### Emits

- Two `vm_compile_blocks` variants on the record.

### Invariants

- The instructions retired unchanged, with and without the cache; the default miss stub's words unchanged.

## artifact contract

### Files modified

- `docs/design-translation-cache-2026-10-30.md` (card 5's gate), `tools/guest_runtime/block_compiler.c` (the entry's
  word named), `crates/w2m_vm/src/` (`monitor_code.rs`, `run.rs`, `record.rs`, `bin/w2m-vm.rs`), tests,
  `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** the figure with and without the cache, each run twice and equal.
- **Planted faults:**
  - (1) the window's end taken as closed: the hand-worked fixture names it.
  - (2) the install's count left unwritten: the installs' figure 0 on a cached run.

## done criteria

Card 5's gate reads the most compiles in a window of 53,500,000 instructions retired, installs apart, written in the
design before any record; a cached and an uncached run each give the figure, the same on a second run, and the record
carries it.

## self-review checklist

- [x] Scope names one verifiable property: the most compiles in a window of instructions.
- [x] context_files cite the gate, the log entry, the call path, the reader and the record.
- [x] Emits: two `vm_compile_blocks` variants.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: the burst on the guest's clock.

## result
