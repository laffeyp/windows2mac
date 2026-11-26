# SIGNAL_REPORT — Sprint 335 — The test build keeps no object files in its deps directory

Made by `signal_report 335` from 0 records under store/runs/ whose baseline names one of the sprint's 1 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 335.

## 1. Observed

### scope_confirmation

> Sprint 334 placed the gates' slowdown (Deferred (Sprint 334), "The gates' time placed and not yet changed"). Its
> findings:
> - `target/debug/deps` holds 859,305 entries, and 854,915 of them are `.rcgu.o` object files. The dev and test
>   profiles keep these as debug information under Cargo's documented macOS default, `split-debuginfo = "unpacked"`.
> - One relink of three crates added 2,111 entries.
> - `codesign -d` on a test binary takes 7.65-7.85 s in that directory and 0.00 s on a copy alone, reading the
>   directory in 3,798 of 3,904 samples.
> - A fresh copy's first launch takes 24.34 s there and 0.12 s alone.
> - nextest spends 470 s of a 1,024 s run before its first test, and 708 s of a 1,159 s run that relinks three crates.

### work_performed

- 98917f666 Sprint 335 card: the test build keeps no object files; Sprint 334's three Decisions

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a build profile setting, no runtime behavior.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 335: its Rubber Duck Pass and close.
