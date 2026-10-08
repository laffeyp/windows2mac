# Sprint 021 — validity layer and edge rules from the generated differential

```yaml
---
id: 021
status: closed
phase: 3
pass_kind: functional
---
```

## scope

Work the Sprint 020 worklist: iclass edge rules (SHL for /6; REP for F2 on non-compare string ops; F3 REX.B 90 is PAUSE; 0F 18/0D register forms NOP; EVEX VPSRAQ; PCMPISTRI64; LOCK XCHG stays XCHG) and a validity layer raising `InvalidEncoding` (guest #UD, v0.2 §5.1): LOCK only on memory-destination forms of lockable rows; VEX vvvv=1111 where unused; VEX/XOP L from the SDM opcode column; register-only shift-immediate groups; strict mandatory prefix on SSE-family opcodes.

## repair-path labels (v0.2 §11.3)

- Spec 0.5.0-0.5.1 (`lock`, `vvvv`, `l`, `form`, iclass edits, 8 rows): contract revision.
- `InvalidEncoding`; LOCK/vvvv checks; strict-prefix rule; PAUSE, XCHG, REP fusion rules: implementation repair.

## command exit codes

- `uv run pytest` → 0 (108 passed); `ruff`, `mypy`, `gen_decoder --check` → 0

## observation (38,976 generated cases, chip ALL)

| outcome | Sprint 020 | Sprint 021 |
|---|---|---|
| agree | 68.78% | 72.61% |
| both reject | 0.00% | 15.27% |
| ours accepts, XED rejects | 29.91% | 11.38% |
| ours has no row, XED decodes | 0.00% | 0.19% |
| length differs | 0 | 0 |
| iclass differs | 1.31% | 0.55% |

Corpus (2,444,456 positions): unchanged — ours equals XED on length and iclass everywhere.

## residue (worklist `specs/x86_generated_worklist.json`)

EVEX broadcast/zeroing/opmask validity; W constraints on VEX/EVEX rows (vpbroadcast*); newer instructions in prefixed slots (PREFETCHIT0/1, PREFETCHRST2, CLFLUSHOPT, TPAUSE/UMONITOR, MOVDDUP/MOVSHDUP, AESENC128KL); 67-prefixed JECXZ; 66-prefixed PUSHF.
