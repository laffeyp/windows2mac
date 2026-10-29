# SIGNAL_REPORT — Sprint 203 — The host's random bytes

Made by `signal_report 203` from 0 records under store/runs/ whose baseline names one of the sprint's 3 commits, or which those commits added. The self-grade and the Rubber Duck observations are the BLACKBOARD tail entry's for Sprint 203.

## 1. Observed

### scope_confirmation

> The Decision "Sequence after Sprint 198", the batch run's stops in row order: the stream equals the Python's rows
> 0-256 and stops at row 257, RANDOM 23 (bcrypt.c's BCryptGenRandom, com.c's CoCreateGuid). The Python answers it from
> `random.Random(0x5EED).randbytes(n)` when the run is on the instruction clock, so two runs of one tree make the same
> calls, and from `os.urandom` otherwise (vm_backend 2357, 3840-3842). Python 3.12's `randbytes(n)` is
> `getrandbits(n * 8).to_bytes(n, 'little')` (random.py 284-286); its generator is CPython's MT19937
> (Modules/_randommodule.c 85-89, 129-275, 282-345, 488-535, fetched 2026-10-28 from the 3.12 branch).

### work_performed

- 729272c7f Sprint 203: a judge for the seed's split into words
- c360ce1a5 Sprint 203: the host's random bytes
- 62be10625 Sprint 203 card: the host's random bytes

### signal_trace

```
(no record)
```

### signal_trace_tags


## 2. Expected

> - None: the judges are `batch_run.rs` and unit tests.

## 3. Delta

### emits_check

- the card's Emits names no tag, and the sprint made no record

## 4. Hypothesis

See the BLACKBOARD tail entry for Sprint 203: its Rubber Duck Pass and close.
