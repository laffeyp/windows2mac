# Sprint 082 — the successor scan in C: the flags a block's successors read

```yaml
---
id: 082
status: closed
phase: 5
pass_kind: architecture
---
```

## scope

The first link of the chain that puts the compiler inside the VM (recentering step 6; Decision 2026-10-15, the
inferred compiler kept and put in the VM). A block's code depends on the flags read after it (`live_out`), which
`VM._flags_live_out` finds by scanning the block's direct successors (`_flags_live_in`: at most LIVENESS_SCAN, 8,
instructions each, on pages already behind translated code, following direct jumps). `vmc_block` takes `live_out` as an
input; the VM has no way to compute it. This sprint writes it in C:

- **Two form facts.** The first branch op's kind (none, `always`, another) and the position among the decoded values
  of its target when the target is a constant (`constant[br[2]]`, as both Python functions read it), else none.
  `gen_stencil_table.facts_of` derives them from each recorded instruction and its decoded values; `record_facts`
  already refuses a form whose instructions disagree.
- **`vmc_live_in` and `vmc_live_out`** in `tools/vm_compiler/vm_compiler.c`, the two Python functions line for line,
  over a memory interface the caller gives: the bytes at an address when its page is behind translated code (the host
  test: a map; the VM, next sprint: the code-page set), and a hook told each page the scan reads, so the block joins
  that page's owners. An instruction with no form ends the scan with every undecided flag live: a superset of what
  the Python returns, so the code is correct and its words may differ; the judge counts these apart.
- **Judges.** (1) The record's memory: every recorded instruction's bytes placed at its address, the pages they cover
  as the code pages; every recorded block end scanned by the C and by `VM._flags_live_out`. (2) From outside the
  table's sources (KIT_DIARY #59): sequences of spec-row random encodings (the review's E1 generator) joined by
  direct jumps and Jcc, scanned by both. Equal where the C found every form; a superset where it did not; never a
  subset. Both in CI; a planted fault (the scan's NZCV kind taken from the wrong instruction) fails them.

## prerequisites

- 081 (closed, on CI)

## context_files

- `src/w2m/vm_backend.py` (`_flags_live_out`, `_flags_live_in`, `insn_flags`, `nzcv_setter`, `LIVENESS_SCAN`)
- `scripts/gen_stencil_table.py`, `tools/vm_compiler/vm_compiler.{c,h}`, `tests/test_vm_compiler_c.py`
- `scripts/stencil_judge.py`, `contracts/w2m.vm_compiler.stencil_table/0.80.0.json`

## signal contract

### Emits

- None new. The judge record (`scripts/stencil_judge.py`) gains a fifth judge, the scan, under the same tags.

### Invariants

- Vocabulary unchanged. `compile_insn` and the host's scan unchanged (no family or class identity changes).

## artifact contract

- `scripts/gen_stencil_table.py`, `tools/vm_compiler/vm_compiler.{c,h}`, `tools/vm_compiler/stencil_table.h`
  (regenerated), `tests/test_vm_compiler_c.py`, `scripts/stencil_judge.py`
- `scripts/gates.sh` exit 0; `gen_stencil_table.py --check` exit 0; CI `success` on the closing commit

## observation contract

- The two judges' counts: scans equal, supersets (and the instruction with no form that ended each), subsets (0).
- The plant fails both judges.

## done criteria

The C scan equals the Python's wherever every instruction it reads has a form, and is never short of it; both judges
in CI.

## observed
- **Facts.** 17,328 forms; 2,514 have a branch, 2,361 a constant target, every one held by a decoded value
  (`constant targets no value holds`: 0); no recorded instruction's facts disagree with its form's.
- **Judges** (`store/runs/stencil-judge-1792108383330932000` on d7a81658, 382 s, CANDIDATE_ACCEPTED): scan over the
  record's memory 1,386,082 equal, 112 wider than the host's, 65 equal with a form missing, 11 with no form for the
  last instruction, 0 short, 0 differing; scan-spec 174,796 equal, 29,975 wider, 67,591 equal with a form missing,
  18,383 with no form for the last instruction, 0 short, 0 differing. In CI: 4,000 record samples
  (`tests/data/scan_samples.txt.gz`, written by `gen_stencil_table.py`, covered by `--check`) and 400 spec layouts.
- **Plant.** The one this card named first (the NZCV kind taken from the instruction itself) changed no scan: both
  judges passed it. A Jcc's flags differ by kind only when it branches on its setter's NZCV, and every setter writes
  the flags its Jcc tests, so they are decided before the scan reaches the Jcc. The plant in CI steps past a jump
  instead of following it: record 621 differing and 2 short of 4,000, spec 129 differing and 74 short. KIT_DIARY #60.
- **Contract** `w2m.vm_compiler.stencil_table` 0.82.0: the scan's relation (equal where every instruction read has a
  form, else wider) and the two judges; 0.80.0 unchanged.
- **Gates:** `scripts/gates.sh` exit 0, 2,442 passed, 27,919 records intact. No family or class identity changed
  (vm_backend.py untouched).
