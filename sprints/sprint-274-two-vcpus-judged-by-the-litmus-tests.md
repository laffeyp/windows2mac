# Sprint 274 — Two vCPUs judged by the litmus tests

```yaml
---
id: 274
status: done
phase: 6
pass_kind: functional
---
```

## scope

The Decision "Sequence for step 6" (as annotated at 271): after the XCHG (272) and the gates' base (273), the Rust
host runs several vCPUs at once for the first time, judged first by the litmus tests MP, SB, LB, 2+2W and IRIW and
their EnTSO plant (census row G1), before any lock is split. The frozen host's judge, `tests/test_litmus.py` at
`frozen-python-2026-10-29` (Sprint 105), needs no guest process: x86 assembled by clang, compiled by w2m, each thread on
a vCPU of one VM against shared words. This card ports it to the Rust host, with reviewer-3's Low 1 at 272: a
misaligned memory XCHG raced on two vCPUs.

- **Where it stands, by lookup (2026-10-31):**
  - The frozen judge (190 lines): HEAD, an arrival counter (`lock inc`, a `cmp` and `pause` loop); each test's
    bodies; TAIL, `jmp` to DONE, which no block holds. vCPU 0 runs a one-instruction program; thread k runs on vCPU
    k + 1 from `dispatch_va` with rcx the iterations, rdi the data, r9 the threads to wait for, x17 the entry, x28 the
    monitor data, the budget 1 << 62; at `hvc 1` it stops at DONE or compiles under one lock and resumes at
    `miss_va + 4`; at `hvc 4` it renews the budget. Each litmus vCPU's `SLICE_UNTIL` is set to none ready, or every
    PAUSE left the VM. Each thread runs alone once, then all at once, 100,000 iterations.
  - Its figures (Sprint 105): MP, LB, 2+2W and IRIW's forbidden outcome 0 in 2,000,000 each over 20 runs; SB's relaxed
    outcome 1 to 36 a run, above 0 in up to five runs; EnTSO cleared, MP's forbidden outcome above 0 in up to ten runs
    (three missed about once in 600 and failed the gates once, Sprint 110).
  - The Rust `hypervisor` crate: a `Vcpu` is neither `Send` nor `Sync` (`lib.rs` 12, 362, a compile_fail doc test), made
    on its own thread; `stop_from_another_thread.rs` holds `hv_vcpus_exit` from another thread; no test runs two vCPUs.
  - The Rust translator: locked forms and, from 272, a memory XCHG lower to the exclusive pair; a misaligned one steps
    in the model at `hvc #11`, atomic on one vCPU only (Deferred, Sprint 272).
- **The work:**
  - `crates/w2m_vm/tests/litmus.rs`: the five tests and the plant as the frozen judge has them, the x86 assembled by
    clang at test time, compiled by the Rust host's compiler, one vCPU thread a litmus thread, the counts held to
    x86-TSO's rules (Owens, Sarkar and Sewell, "A Better x86 Memory Model: x86-TSO", TPHOLs 2009).
  - A misaligned XCHG raced: two threads each exchanging into one word that straddles a 16-byte line, the values
    seen and left conserved; its result recorded, not fixed here.
  - Census row G1 and Deferred (Sprint 272) annotated with what the tests showed.
- **Not here:** the in-VM compiler's shared counters (275); a guest process on two vCPUs.

## prerequisites

- 273 (closed)

## context_files

- `tests/test_litmus.py` at `frozen-python-2026-10-29`; `crates/hypervisor/src/lib.rs`; `crates/w2m_vm/src/run.rs`,
  `src/guest.rs`, `src/monitor_code.rs`; `crates/translator/src/lower.rs`; `docs/step-6-census-2026-10-30.md` (G1)

## signal contract

### Emits

- None; the memory model on several vCPUs, held by tests.

### Invariants

- One-vCPU runs unchanged.

## artifact contract

### Files modified

- `crates/w2m_vm/tests/litmus.rs` new, `crates/w2m_vm/src/` as the harness needs, `docs/step-6-census-2026-10-30.md`,
  `BLACKBOARD.md`

### Command exit codes

- `scripts/gates.sh` returns 0

## observation contract

- **Expected values:** MP, LB, 2+2W and IRIW's forbidden counts 0; SB's relaxed count above 0; the misaligned XCHG's
  conservation, held or broken, as measured.
- **Planted faults:**
  - (1) EnTSO cleared on every vCPU: MP's forbidden count above 0, the frozen judge's plant.
  - (2) the threads run one after another, not at once: SB's relaxed count stays 0 and the test that wants it fails.

## done criteria

The Rust host runs the litmus threads on several vCPUs at once, the forbidden outcomes never appear, the store buffer
does, and the EnTSO plant is named; the misaligned XCHG's behaviour on two vCPUs is measured and recorded.

## self-review checklist

- [x] Scope names one verifiable property: x86-TSO on several vCPUs of the Rust host.
- [x] context_files cite the frozen judge, the hypervisor crate, the run loop and the lowering.
- [x] Emits: none.
- [x] Artifact contract gradable by command.
- [x] Observation contract: values and two plants.
- [x] One concept: the memory model on several vCPUs.

## result

- **Several vCPUs** (a892fabe6): `hypervisor::Vm` is `Sync`, as `hv_vm.h` puts no thread on a VM call and
  `hv_vcpu_create` "Creates a vCPU instance for the current thread", one a thread (`hv_vcpu.h` 20-25, read in the SDK);
  `GuestMemory` is `Send`, the mapping it owns moving with it. The Rust host had neither, so no test could make a
  second vCPU on another thread.
- **The judge:** `crates/w2m_vm/tests/litmus.rs`, the frozen judge's five tests and plant, the x86 assembled by clang,
  one vCPU thread a litmus thread, misses compiled at the host and model steps stepped there, both under one `Mutex`
  around the guest; `run_threads` takes whole thread programs and the data's start.
- **The figures,** three runs at 100,000 iterations: MP, LB, 2+2W and IRIW's forbidden outcome 0 every run; SB's
  relaxed outcome 3,663, 9,901 and 6,154 (the frozen host's 1 to 36); EnTSO cleared, MP's forbidden outcome 870, 1,652
  and 549 (the frozen host's 4 to 9), each on its first run.
- **The misaligned XCHG:** two threads exchanging their tokens with a word four bytes off its alignment, 200,002 model
  steps, keep the values `[1, 2, 100]`: the model steps under the harness's one lock. Deferred (Sprint 272) annotated.
- **Census row G1** met (f4e7affe2).
- **Not as the card said:** plant 2, the threads run one after another, would hang at the arrival barrier, not fail;
  it became the misaligned XCHG moved to an aligned word, which its step count names. Plant 1 became EnTSO cleared on
  every run, the five-test judge's own runs included, a fault against that judge rather than its positive control.
- **The gates:** `scripts/gates.sh` exit 0 in 763 s on f4e7affe2, copied from `store/gates/passes.tsv`, `rdeps(hypervisor) | rdeps(signal_report) | rdeps(w2m_vm)`, 189 tests, its base Sprint 273's close 44ea6c9e4; the timing stage 688 ns an exit at load 3.02.
- **Plant 1,** restored by `cp` and checked by `cmp`: EnTSO cleared on every run; the test failed, "MP's forbidden
  outcome appeared, left: 1245".
- **Plant 2,** restored by `cp` and checked by `cmp`: the XCHG's word aligned; the test failed, "the exchanges did not
  all reach the model, left: 0 right: 200002".
