# SIGNAL_REPORT — Sprint 300 — One test binary a crate

Made by `signal_report 300` from 0 records under store/runs/ whose baseline names one of the sprint's 2 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 300.

## 1. Observed

### scope_confirmation

> Each file under a crate's `tests/` is its own binary: `w2m_vm` has 26, `hypervisor` 5, and the gates' nextest run
> counted 40 binaries over the crates it selected (Sprint 298). On this Mac, macOS's `syspolicyd` assesses each newly
> linked executable on its first run: at Sprint 298's gates it ran at 210% CPU while three listings waited 1 min 28 s
> each, and a run of one 1.1 s test after a manifest change passed 600 s (298's Observation 4). nextest runs every
> binary twice to list it before any test. None of the 31 files has `fn main`, an `[[test]]` entry, `harness = false`
> or `env::set_var`; nextest runs each test in its own process, so a test that needs one VM a process keeps it.

### work_performed

- 84fd8c7d3 Sprint 300: one test binary a crate
- 68865f2bb Sprint 300 card: one test binary a crate

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None; the tests' layout.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 300: its Rubber Duck Pass and close.
