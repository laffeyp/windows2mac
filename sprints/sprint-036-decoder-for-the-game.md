# Sprint 036 — the decoder for Hail to the Rainbow: validity rules and the game's rows

```yaml
---
id: 036
status: closed
phase: 3
pass_kind: functional
---
```

## scope

Decoder exit criterion (d), re-pointed at Hail to the Rainbow (2026-10-09): every spec row the game's reachable code uses is XED-confirmed. Sprint 033 measured 579 rows used, 141 unconfirmed, and 8,962 positions in 125 kinds with no row (`store/game-runs/37982448071/gamediff2.json`). This sprint closes both:

1. Two validity rules the generated differential shows missing (Sprint 033 observed): LEA with a register operand (mod = 3; SDM Vol 2A, LEA: "#UD If source operand is not a memory location") gets `form: mem`; on an SSE-family opcode the mandatory prefix is the present prefix of highest precedence (the last F2/F3, then 66), and bytes with no row for it are invalid — `_pick` no longer falls back to the 66 row or the no-prefix row.
2. Rows for every gap kind: SSE and SSE2 packed and scalar single/double operations, MMX, VEX (AVX, AVX2, F16C-free) and FMA forms, x87 escapes (D8-DF), and the system and legacy one-byte and 0F opcodes the traversal reaches. Each row cites its SDM Appendix A table, as every existing row does.
3. Then the remaining unconfirmed rows, cause by cause, from `scripts/generated_differential.py --confirmations`.
4. `scripts/game_code_differential.py` over the native files and the run 37982448071 capture, recorded.

## prerequisites

- 033 (measurement), 035

## context_files

- Intel SDM Vol 2D Appendix A (opcode maps, Tables A-2 to A-6, A-7 to A-22 for x87 escapes); Vol 2A section 2.3.4 (VEX), 3.x per-instruction pages for #UD conditions
- `specs/x86_decode.json` 0.6.3, `scripts/gen_decoder.py`, `src/w2m/x86dec.py`, `scripts/generated_differential.py`, `scripts/game_code_differential.py`

## signal contract

### Emits

- harness plane: the game differential's record (COMPARISON_COMPLETED, and DISCREPANCY_REPORTED/ROUTED per remaining class) and the generated differential's record (`--record`).

### Invariants

- No vocabulary change. Every new row cites an SDM table.

## artifact contract

### Files modified

- `specs/x86_decode.json` (0.7.0), `src/w2m/x86dec_tables.py` (generated), `src/w2m/x86dec.py`, `specs/x86_decode_confirmations.json` (derived), tests
- created during the sprint: `scripts/rows_from_sdm.py`, `scripts/merge_sdm_rows.py`, `scripts/rebuild_decode_spec.py`, `specs/x86_decode_sdm_request.txt`

### Command exit codes

- `scripts/gates.sh` → 0

## observation contract

### Expected runtime signals

- Generated differential: `length_differs` 0, `ours_accepts` falls from its 0.6.3 count, confirmations rise from 549 of 809.
- Game differential: 0 length or iclass differences; positions without a row 0; criterion (d): every used row confirmed, or each unconfirmed row named with its cause.
- Corpus and probe differentials (criteria (a)-(c)) stay as they were.

## done criteria

Every position of the game's native and captured code decodes to a row, and every row the game uses is XED-confirmed, or the card names each exception with its cause.

## self-review checklist

- [x] Scope concrete. [x] Sources named. [x] No new vocabulary. [x] Observation declared before running. [x] File count recorded (one spec, its generated table, the decoder, derived confirmations, tests).

## observed
- Baseline (spec 0.6.3, 41,616 generated cases): ours_accepts 4,490; iclass_differs 209; ours_gap 73; length_differs 0; 549 of 809 rows confirmed. Game: 141 of 579 used rows unconfirmed; 8,962 positions without a row in 125 kinds.
- Two validity rules first. LEA `form: mem`. The SSE-family mandatory prefix became strict (highest-precedence prefix must have a row); that refused valid bytes on general-purpose opcodes that carry F2/F3 rows, so the spec gained `prefixes.legacy_prefix_opcodes`, per opcode, with the prefixes that keep a legacy meaning there (MOVBE: 66; BSF/BSR: 66 and F2; 0F C7: 66, F2, and F3 where no F3 row fits the ModRM — each from an XED decode of the generated case). The old fallback had been hiding missing rows: MOVDDUP, MOVSLDUP, MOVSHDUP, MOVQ2DQ and the Key Locker instructions decoded as their 66/no-prefix neighbours.
- Rows from the SDM, not from XED: `scripts/rows_from_sdm.py` parses the Opcode/Instruction column of the SDM Vol 2 instruction pages (felixcloutier.com transcription, retrieved 2026-10-09) into spec rows for a requested list of XED iclasses (`specs/x86_decode_sdm_request.txt`, 196 iclasses: the game's gap kinds, the instructions of its unconfirmed rows, and prefix neighbours); XED names which lines are wanted, the SDM line supplies the fields, and XED then judges them. The parser learned, one failure at a time: split Opcode|Instruction columns; footnote digits and `<sup>` footnotes (and a stripper that ate the 5 of `/05`); a space inside a VEX field (`VEX.LZ. 0F38.W1`); W0 written `.0`; pages named for a family (VBROADCAST, PMOVSX, UD); empty Instruction cells (FCOS); `NFx`; lines invalid in 64-bit mode (CALL ptr16:16) dropped, `Invalid*` (SAHF, a CPUID feature) kept; register-only and memory-only forms from the operand list (an `mm` operand had read as memory); VEX.vvvv = 1111 where an instruction has no NDS operand; W=0 on a partner whose sibling needs W=1 (MOVD/MOVQ). The SDM's `NP` became a row field checked per row (an opcode-level NP refused SGDT with 66).
- `scripts/merge_sdm_rows.py --replace` lets SDM rows replace hand-written vector rows of the same iclass (75 rows; they lacked L, W, vvvv, form or NP constraints — RORX/ANDN/SHRX/SHLX/MULX LZ, VPERMD W0, two-operand VEX vvvv, MMX shift-immediate register-only, MOVBE memory-only, SHA and fence NP) and keeps the general-purpose rows of 153 iclasses (operand-size immediates `z`, sizes and templates that one SDM line per size does not carry: a first version that replaced MOV broke C7 /0). Matching by mnemonic first dropped CALL_FAR and RET_FAR (they share `call`/`ret`); matched by iclass, then a merge that reports same-bytes-different-iclass collisions caught the generator taking near CALL/RET and string CMPSD lines for CALL_FAR, RET_FAR and CMPSD_XMM (line filters added) and IRET shadowing IRETD at CF (IRET removed from the request; IRETD carries the 16-bit iclass).
- Rows not from SDM text, each labelled in its `src`: FEMMS, VFMADDPS (FMA4), MOVNTSS, MOVNTSD (AMD) and the Key Locker rows (Intel's separate specification), from their opcodes, confirmed by XED; UD0 without ModR/M because XED decodes it so (the SDM page's footnote on older processors). Hand edits: PUSHFQ/POPFQ/IRETD 16-bit iclasses; XGETBV, LFENCE, SFENCE, MFENCE NP; RDRAND/RDSEED NFx; MOV_CR/MOV_DR iclasses and ModRM.reg validity (CR0/2/3/4/8, DR0-7, Sreg 0-5 with REX.R ignored); XOP rotate-immediate W0 and vvvv; JECXZ for E3 with 67; ENTER's `iw ib`.
- Decoder defects found on the way: XOP never set vvvv, so any XOP row requiring 1111 refused everything; the vvvv check ran for VEX only.
- `scripts/rebuild_decode_spec.py` rebuilds 0.7.0 from 0.6.3 (git 366e255), the request list and the SDM pages; its output equals the spec built step by step (0 rows differ).
- Spec 0.7.0, generated differential (55,030 cases; record `store/runs/gendiff-1791583020404460000`): agree 39,305; both_reject 13,060; ours_accepts 2,537; ours_gap 0; length_differs 0; iclass_differs 147 (rows the game does not use: PREFETCHIT0/1, PREFETCHRST2, POPF/PUSHF size forms on other opcodes, and similar); 971 of 1,061 rows confirmed (was 549 of 809). The remaining ours_accepts are mostly EVEX validity, not a gate (Decision 2026-10-08).
- Criteria (a)-(c) unchanged: Python 3.14.8 corpus 2,391,551 positions (Sprint 026 removed 120 int3 positions after non-returning calls), probes 53,406 positions; spec gaps 0; length and iclass differ from XED at 0 positions.
- **Criterion (d) met.** Game differential, spec 0.7.0 (record `store/runs/gamediff-1791583316617553000`; summary `store/game-runs/37982448071/gamediff-spec-0.7.0.json`): 6,192,478 positions (native 5,801,517; Mono-generated 390,961), agree 6,192,476, length or iclass differences 0; rows used 718, confirmed 718. Exception named: 2 positions `d5 48 81 c2 …` that XED decodes as APX REX2 ADD under chip ALL and FUTURE and refuses under SAPPHIRE_RAPIDS and ALDER_LAKE; no CPU this game runs on has APX, where `d5` is #UD in 64-bit mode, so w2m's refusal matches the hardware.
- Tests updated: `f3 0f 12` is MOVSLDUP (the old "invalid" case was a missing row); new invalid cases (F2 over 66 on 0F 6F, 66 0F 52, 66 0F 01 D0, LEA mod 3, F3 on MOVBE) and valid ones (MOVSLDUP, 66 MOVBE, F2 BSF, F3 0F C7 /1, 66 CF, 67 E3). Gates: 231 passed.

