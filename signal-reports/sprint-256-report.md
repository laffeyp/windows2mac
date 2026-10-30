# SIGNAL_REPORT — Sprint 256 — The cache installed into the code area

Made by `signal_report 256` from 0 records under store/runs/ whose baseline names one of the sprint's 3 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 256.

## 1. Observed

### scope_confirmation

> The Decision "The translation cache's shape" as annotated at Sprint 255: card 4 of
> `docs/design-translation-cache-2026-10-30.md` cannot be built as designed. The C runtime maps a host address outside
> `W2M_CODE_START..W2M_CODE_END` to x86 address 0 at an exception (`runtime.c` 245) and at an in-VM slice switch
> (`threads.c` 258); the cache's 7,828,283 marks fit no table it reads; the host's give-backs read the 64 MiB code area
> alone. This card revises the design (KIT_DIARY #108: every reader of a code address listed first) and measures what
> the revision rests on.

### work_performed

- 1f388ea8e Sprint 256: the relocation's rule held by clang's words
- ab0f395f8 Sprint 256: the cache installed into the code area, its relocation measured
- cd1be9d9d Sprint 256 card: the cache installed into the code area

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a design and its measured relocation.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 256: its Rubber Duck Pass and close.
