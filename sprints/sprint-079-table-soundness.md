# Sprint 079 — the table's soundness: guards, canonical flag arguments, a held-out judge

```yaml
---
id: 079
status: closed
phase: 5
pass_kind: functional
---
```

## scope

A red-team pass over Sprints 076 to 078, at the user's request. Every judge of the stencil table so far drew from the
table's own sources: instances from the same generator (Sprint 076's first judge, KIT_DIARY #58), then the record the
table was derived from (Sprints 077 and 078). Those judges show that the C transcribes the table and that the table
rebuilds the record. They do not show that the table holds for an instruction it never saw.

- **A. Canonical flag arguments.** `compile_insn` reads `live_after` and `nzcv_out` only through a flags op, and
  `nzcv_from` only through a Jcc whose condition its kind's NZCV_CONDITIONS holds (vm_backend.py, `compile_insn`'s
  flag masks, dead ops and Jcc arm). `stencil_census.canonical_flags` writes an argument the instruction cannot read
  as its default, in the key; the C does the same from each form's facts. Keys that differ only there become one, and
  their instances pool.
- **B. Guards.** A value the samples of a stencil never varied, which some expression of maps onto a constant
  argument, is guarded: an instruction whose value there differs has no stencil from the table and goes to the host.
- **C. A held-out judge.** The class case sets (250,840 Intel-checked instructions with random operands, none from
  the game record), each compiled at two flag contexts by the C and by `compile_insn`.
- **D. Provenance.** The record and the table pickle kept in `store/stencil-inputs/` (ignored by git), their sha256
  here.

## prerequisites

- 078 (closed, on CI)

## context_files

- `scripts/stencil_table.py`, `scripts/stencil_census.py`, `scripts/gen_stencil_table.py`,
  `tools/vm_compiler/vm_compiler.c`, `src/w2m/vm_backend.py` (`compile_insn`)

## signal contract

### Emits

- None new: scripts, generated data, C and tests; the C is not yet run in the VM.

### Invariants

- Vocabulary unchanged. The Python compiler unchanged.

## artifact contract

- `canonical_flags` in `scripts/stencil_census.py`; guards in `scripts/stencil_table.py`, the generator and the C;
  `test_the_case_sets_compile_as_compile_insn` in `tests/test_vm_compiler_c.py`
- `uv run pytest` exit 0; CI `success` on the closing commit

## observation contract

- **A:** keys before and after; keys whose instances split after (0 required: a split would show a dropped argument
  the words read).
- **B:** stencils with guards; record and fresh instances rebuilt, guarded, differing (0 required).
- **C:** the held-out compiles on the old table and the new: equal, differing (0 required on the new), no stencil,
  guarded.

## done criteria

A to D; C with 0 differing on the new table; CI passes.

## observed
- **The defect.** Measuring how many flag variants collapse (A) turned up 7 groups whose stencils differed where the
  flags could not matter. The cause is in Sprint 076's table: PSRLDQ, PSRLD, PSLLD and PSRLW's refined branches (a
  count at or past the lane width) zero a fixed register (`movi_zero(13)`, `movi_zero(5)`), CALL_NEAR builds its
  return address as a constant, a JNZ its target. Each group's instances never varied that value, so the derivation
  took it for a constant; each judge drew from the same instances or the same record and agreed. Sprints 077 and 078
  carried the defect into the C.
- **C, on Sprint 078's committed table:** 22,665 equal, **144 differing** (PSRLDQ 48, PSRLW 32, PSRLD 16, PSLLD 16,
  VPSRLD 16, PSLLQ 8, VPSRLQ 8; e.g. `66440f73dc34`, PSRLDQ xmm4, 0x34, zeroes v0 where compile_insn zeroes v4),
  478,871 with no stencil.
- **A.** 38,154 keys become 21,182; 0 keys split by their instances after (the dropped arguments do not reach the
  words). The forms' facts gain `has_flags`; 17,328 forms, 0 disagreeing.
- **B.** 3,365 of 21,244 stencils guarded, 4,957 guards. `scripts/stencil_table.py` (70 s): 567,047 fresh instances
  judged, 567,040 equal, 7 guarded, 0 differing; the record's 1,186,511 calls rebuilt, 1,186,509 equal, 2 guarded,
  0 differing.
- **C, on the new table:** 31,311 equal, **0 differing**, 224 guarded, 470,145 with no stencil (94% of the 501,680
  compiles: the case sets sweep forms and flag contexts the game's record never reached). 245 iclasses compiled. In CI
  as `test_the_case_sets_compile_as_compile_insn` (6 s).
- **The record judges on the new table:** 1,186,511 calls, C key and words equal in 1,186,509, 2 guarded, 0 differing.
  1,177,340 blocks: 1,171,852 equal, 0 differing; 5,488 to the host and 173 cut short (17,842 on Sprint 078's table),
  of which 7 by a guard.
- **D.** `store/stencil-inputs/`: `insn-bytes-100g.pickle` sha256:02eb7d3b08b793579f59834c7ff85cf197420b933c7deddcbb1917068b8d8193,
  `stencil-table-079.pickle` sha256:8694eb636439ae78d847ecd1fd7ba388d08b6b62f6f2fd3046d2d7910848f107,
  `record_insn_bytes.py` (the scratch driver that wrote the record) sha256:669338b4812e8b91c8d4d0cc23bcf4773eb46aee741fada39807017b328729dc.
- **Also checked:** `vmc_block` keeps 64 decoded instructions on its stack, about 14 KB of the guest runtime's 64 KB
  (`RUNTIME_STACK_SIZE`). The thunk test chose its block too narrowly: with a thunk at the third instruction, the
  second had no stencil for its new flags and the C cut the block, correctly; it now runs a thunk through every
  sample block of three or more instructions (over 1,000 end at the thunk).
- **Open:** coverage. The table holds forms the record reached; 94% of the held-out compiles have none. Keys taken
  from the decoder's spec rows rather than the record (instances made per key as now) would widen it; the VM's own
  rate of host compiles decides whether that comes next.
- **Closed** on c95b650d (CI run 38803778005, success). `uv run pytest`: 2,435 passed.
