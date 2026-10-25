# SIGNAL_REPORT — Sprint 135 — The A64 encoder in Rust

Made by `signal_report 135` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 135.

## 1. Observed

### scope_confirmation

> The walking skeleton's fourth link, and the first piece of the translator (S3) the plan's step 4 needs. A crate,
> `a64_encoder`, carries `src/w2m/a64_encoder.py` function for function: every AArch64 word w2m emits, the integer,
> memory, branch, SIMD and floating-point forms, the system registers, and the fixed words (ERET, the barriers, the TLB
> and cache maintenance, the runtime lock's CASA and STLR). Its judge is the Python's: each case of
> `tests/test_a64_encoder.py` (248) assembled by clang (`--target=aarch64-linux-gnu`) and read back with llvm-objdump
> must give the word the function gives, plus ten words the monitor code uses that the Python test never checked
> (`ic ialluis`, `tlbi vmalle1is`, the CASA and STLR, `adr`, the register and unscaled loads and stores).

### work_performed

- b431194a9 Sprint 135: the A64 encoder in Rust

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> None. An encoder; it puts nothing on a record.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 135: its Rubber Duck Pass and close.
