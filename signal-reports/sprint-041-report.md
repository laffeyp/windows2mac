# SIGNAL_REPORT — Sprint 041

## 1. Observed

### scope_confirmation

> The instruction class is now the unit of work. Each of 140 iclasses has its own rule module, and 139 of them are accepted on their own substrate record, judged by an Intel Xeon Platinum 8573C in both the model and the VM. CPUID is judged against its profile table instead, since its whole point is to differ from the runner's CPU. The VM executes SSE and the new block, which it could not do before, and agrees with the CPU on every case of both case sets, faults included.

### work_performed

- **Vector state in the VM.** The monitor gets and sets Q0-Q15, FPCR and FPSR. MXCSR lives in the monitor data page, and FPCR follows it: AH=1, FIZ from DAZ, FZ from FTZ, RMode from RC. FPSR's cumulative flags are read as MXCSR's. CPACR_EL1.FPEN is set. Without it, the first SIMD instruction in the exception entry trapped into the same vector forever.
- **SSE lowering for 52 classes.** FCVTZS needs a fix-up to the integer indefinite. The flags of an integer conversion are rebuilt from FPSR as it stood before the instruction, so only IE or PE is reported, as x86 does. When MXCSR unmasks an exception, the instruction runs in x86fp on the host.
- **The rules package (`src/w2m/rules/`).** 92 modules cover 140 iclasses, and `lift.py` dispatches by iclass. A rule's identity is the sha256 of its module, the shared modules it uses, and `lift.py`.
- **Item runs (`scripts/class_run.py`).** Each runs the chain SESSION_INIT → CANDIDATE_GENERATED → COMPARISON_COMPLETED for the model and for the VM → CANDIDATE_ACCEPTED, or DISCREPANCY_REPORTED and ROUTED, → SESSION_COMPLETED. A pool of 8 processes runs them; `store/runs/class-runs.json` indexes the records.
- **The next block.** IMUL, CMOVcc ×16, SHL, SAR, ROL, ROR, XADD, CMPXCHG and nine LOCK forms, the last lowered as LDAXR/STLXR retry loops. Case set 041 (seed 2026_10_10_41, 13,243 cases, 32 iclasses) runs on the runner from the 039 harness, through `probe.json` `source`. Its results are in the release `semantics-041`.

### signal_trace

```
store/runs/vmoracle-1791611605776011000, -1791611631948935000 (set 039, Xeon 8573C and 8370C): COMPARISON_COMPLETED conforms, 39,708 cases, 5,485 faults, XMM and MXCSR compared
store/runs/vmoracle-1791613138236701000 (set 041, Xeon 8573C): COMPARISON_COMPLETED conforms, 13,243 cases, 2,195 faults
store/runs/class-*-17916129* (139 runs): SESSION_INIT, CANDIDATE_GENERATED, COMPARISON_COMPLETED x2, CANDIDATE_ACCEPTED, SESSION_COMPLETED
```

## 2. Expected

> The card expected the VM to agree with the CPU on SSE, three planted faults to be caught, behavior to be unchanged after the split, causal item runs with a planted wrong rule reported, and the next block in. Every expectation held. Three defects were found and fixed on the way, below.

## 3. Delta

### dual_contract_self_grade

**Signal contract:** pass. **Artifact contract:** `scripts/gates.sh` 0 (280 tests; 289 records replay). **Observation contract:**

- VM SSE: all 39,708 cases of set 039 agree on both Intel CPUs, including XMM, MXCSR and 5,485 faults (before this sprint: 22,364 compiled, no SSE). The slow-path count equals the number of unmasked-MXCSR cases. Pass.
- Planted faults, reach counted first: no indefinite fix-up (37 cases, caught as a GPR difference); FPCR.AH=0 (117 cases, caught as an XMM difference); IDC not mapped to DE (105 cases, caught as an MXCSR difference); a wrong ADD rule (reported and routed on its record). Pass.
- Split: 0 differences on both executors after the move. Pass.
- Item runs: 139 accepted; `trace_ancestry` from CANDIDATE_ACCEPTED reaches the rule's CANDIDATE_GENERATED and the run's start. Pass.
- Next block: the model and the VM agree with the Intel 8573C on all 13,243 cases; AMD EPYC 7763 and 9V74 agree too. Pass.

**overall:** pass.

## 4. Hypothesis

### rubber_duck_observations

**Sequence narration:** the VM ran each case of both case sets and compared it with Intel's results. Each class's item run caused its two comparisons and then its decision. The runner ran case set 041 on four CPUs, and both executors agreed with all of them.

- *Observation 1 (the CPU corrected the SDM's wording):* CMPXCHG with a 32-bit register destination does not write that register when the comparison fails; Intel kept its upper half in 12 cases. The SDM's "always written" holds only for memory. Disposition: resolved-here (`cstz`); the rule cites the run.
- *Observation 2 (a helper's scratch register clobbered state):* `_mark_defined` uses x16, and the shift flag code then tested x16 as if it still held the count, defining OF for every count. The CPU caught this in 625 cases on three classes. Disposition: resolved-here; KIT_DIARY #24.
- *Observation 3 (a rule defined what the SDM leaves undefined):* SHR's CF for count ≥ size had been defined since Sprint 029. Intel happens to give 0, so no judge could object. Disposition: resolved-here from the SDM's flag text; the remaining per-class undefined-flag check is an open item for each class's next revision (KIT_DIARY #21).
- *Observation 4 (what an item run does not yet do):* the "draft" step is the hand-written module; no model writes a rule yet. Disposition: recorded. Sprint 042 tests the generation bet, and LOCK atomicity under more than one vCPU is judged in Sprint 047, where threads run.

### status_and_blockers

`status: complete`
