# Sprint 050 — mono's and UnityPlayer's remaining instruction classes

```yaml
---
id: 050
status: open
phase: 5
pass_kind: functional
---
```

## scope

Sprint 049 left the frontier at 5,376,528 instructions, on legacy CVTDQ2PS (0f 5b c0) in mono-2.0-bdwgc.dll's own code at 0x1180023e52. The censuses of 2026-10-11 (`store/census/class-census-mono-2026-10-11b.json`, `store/census/class-census-unity-2026-10-11.json`) already list every class the two DLLs reach that w2m does not lift; this sprint takes them as one case set instead of one class per frontier stop (user, 2026-10-10: plan from what is held).

Set 050, judged on the Intel Xeon 8573C, model and VM:

1. **Legacy SSE floating point:** CVTDQ2PS, CVTPS2DQ, CVTTPS2DQ, CVTPS2PD, CVTPD2PS, CMPPS, CMPPD, CMPSS, SHUFPD, MOVLPS, MOVHPD.
2. **Legacy SSE integer:** PSHUFB, PALIGNR, PSADBW, PACKSSWB, PACKSSDW, PMADDUBSW, PMADDWD, PEXTRW, PINSRB, PINSRD, PUNPCKHQDQ, PMULHW, PMULLD, PADDUSB, PADDB, PAVGB, PABSW, PCMPGTB, PCMPGTW, PCMPEQQ, PMAXUD, PMAXUW, PMAXSW, PMINSD, PMOVSXDQ, PMOVZXWD, PSLLW, MOVNTDQ.
3. **Integer:** SHRD, SHLD, RCR, STC, CLC, CMC, MOVBE, POPCNT, SAHF, PUSHFQ, POPFQ, LEAVE, LOOP, LOOPE, LOOPNE, JRCXZ, PAUSE; the single string forms LODSB, SCASB, SCASD, STOSB, MOVSB, MOVSD, CMPSB, CMPSD.

Not in this set, each with its reason: the MMX-register forms of MOVD, MOVQ, PADDW, PUNPCKLBW and the rest, EMMS and MOVDQ2Q, and the x87 classes (FLDCW, FNCLEX, FNSTSW, FWAIT, FISTP, FIADD, FADD, FIDIVR) share the x87 register file, one concept for a later sprint; AES, SHA, PCLMULQDQ, ADX, RDRAND, RDSEED, FMA4, XOP and 3DNow! sit behind CPUID features w2m does not report (Decision 2026-10-11); INT3, INT, INT1, UD2, IN, OUT, INS, OUTS, CLI, STI, HLT and far RET raise exceptions in user mode, which the exceptions sprint takes; XLAT and ENTER are one instance each in data the traversal decoded.

## prerequisites

- 049 (closed: CI run 38121563657 on d470850a; main 19b568c4)

## context_files

- `specs/x86_decode.json`, `scripts/gen_decoder.py`, `src/w2m/x86ops.py`, `src/w2m/lift.py`, `src/w2m/rules/_sse.py`, `src/w2m/rules/_int.py`, `src/w2m/runtime.py`, `src/w2m/x86fp.py`, `scripts/semantics_differential.py`, `scripts/vm_oracle.py`, `scripts/class_run.py`, `probes/semantics/semantics.c`; Intel SDM Vol 2 instruction pages for each class

## signal contract

### Emits

- Per class: the class run (`scripts/class_run.py`, one record per class in `store/runs/`). Per frontier run: EXECUTION_STARTED, then SERVICE_UNIMPLEMENTED or EXECUTION_COMPLETED.

### Invariants

- Vocabulary 0.7; no new tags.

## artifact contract

- modified: `specs/x86_decode.json` (operand templates, version 0.8.2), `src/w2m/rules/` (rules for the set's classes), `src/w2m/runtime.py` and `src/w2m/x86fp.py` (model semantics), `scripts/semantics_differential.py` (CLASSES_050), `scripts/vm_oracle.py` and `scripts/class_run.py` (set 050 registered), `.github/workflows/ci.yml` if the case file is not committed
- created: the Intel run's record in `store/probe-runs/<run>/`, release `semantics-050`
- gates 0 and CI `success` on the closing commit

## observation contract

- **The model and the VM equal the Xeon 8573C** on every case of set 050, three runs whose CPU results agree, faults included.
- **The censuses shrink:** mono's and UnityPlayer's rerun lists none of the set's classes as missing.
- **The frontier passes CVTDQ2PS at 0x1180023e52**, and its next stop is recorded.

## done criteria

Set 050 accepted class by class in `store/runs/class-runs.json`; the frontier runs past 0x1180023e52.

## observed
