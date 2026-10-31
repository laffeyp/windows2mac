# SIGNAL_REPORT — Sprint 272 — A memory XCHG made atomic

Made by `signal_report 272` from 0 records under store/runs/ whose baseline names one of the sprint's 4 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 272.

## 1. Observed

### scope_confirmation

> Census row D1 (`docs/step-6-census-2026-10-30.md`), the first of the Decision "Sequence for step 6": x86 locks an XCHG
> with a memory operand without a LOCK prefix (Intel SDM vol. 2, XCHG, as https://www.felixcloutier.com/x86/xchg renders it: "If
> a memory operand is referenced, the processor's locking protocol is automatically implemented for the duration of the
> exchange operation, regardless of the presence or absence of the LOCK prefix or of the value of the IOPL"), and both of w2m's compilers lower it as a load
> and a plain store, so a second vCPU can write between them. This card makes it atomic in the one lowering that already
> knows how: the Rust translator's exclusive pair for a `_LOCK` iclass (`crates/translator/src/lower.rs` 1289-1450); the
> C compiler inside the VM declines the form, so its blocks go to the host.

### work_performed

- 2b27787dc Sprint 272: the exit ledger aligned on the host's extra compiles
- abb33727c Sprint 272: runtime addresses named by symbol in the batch run's stream; the held stop moves by the XCHG's declines
- 05ae6da85 Sprint 272: a memory XCHG made atomic
- d7ef0a957 Sprint 272 card: a memory XCHG made atomic

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a lowering and a decline, held by tests and the whole-block judge.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 272: its Rubber Duck Pass and close.
