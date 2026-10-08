# Sprint 020 — generated-encoding differential

```yaml
---
id: 020
status: closed
phase: 3
pass_kind: observation
---
```

## scope

Author `src/w2m/encgen.py` (seeded generation: for each spec row, vary legacy prefixes, REX bits, ModRM mode/r/m incl. SIB base 5 and RIP-relative, ModRM.reg when free, VEX/XOP W/L/vvvv, EVEX W/L'L, immediate and displacement bytes) and `scripts/generated_differential.py` (XED chip ALL vs ours; outcome classes both_reject / agree / ours_accepts / ours_gap / length_differs / iclass_differs). Method: Paleari et al. ISSTA 2010; Woodruff et al. LangSec 2021.

## observation (seed 20261008, 48 cases per row, 38,592 cases)

- agree 68.78%; length_differs 0; ours_gap 0.
- ours_accepts 29.91%: the spec has no validity rules — LOCK on non-lockable or register-destination forms (BAD_LOCK_PREFIX), VEX/EVEX L, W, vvvv constraints (kmov*, vmovq, vpbroadcast*, vpermq).
- iclass_differs 1.31%: C0/C1/D0-D3 /6 is SHL in XED (ours SAL); F2 on movs/stos/ins is REP (REPNE only for cmps/scas); F3 + REX.B + 90 is PAUSE; 0F 18 and 0F 0D with mod=3 are NOP; EVEX 72 /4 W1 is VPSRAQ; pcmpistri with REX.W is PCMPISTRI64; LOCK XCHG stays XCHG.
- Worklist: `specs/x86_generated_worklist.json`.
