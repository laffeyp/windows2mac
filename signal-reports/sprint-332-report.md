# SIGNAL_REPORT — Sprint 332 — The test runner checks a binary's signature once

Made by `signal_report 332` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 332.

## 1. Observed

### scope_confirmation

> Every process the gates start for a test binary, its two listings and one process a test, goes through
> `scripts/sign_and_run.sh`, the cargo runner (`.cargo/config.toml`, Sprint 132), which runs
> `codesign -d --entitlements -` on the binary before it `exec`s it, to sign it with the Hypervisor entitlement if it
> lacks one. Measured on 2026-11-24 during Sprint 331's second gates run: that check took 26.27 s on an already-signed,
> already-run binary (`target/debug/deps/signal_report-e2b7d80e5e2d45f2`), and launching the binary took 0.00 s; Sprint
> 331's two gates runs spent about 45 and 40 minutes before their first test (the Decision "The test runner's
> entitlement check first", 2026-11-24). This card makes the runner check a binary once: after it signs a binary, or
> finds it signed, it writes a stamp beside it, and a later launch whose binary is not newer than its stamp skips
> `codesign -d` and `exec`s at once. A relinked binary, newer than its stamp, is checked and signed as today.

### work_performed

- ae5f2494e Sprint 332: the test runner checks a binary's signature once
- 574d35693 Sprint 332 card: the test runner checks a binary's signature once

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; a build script.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 332: its Rubber Duck Pass and close.
